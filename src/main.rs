mod config;
mod daemon;
mod cli;

use cli::{Cli, run};
use daemon::Daemon;
use structopt::StructOpt;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    dbg!(&std::env::args());

    // Start daemon if no command is provided
    if std::env::args().len() == 1 {
        let config_path = PathBuf::from("/home/roby/.config/ProjectPilot/config.toml");
        let socket_path = "/home/roby/.cache/ProjectPilot.socket";
        let mut daemon = Daemon::new(config_path);
        daemon.start(socket_path).await;
    } else {
        let cli = Cli::from_args();
        run(cli).await;
    }
}
