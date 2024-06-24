use std::path::PathBuf;
use tokio::net::UnixListener;
use tokio_stream::StreamExt;
use serde_json::Value;
use tokio_serde::formats::SymmetricalJson;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};
use crate::config::{Config, Project};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use std::cell::RefCell;

pub struct Daemon {
    config_path: PathBuf,
    config: Config,
}

impl Daemon {
    pub fn new(config_path: PathBuf) -> Self {
        let config = Config::load(&config_path);
        Daemon { config_path, config }
    }

    pub async fn start(&mut self, socket_path: &str) {
        let listener = UnixListener::bind(socket_path).expect("Failed to bind socket");
        let sp2=RefCell::new(socket_path.to_string());
        ctrlc::set_handler(move || {
            println!("removing socket file");
            std::fs::remove_file(sp2.borrow().clone()).unwrap();
            std::process::exit(1);
        })
            .expect("Error setting Ctrl-C handler");
 
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {

                    let length_delimited = FramedRead::new(stream,LengthDelimitedCodec::new());
                    let mut deserializer = tokio_serde::SymmetricallyFramed::new(length_delimited,SymmetricalJson::<Value>::default());

                    tokio::spawn(async move {
                        while let Some(msg)=deserializer.try_next().await.unwrap() {
                            dbg!(msg);
                        }
                    });






                    // while let Ok(line)=consumer.read_line().await {
                    //     match line.as_str() {
                    //         // kill the daemon
                    //         "quit" => {
                    //             return;
                    //         }
                    //         // end this connection
                    //         "end" => {
                    //             break;
                    //         }
                    //         // any other command => handle it
                    //         _ => {
                    //             let response = self.handle_request(&line).await;
                    //             dbg!(&response);
                    //             let _ = writer.write_all(response.as_bytes()).await;
                    //         }
                    //     }
                    // }
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    }

    async fn handle_request(&mut self, request: &str) -> String {
        let parts: Vec<&str> = request.split_whitespace().collect();
        match parts[0] {
            "project_add" => {
                if parts.len() > 1 {
                    let project_name = parts[1].to_string();
                    self.config.projects.push(Project {
                        name: project_name,
                        plugins: Vec::new(),
                        properties: HashMap::new(),
                    });
                    self.config.save(&self.config_path);
                    format!("Project {} added", parts[1])
                } else {
                    "Invalid command".to_string()
                }
            }
            "project_list" => {
                let project_names: Vec<String> = self.config.projects.iter().map(|p| p.name.clone()).collect();
                project_names.join(", ")
            }
            _ => "Unknown command".to_string(),
        }
    }
}
