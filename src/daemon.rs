use std::path::PathBuf;
use tokio::net::UnixListener;
use tokio_stream::StreamExt;
use tokio_serde::formats::Json;
use crate::config::{Config, Project};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;

struct StreamConsumer {
    read_buffer: String,
}

impl StreamConsumer {
    pub fn new() -> Self {
        StreamConsumer {
            read_buffer: String::new(),
        }
    }
    pub async fn read_line<T>(&mut self, reader: &mut tokio::io::ReadHalf<T>) -> tokio::io::Result<String> {
        // let's read chunks of data from the stream until we find a newline character. we use self.read_buffer to store the data we read so far.
        // we remove the returned data from the buffer and return it as a string
        // on next call we start from the remaining data in the buffer, and then read more from the string if we didn't find a /n
        loop {
            if let Some(pos) = self.read_buffer.find('\n') {
                let mut line = self.read_buffer;
                self.read_buffer=line.split_off(pos + 1);
                line.pop(); // remove the newline character
                return Ok(line);
            }
            let mut buf = vec![0; 1024];
            match reader.read(&mut buf).await {
                Ok(0) => {
                    // the stream has been closed, return the last line
                    return Ok(self.read_buffer.split_off(0));
                }
                Ok(n) => {
                    // append the read data to the buffer
                    self.read_buffer.push_str(&String::from_utf8_lossy(&buf[..n]));
                }
                Err(e) => {
                    eprintln!("Error reading from stream: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
}



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
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    // for every request received, use the handle_request method to generate a response and send it back. we use stream.split() to split the stream into a read half and a write half
                    let (mut reader, mut writer) = stream.into_split();
                    let mut consumer = StreamConsumer::new(reader);
                    // let mut buf_reader = tokio::io::BufReader::new(reader);
                    // let mut buf_writer = tokio::io::BufWriter::new(writer);
                    while let Ok(line)=consumer.read_line().await {
                        match line.as_str() {
                            // kill the daemon
                            "quit" => {
                                return;
                            }
                            // end this connection
                            "end" => {
                                break;
                            }
                            // any other command => handle it
                            _ => {
                                let response = self.handle_request(&line).await;
                                dbg!(&response);
                                let _ = writer.write_all(response.as_bytes()).await;
                            }
                        }
                    }
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
