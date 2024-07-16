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
    pub async fn handle_global(
        config: &Arc<Mutex<SavedConfig>>,
        plugin_manager: &Arc<Mutex<PluginFactory>>,
        command: &str,
        arguments: &serde_json::Map<String, Value>,
    ) -> String {
        let mut config = config.lock().await;
        match command {
            "set-property" => {
                let property = arguments.get("property").unwrap().as_str().unwrap();
                let value = arguments.get("value").unwrap().as_str().unwrap();

                config.data.properties.insert(property.to_string(), value.to_string());
                config.save();
                format!(
                    "Global property {} set to {}",
                    property, value
                )
            }
            "get-property" => {
                let property = arguments.get("property").unwrap().as_str().unwrap();

                if let Some(value) = config.data.properties.get(property) {
                    value.clone()
                } else {
                    format!("Global property {} not found", property)
                }
            }
            "del-property" => {
                let property = arguments.get("property").unwrap().as_str().unwrap();

                if let Some(_) = config.data.properties.remove(property) {
                    config.save();
                    format!("Global property {} removed", property)
                } else {
                    format!("Global property {} not found", property)
                }

            }
            "list-properties" => {
                let mut properties = vec![];
                for (key, value) in config.data.properties.iter() {
                    properties.push(json!({
                        "property": key,
                        "value": value
                    }));
                }
                json!(properties).to_string()
            }
            _ => "Unknown command".to_string(),
        }
    }
}
