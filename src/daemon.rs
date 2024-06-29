use crate::config::{SavedConfig, Project};
use futures::sink::SinkExt;
use serde_json::{Value,json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, split};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tokio_serde::formats::SymmetricalJson;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub struct Daemon {
    config: Arc<Mutex<SavedConfig>>,
}

impl Daemon {
    pub fn new(config_path: PathBuf) -> Self {
        let config = SavedConfig::new(config_path);
        Daemon { config: Arc::new(Mutex::new(config)) }
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
                    println!("--- new connection ---");

                    let (read_socket,write_socket)=split(stream);
                    
                    let length_delimited_read = FramedRead::new(read_socket,LengthDelimitedCodec::new());
                    let mut deserializer = tokio_serde::SymmetricallyFramed::new(length_delimited_read,SymmetricalJson::<Value>::default());
                    let length_delimited_write = FramedWrite::new(write_socket,LengthDelimitedCodec::new());
                    let mut serializer = tokio_serde::SymmetricallyFramed::new(length_delimited_write,SymmetricalJson::<Value>::default());
                    let config = Arc::clone(&self.config);

                    tokio::spawn(async move {
                        // we accept multiple messages on the same connection, a sort of scripting
                        while let Some(msg)=deserializer.try_next().await.unwrap() {

                            dbg!(msg.clone());
                            let subject = msg.get("subject").unwrap().as_str().unwrap();
                            let command = msg.get("command").unwrap().as_str().unwrap();
                            let params = msg.get("params").unwrap().as_object().unwrap();
                            dbg!(&params);
                            let response = Daemon::handle_request(&config,subject, command, params).await;

                            dbg!(&response);
                            serializer.send(json!({"lines":response}))
                                .await.unwrap();
                            // let _ = writer.write_all(response.as_bytes()).await;
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

    async fn handle_request(config:&Arc<Mutex<SavedConfig>>, subject: &str, command: &str, arguments: &serde_json::Map<String, Value>) -> String {
        match subject {
            "project" => {
                Daemon::handle_project(config,command,arguments).await
            }
            _ => "Unknown command".to_string(),
        }
    }

    async fn handle_project(config:&Arc<Mutex<SavedConfig>>, command: &str, arguments: &serde_json::Map<String, Value>) -> String {
        let mut config = config.lock().await;
        match command {
            "add" => {
                if let Some(serde_json::value::Value::String(project_name)) = arguments.get("project-name") {
                    if config.data.projects.iter().any(|p| p.name == *project_name) {
                        return format!("Project {} already exists", project_name);
                    }
                    config.data.projects.push(Project {
                        name: project_name.clone(),
                        plugins: Vec::new(),
                        properties: HashMap::new(),
                    });
                    config.save();
                    format!("Project {} added", project_name)
                } else {
                    "Invalid command".to_string()
                }
            }
            "list" => {
                let project_names: Vec<String> = config.data.projects.iter().map(|p| p.name.clone()).collect();
                project_names.join(", ")
            }
            _ => "Unknown command".to_string(),
        }
    }
}
