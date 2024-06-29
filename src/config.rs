use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use toml;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Project {
    pub name: String,
    pub plugins: Vec<String>,
    pub properties: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    pub projects: Vec<Project>,
}

impl Config {
    pub fn load(path: &PathBuf) -> Self {
        if path.exists() {
            let content = fs::read_to_string(path).expect("Unable to read config file");
            toml::from_str(&content).expect("Invalid config format")
        } else {
            Config::default()
        }
    }

    pub fn save(&self, path: &PathBuf) {
        println!("Saving config to {:?}", path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).expect("Failed to create directories");
            }
        }
        let content = toml::to_string(&self).expect("Failed to serialize config");
        fs::write(path, content).expect("Unable to write config file");
    }
}

pub struct SavedConfig {
    pub path: PathBuf,
    pub data: Config,
}

impl SavedConfig {
    pub fn new(path: PathBuf) -> Self {
        let config = Config::load(&path);
        SavedConfig { path, data:config }
    }

    pub fn save(&self) {
        self.data.save(&self.path);
    }
}
