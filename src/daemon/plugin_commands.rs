use crate::config::{Project, SavedConfig};
use crate::daemon::Daemon;
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

impl Daemon {
    pub async fn handle_plugin(
        config: &Arc<Mutex<SavedConfig>>,
        plugin_manager: &Arc<Mutex<PluginFactory>>,
        command: &str,
        arguments: &serde_json::Map<String, Value>,
    ) -> String {
        let mut config = config.lock().await;
        let plugin_manager = plugin_manager.lock().await;
        match command {
            "run" => {
                // if let Some(serde_json::value::Value::String(event_name)) =
                //     arguments.get("event-name")
                // {
                //     if let Ok(event) = Event::from_str(event_name) {
                //         for project in config.data.projects.iter_mut() {
                //             for plugin_name in &project.plugins.clone() {
                //                 let plugin =
                //                     &plugin_manager.get_plugin(plugin_name).unwrap().lock().await;
                //                 plugin.on_event(event, project, arguments);
                //             }
                //         }
                //         "Event triggered".to_string()
                //     } else {
                //         "Invalid event".to_string()
                //     }
                // } else {
                //     "Missign event name".to_string()
                // }
                let plugin_name = arguments.get("plugin").unwrap().as_str().unwrap();
                let action = arguments.get("action").unwrap().as_str().unwrap();
                let project_name = arguments.get("project-name").unwrap().as_str().unwrap();
                let project = config.data.projects.iter_mut().find(|p| p.name == project_name).unwrap();
                let plugin = plugin_manager.get_plugin(plugin_name).unwrap().lock().await;
                plugin.run_action(action, project, arguments).unwrap()

            }
            "list-actions" => {
                let plugin = arguments.get("plugin").unwrap().as_str().unwrap();
                return plugin_manager
                    .get_plugin(plugin)
                    .unwrap()
                    .lock()
                    .await
                    .list_actions()
                    .join(", ");
            }
            _ => "Unknown command".to_string(),
        }
    }
}
