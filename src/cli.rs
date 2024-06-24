use std::path::PathBuf;
use tokio_serde::formats::SymmetricalJson;
use structopt::StructOpt;
use serde_json::Value;
use serde_json::json;
use tokio::net::UnixStream;
use tokio_serde::formats::Json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};
use futures::sink::SinkExt;

#[derive(StructOpt)]
pub enum Command {
    ProjectAdd { name: String },
    ProjectList,
}

#[derive(StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub command: Command,
}

pub async fn run(cli: Cli,socket_path: &str) {
    let stream = UnixStream::connect(socket_path).await.expect("Failed to connect to daemon");
    let length_delimited = FramedWrite::new(stream,LengthDelimitedCodec::new());
    // let mut serializer: tokio_serde::Framed<FramedWrite<tokio::net::UnixStream, LengthDelimitedCodec>, Value, Value, Json<Value, Value>>  = tokio_serde::SymmetricallyFramed::new(length_delimited,SymmetricalJson::<Value>::default());
    let mut serializer = tokio_serde::SymmetricallyFramed::new(length_delimited,SymmetricalJson::<Value>::default());

    serializer.send(json!({
        "subject":"project",
        "command":"add",
        "params":["test"]
    }))
        .await.unwrap()



























    // let (mut reader, mut writer) = stream.split();
    // {
    // let mut buf =  String::new();
    // match &cli.command {
    //     Command::ProjectAdd { name } => {
    //         buf.push_str("project add ");
    //         buf.push_str(name);
    //     }
    //     Command::ProjectList => {
    //         buf.push_str("project list");
    //     }
    // }
    // buf.push_str("\n");
    // writer.write_all(buf.as_bytes()).await.expect("Failed to write to daemon");
    // }
    // {
    //     let mut response = String::new();
    //     match reader.read_to_string(&mut response).await {
    //         Ok(_) => {
    //             println!("{}", response);
    //         }
    //         Err(e) => {
    //             eprintln!("Failed to read from daemon: {}", e);
    //         }
    //     }
    // }


}
