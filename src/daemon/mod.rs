use crate::config::{Project, SavedConfig};
use crate::event::Event;
use crate::plugin::{Plugin, PluginFactory, TmuxPlugin};
use futures::sink::SinkExt;
use serde_json::{json, Value};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::str::FromStr;
use strum::IntoEnumIterator;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tokio_serde::formats::SymmetricalJson;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

mod project_commands;
mod event_commands;

pub struct Daemon {
    config: Arc<Mutex<SavedConfig>>,
    plugin_manager: Arc<Mutex<PluginFactory>>,
}

impl Daemon {
    pub fn new(config_path: PathBuf) -> Self {
        let config = Arc::new(Mutex::new(SavedConfig::new(config_path)));
        let plugin_manager = Arc::new(Mutex::new(PluginFactory::new()));

        Daemon {
            config,
            plugin_manager,
        }
    }

    pub async fn start(&mut self, socket_path: &str) {
        self.plugin_manager
            .lock()
            .await
            .register_plugin(Mutex::new(Box::new(TmuxPlugin {})))
            .await;

        let listener = UnixListener::bind(socket_path).expect("Failed to bind socket");
        let sp2 = RefCell::new(socket_path.to_string());
        ctrlc::set_handler(move || {
            println!("removing socket file");
            std::fs::remove_file(sp2.borrow().clone()).unwrap();
            std::process::exit(1);
        })
        .expect("Error setting Ctrl-C handler");

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
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

                    tokio::spawn(async move {
                        // we accept multiple messages on the same connection, a sort of scripting
                        while let Some(msg) = deserializer.try_next().await.unwrap() {
                            dbg!(msg.clone());
                            let subject = msg.get("subject").unwrap().as_str().unwrap();
                            let command = msg.get("command").unwrap().as_str().unwrap();
                            let params = msg.get("params").unwrap().as_object().unwrap();
                            dbg!(&params);
                            let response = Daemon::handle_request(
                                &config,
                                &plugin_manager,
                                subject,
                                command,
                                params,
                            )
                            .await;

                            dbg!(&response);
                            serializer.send(json!({"lines":response})).await.unwrap();
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    }

    async fn handle_request(
        config: &Arc<Mutex<SavedConfig>>,
        plugin_manager: &Arc<Mutex<PluginFactory>>,
        subject: &str,
        command: &str,
        arguments: &serde_json::Map<String, Value>,
    ) -> String {
        match subject {
            "project" => Daemon::handle_project(config, plugin_manager, command, arguments).await,
            "event" => Daemon::handle_event(config, plugin_manager, command, arguments).await,
            _ => "Unknown command".to_string(),
        }
    }


}
