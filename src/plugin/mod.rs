use crate::config::Project;
use crate::event::Event;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::boxed::Box;
use std::collections::HashMap;
use tokio::sync::Mutex;

pub trait Plugin: Send + Sync {
    fn name(&self) -> String;
    fn on_event(&self, event: Event, project: &mut Project, arguments: &Map<String,Value>);
    fn list_actions(&self) -> Vec<String>;
    fn run_action(&self, action: &str, project: &mut Project, arguments: &Map<String,Value>) -> Result<String,String>;
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

// export plugins
pub mod tmux;
pub use crate::plugin::tmux::TmuxPlugin;
