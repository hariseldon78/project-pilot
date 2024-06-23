use std::path::PathBuf;
use structopt::StructOpt;
use tokio::net::UnixStream;
use tokio_serde::formats::Json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(StructOpt)]
pub enum Command {
    ProjectAdd { name: String },
    ProjectList,
}

#[derive(StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub command: Command,
    #[structopt(long, default_value = "/home/roby/.cache/ProjectPilot.socket")]
    pub socket_path: PathBuf,
}

pub async fn run(cli: Cli) {
    let mut stream = UnixStream::connect(cli.socket_path).await.expect("Failed to connect to daemon");
    let (mut reader, mut writer) = stream.split();
    {
    let mut buf = String::new();
    match &cli.command {
        Command::ProjectAdd { name } => {
            buf.push_str("project add ");
            buf.push_str(name);
        }
        Command::ProjectList => {
            buf.push_str("project list");
        }
    }
    buf.push_str("\n");
    writer.write_all(buf.as_bytes()).await.expect("Failed to write to daemon");
    }
    {
        let mut response = String::new();
        match reader.read_to_string(&mut response).await {
            Ok(_) => {
                println!("{}", response);
            }
            Err(e) => {
                eprintln!("Failed to read from daemon: {}", e);
            }
        }

    }


}
