#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]
mod cli;
mod config;
mod daemon;
mod event;
mod plugin;

use cli::{Cli, run};
use structopt::StructOpt;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let cli = Cli{};
    run(cli).await;
}
