use crate::config::{Project, SavedConfig};
use crate::event::Event;
use crate::plugin::{Plugin, PluginFactory, TmuxPlugin};
use crate::daemon::Daemon;
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

impl Daemon {
    pub async fn handle_project(
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
