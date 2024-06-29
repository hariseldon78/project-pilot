use std::boxed::Box;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::config::Project;
use serde_json::{Map, Value};

pub trait Plugin: Send + Sync {
    fn name(&self) -> String;
    fn on_event(&self, event: &str, project: &mut Project, arguments: &Map<String,Value>);
}

pub struct PluginFactory {
    map: HashMap<String, Mutex<Box<dyn Plugin>>>
}
impl PluginFactory {
    pub fn new() -> PluginFactory {
        PluginFactory {
            map: HashMap::new()
        }
    }
    pub async fn register_plugin(&mut self, plugin: Mutex<Box<dyn Plugin>>) {
        let name = plugin.lock().await.name().clone();
        self.map.insert(name, plugin);
    }
    pub fn get_plugin(&self, name: &str) -> Option<&Mutex<Box<dyn Plugin>>> {
        self.map.get(name)
    }
}

pub struct TmuxPlugin {
}

impl Plugin for TmuxPlugin {
    fn name(&self) -> String {
        "tmux".to_string()
    }
    fn on_event(&self, event: &str, project: &mut Project, arguments: &Map<String,Value>) {
        println!("tmux plugin: {} project: {}", event,project.name);
        match event {
            // "start" => {
            //     let session = arguments.get("session").unwrap().as_str().unwrap();
            //     let window = arguments.get("window").unwrap().as_str().unwrap();
            //     let pane = arguments.get("pane").unwrap().as_str().unwrap();
            //     let command = arguments.get("command").unwrap().as_str().unwrap();
            //     let tmux_command = format!("tmux new-session -d -s {} -n {} -c {} -P \"{}\"", session, window, project.path, command);
            //     let output = std::process::Command::new("sh")
            //         .arg("-c")
            //         .arg(tmux_command)
            //         .output()
            //         .expect("failed to execute process");
            //     if !output.status.success() {
            //         eprintln!("failed to execute tmux command: {}", String::from_utf8_lossy(&output.stderr));
            //     }
            // }
            // "stop" => {
            //     let session = arguments.get("session").unwrap().as_str().unwrap();
            //     let tmux_command = format!("tmux kill-session -t {}", session);
            //     let output = std::process::Command::new("sh")
            //         .arg("-c")
            //         .arg(tmux_command)
            //         .output()
            //         .expect("failed to execute process");
            //     if !output.status.success() {
            //         eprintln!("failed to execute tmux command: {}", String::from_utf8_lossy(&output.stderr));
            //     }
            // }
            _ => {}
        }
    }
}
