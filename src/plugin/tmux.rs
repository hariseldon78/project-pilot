use crate::config::Project;
use crate::event::Event;
use crate::plugin::Plugin;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::boxed::Box;
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct TmuxPlugin {
}

fn tmux_has_session(session_name: &str) -> bool {
    std::process::Command::new("tmux")
        .arg("has-session")
        .arg("-t")
        .arg(session_name)
        .output()
        .expect("failed to execute process")
        .status.success()
}
impl Plugin for TmuxPlugin {
    fn name(&self) -> String {
        "tmux".to_string()
    }
    fn on_event(&self, event: Event, project: &mut Project, arguments: &Map<String,Value>) {
        println!("tmux plugin: {} project: {}", event,project.name);
        match event {
            Event::PluginEnable => {
                let session_name = project.name.clone();
                if !tmux_has_session(&session_name) {
                    let session_path = if let Some(project_path)=project.properties.get("path") {
                        format!("-c {}",project_path.as_str()).to_string()
                    } else {
                        "".to_string()
                    };
                    let tmux_command = format!("tmux new-session -d -s {} {} -P", session_name, session_path);
                    let output = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&tmux_command)
                        .output()
                        .expect("failed to execute process");
                    println!("tmux command: {}", &tmux_command);
                    if !output.status.success() {
                        eprintln!("failed to execute tmux command: {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
            }
            Event::PluginDisable => {
                let session_name = project.name.clone();
                if tmux_has_session(&session_name) {
                    let tmux_command = format!("tmux kill-session -t {}", session_name);
                    let output = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&tmux_command)
                        .output()
                        .expect("failed to execute process");
                    println!("tmux command: {}", &tmux_command);
                    if !output.status.success() {
                        eprintln!("failed to execute tmux command: {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
            }
            _ => {}
        }
    }
    fn list_actions(&self) -> Vec<String> {
        vec!["gen_init_terminal".to_string()]
    }
    fn run_action(&self, action: &str, project: &mut Project, arguments: &Map<String,Value>) -> Result<String,String> {
        match action {
            "gen_init_terminal" => {
                let session_name = project.name.clone();
                if project.plugins.contains(&"tmux".to_string()) {
                    Ok(format!("tmux attach-session -t {}", session_name))
                } else {
                    Err("tmux plugin is not enabled".to_string())
                }
            }
            _ => {
                Err(format!("unknown action: {}", action))
            }
        }
    }
}
