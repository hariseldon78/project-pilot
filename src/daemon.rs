use crate::config::{Project, SavedConfig};
use crate::plugin::{Plugin, PluginFactory, TmuxPlugin};
use futures::sink::SinkExt;
use serde_json::{json, Value};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tokio_serde::formats::SymmetricalJson;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

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
        self.plugin_manager.lock().await.register_plugin(Mutex::new(Box::new(TmuxPlugin{}))).await;
        
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
                            let response =
                                Daemon::handle_request(&config, &plugin_manager, subject, command, params).await;

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

    async fn handle_event(
        config: &Arc<Mutex<SavedConfig>>,
        plugin_manager: &Arc<Mutex<PluginFactory>>,
        command: &str,
        arguments: &serde_json::Map<String, Value>,
    ) -> String {
        let mut config = config.lock().await;
        let plugin_manager = plugin_manager.lock().await;
        match command {
            "trigger" => {
                if let Some(serde_json::value::Value::String(event_name)) =
                    arguments.get("event-name")
                {
                    for project in config.data.projects.iter_mut() {
                        for plugin_name in &project.plugins.clone() {
                            let plugin = &plugin_manager
                                .get_plugin(plugin_name)
                                .unwrap()
                                .lock().await;
                            plugin.on_event(event_name,  project, arguments);
                        }
                    }
                    "Event triggered".to_string()
                } else {
                    "Invalid command".to_string()
                }
            }
            _ => "Unknown command".to_string(),
        }
    }

    async fn handle_project(
        config: &Arc<Mutex<SavedConfig>>,
        plugin_manager: &Arc<Mutex<PluginFactory>>,
        command: &str,
        arguments: &serde_json::Map<String, Value>,
    ) -> String {
        let mut config = config.lock().await;
        match command {
            "add" => {
                if let Some(serde_json::value::Value::String(project_name)) =
                    arguments.get("project-name")
                {
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
            "del" => {
                if let Some(serde_json::value::Value::String(project_name)) =
                    arguments.get("project-name")
                {
                    if let Some(index) = config
                        .data
                        .projects
                        .iter()
                        .position(|p| p.name == *project_name)
                    {
                        config.data.projects.remove(index);
                        config.save();
                        format!("Project {} removed", project_name)
                    } else {
                        format!("Project {} not found", project_name)
                    }
                } else {
                    "Invalid command".to_string()
                }
            }
            "info" => {
                // return information about a project, with the plugins list and properties
                if let Some(serde_json::value::Value::String(project_name)) =
                    arguments.get("project-name")
                {
                    if let Some(project) = config
                        .data
                        .projects
                        .iter()
                        .find(|p| p.name == *project_name)
                    {
                        let plugins = project.plugins.join(", ");
                        let properties: Vec<String> = project
                            .properties
                            .iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect();
                        let properties = properties.join(", ");
                        format!(
                            "Project: {}\nPlugins: {}\nProperties: {}",
                            project_name, plugins, properties
                        )
                    } else {
                        format!("Project {} not found", project_name)
                    }
                } else {
                    "Invalid command".to_string()
                }
            }
            "set-property" => {
                let project_name = arguments.get("project-name").unwrap().as_str().unwrap();
                let property = arguments.get("property").unwrap().as_str().unwrap();
                let value = arguments.get("value").unwrap().as_str().unwrap();

                if let Some(project) = config
                    .data
                    .projects
                    .iter_mut()
                    .find(|p| p.name == project_name)
                {
                    project
                        .properties
                        .insert(property.to_string(), value.to_string());
                    config.save();
                    format!(
                        "Property {} set to {} for project {}",
                        property, value, project_name
                    )
                } else {
                    format!("Project {} not found", project_name)
                }
            }
            "get-property" => {
                let project_name = arguments.get("project-name").unwrap().as_str().unwrap();
                let property = arguments.get("property").unwrap().as_str().unwrap();

                if let Some(project) = config.data.projects.iter().find(|p| p.name == project_name)
                {
                    if let Some(value) = project.properties.get(property) {
                        value.clone()
                    } else {
                        format!(
                            "Property {} not found for project {}",
                            property, project_name
                        )
                    }
                } else {
                    format!("Project {} not found", project_name)
                }
            }
            "del-property" => {
                let project_name = arguments.get("project-name").unwrap().as_str().unwrap();
                let property = arguments.get("property").unwrap().as_str().unwrap();

                if let Some(project) = config
                    .data
                    .projects
                    .iter_mut()
                    .find(|p| p.name == project_name)
                {
                    if let Some(_) = project.properties.remove(property) {
                        config.save();
                        format!("Property {} removed for project {}", property, project_name)
                    } else {
                        format!(
                            "Property {} not found for project {}",
                            property, project_name
                        )
                    }
                } else {
                    format!("Project {} not found", project_name)
                }
            }
            "enable-plugin" => {
                let project_name = arguments.get("project-name").unwrap().as_str().unwrap();
                let plugin_name = arguments.get("plugin").unwrap().as_str().unwrap();

                if let Some(project) = config
                    .data
                    .projects
                    .iter_mut()
                    .find(|p| p.name == project_name)
                {
                    if !project.plugins.contains(&plugin_name.to_string()) {
                        project.plugins.push(plugin_name.to_string());
                        config.save();
                        format!(
                            "Plugin {} enabled for project {}",
                            plugin_name, project_name
                        )
                    } else {
                        format!(
                            "Plugin {} already enabled for project {}",
                            plugin_name, project_name
                        )
                    }
                } else {
                    format!("Project {} not found", project_name)
                }
            }
            "disable-plugin" => {
                let project_name = arguments.get("project-name").unwrap().as_str().unwrap();
                let plugin_name = arguments.get("plugin").unwrap().as_str().unwrap();

                if let Some(project) = config
                    .data
                    .projects
                    .iter_mut()
                    .find(|p| p.name == project_name)
                {
                    if let Some(index) = project.plugins.iter().position(|p| p == plugin_name) {
                        project.plugins.remove(index);
                        config.save();
                        format!(
                            "Plugin {} disabled for project {}",
                            plugin_name, project_name
                        )
                    } else {
                        format!(
                            "Plugin {} not enabled for project {}",
                            plugin_name, project_name
                        )
                    }
                } else {
                    format!("Project {} not found", project_name)
                }
            }
            "list" => {
                let project_names: Vec<String> = config
                    .data
                    .projects
                    .iter()
                    .map(|p| p.name.clone())
                    .collect();
                project_names.join(", ")
            }
            _ => "Unknown command".to_string(),
        }
    }
}
