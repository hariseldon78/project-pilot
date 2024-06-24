#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]
mod config;
mod daemon;
mod cli;

use std::env;
use cli::{Cli, run};
use daemon::Daemon;
use structopt::StructOpt;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let home:String = env::var("HOME").unwrap();
    let socket_path:String = home.clone()+"/.cache/project-pilot.socket";


    // Start daemon if no command is provided
    if std::env::args().len() == 1 {
        println!("Starting daemon");
        let config_path = PathBuf::from(home+".config/project-pilot/config.toml");
        let mut daemon = Daemon::new(config_path);
        daemon.start(&socket_path).await;
    } else {
        let cli = Cli::from_args();
        run(cli,&socket_path).await;
    }
}
