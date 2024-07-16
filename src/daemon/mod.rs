use crate::config::{Project, SavedConfig};
use crate::event::Event;
use crate::plugin::{Plugin, PluginFactory, TmuxPlugin};
use futures::sink::SinkExt;
use serde_json::{json, Value};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use strum::IntoEnumIterator;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tokio_serde::formats::SymmetricalJson;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

mod event_commands;
mod global_commands;
mod plugin_commands;
mod project_commands;

pub struct Daemon {
    config: Arc<Mutex<SavedConfig>>,
    plugin_manager: Arc<Mutex<PluginFactory>>,
    should_stop: Arc<Mutex<bool>>,
}

impl Daemon {
    pub fn new(config_path: PathBuf) -> Self {
        Daemon {
            config: Arc::new(Mutex::new(SavedConfig::new(config_path))),
            plugin_manager: Arc::new(Mutex::new(PluginFactory::new())),
            should_stop: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn start(&mut self, socket_path: &str, force: bool) {
        self.plugin_manager
            .lock()
            .await
            .register_plugin(Mutex::new(Box::new(TmuxPlugin {})))
            .await;

        if force && std::path::Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path).unwrap();
        }
        let listener = UnixListener::bind(socket_path).expect("Failed to bind socket");
        let sp2 = RefCell::new(socket_path.to_string());
        ctrlc::set_handler(move || {
            println!("removing socket file");
            let socket_path = sp2.borrow().clone();
            if force && std::path::Path::new(&socket_path).exists() {
                std::fs::remove_file(&socket_path).unwrap();
            }
            std::process::exit(1);
        })
        .expect("Error setting Ctrl-C handler");

        let cancellation_token = tokio_util::sync::CancellationToken::new();
        loop {
            // if *(self.should_stop.lock().await) {
            //     println!("--- stopping (1) ---");
            //     break;
            // }
            println!("--- waiting for connection or cancellation ---");

            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    println!("--- stopping (3) ---");
                    break;
                },
                connection = listener.accept() => {
                    println!("--- new connection ---");
                    match connection {
                        Ok((stream, _)) => {
                            self.handle_connection(stream,cancellation_token.clone()).await;
                        }
                        Err(e) => {
                            eprintln!("Error accepting connection: {}", e);
                        }

                    }
                }
            }
        }
        println!("--- stopping daemon ---");
        if force && std::path::Path::new(&socket_path).exists() {
            println!("--- removing socket file ---");
            std::fs::remove_file(&socket_path).unwrap();
        }
    }
    async fn handle_connection(&mut self,stream: tokio::net::UnixStream,cancellation_token: tokio_util::sync::CancellationToken) {
        println!("--- new connection ---");

        let (read_socket, write_socket) = split(stream);

        let length_delimited_read =
            FramedRead::new(read_socket, LengthDelimitedCodec::new());
        let mut deserializer = tokio_serde::SymmetricallyFramed::new(
            length_delimited_read,
            SymmetricalJson::<Value>::default(),
        );
        let length_delimited_write =
            FramedWrite::new(write_socket, LengthDelimitedCodec::new());
        let mut serializer = tokio_serde::SymmetricallyFramed::new(
            length_delimited_write,
            SymmetricalJson::<Value>::default(),
        );
        let config = Arc::clone(&self.config);
        let plugin_manager = Arc::clone(&self.plugin_manager);
        let should_stop = Arc::clone(&self.should_stop);

        tokio::spawn(async move {
            // we accept multiple messages on the same connection, a sort of scripting
            println!("--- new async ---");
            loop {
                println!("--- inside loop ---");
                if *(should_stop.lock().await) {
                    println!("--- stopping (2) ---");
                    cancellation_token.cancel();
                    break;
                }
                match deserializer.try_next().await.unwrap() {
                    Some(msg) => {
                        dbg!(msg.clone());
                        let subject = msg.get("subject").unwrap().as_str().unwrap();
                        let command = msg.get("command").unwrap().as_str().unwrap();
                        let params = msg.get("params").unwrap().as_object().unwrap();
                        dbg!((subject,command,&params));
                        let response = Daemon::handle_request(
                            &config,
                            &plugin_manager,
                            &should_stop,
                            subject,
                            command,
                            params,
                        )
                            .await;

                        dbg!(&response);
                        serializer.send(json!({"lines":response})).await.unwrap();
                    }
                    None => {
                        break;
                    }
                }
            }
            println!("--- exiting async ---");
        });
        
    }


    async fn handle_request(
        config: &Arc<Mutex<SavedConfig>>,
        plugin_manager: &Arc<Mutex<PluginFactory>>,
        should_stop: &Arc<Mutex<bool>>,
        subject: &str,
        command: &str,
        arguments: &serde_json::Map<String, Value>,
    ) -> String {
        match subject {
            "global" => Daemon::handle_global(config, plugin_manager, command, arguments).await,
            "project" => Daemon::handle_project(config, plugin_manager, command, arguments).await,
            "event" => Daemon::handle_event(config, plugin_manager, command, arguments).await,
            "plugin" => Daemon::handle_plugin(config, plugin_manager, command, arguments).await,
            "daemon" => {
                Daemon::handle_daemon(config, plugin_manager, should_stop, command, arguments).await
            }
            _ => "Unknown command".to_string(),
        }
    }

    pub async fn handle_daemon(
        config: &Arc<Mutex<SavedConfig>>,
        plugin_manager: &Arc<Mutex<PluginFactory>>,
        should_stop: &Arc<Mutex<bool>>,
        command: &str,
        arguments: &serde_json::Map<String, Value>,
    ) -> String {
        let plugin_manager = plugin_manager.lock().await;
        match command {
            "status" => {
                format!("up and running")
            }
            "stop" => {
                *should_stop.lock().await = true;
                format!("stopping")
            }
            _ => "Unknown command".to_string(),
        }
    }
}
