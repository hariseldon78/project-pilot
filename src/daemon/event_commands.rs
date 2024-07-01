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
    pub async fn handle_event(
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
                    if let Ok(event) = Event::from_str(event_name) {
                        for project in config.data.projects.iter_mut() {
                            for plugin_name in &project.plugins.clone() {
                                let plugin =
                                    &plugin_manager.get_plugin(plugin_name).unwrap().lock().await;
                                plugin.on_event(event, project, arguments);
                            }
                        }
                        "Event triggered".to_string()
                    } else {
                        "Invalid event".to_string()
                    }
                } else {
                    "Missign event name".to_string()
                }
            }
            "list" => {
                let event_list: String = Event::iter()
                    .map(|event| event.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                event_list
            }
            _ => "Unknown command".to_string(),
        }
    }
}
