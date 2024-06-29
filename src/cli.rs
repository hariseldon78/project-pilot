use clap::{Arg,arg,Command,command};
use crate::config::{Config, Project};
use futures::sink::SinkExt;
use futures::executor::block_on;
use serde_json::{Value,json};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::io::{AsyncReadExt, AsyncWriteExt, split};
use tokio::net::{UnixListener, UnixStream};
use tokio_serde::formats::SymmetricalJson;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};


pub struct Cli {
}

pub async fn run(cli: Cli,socket_path: &str) {

    let clargs = command!() 
        .next_line_help(true)
        .subcommand(Command::new("project")
                    .subcommands( [
                        Command::new("add")
                            .about("add a new project")
                            .arg(Arg::new("project-name")),
                        Command::new("del")
                            .about("delete a project")
                            .arg(Arg::new("project-name")),
                        Command::new("info")
                            .about("get info about a project")
                            .arg(Arg::new("project-name")),
                        Command::new("get-property")
                            .about("get the value of a property")
                            .arg(Arg::new("project-name"))
                            .arg(Arg::new("property")),
                        Command::new("del-property")
                            .about("delete a property")
                            .arg(Arg::new("project-name"))
                            .arg(Arg::new("property")),
                        Command::new("set-property")
                            .about("set property to a value")
                            .arg(Arg::new("project-name"))
                            .arg(Arg::new("property"))
                            .arg(Arg::new("value")),
                        Command::new("disable-plugin")
                            .about("disable a plugin for a project")
                            .arg(Arg::new("project-name"))
                            .arg(Arg::new("plugin")),
                        Command::new("enable-plugin")
                            .about("enable a plugin for a project")
                            .arg(Arg::new("project-name"))
                            .arg(Arg::new("plugin")),
                        Command::new("list")
                            .about("list the defined projects")]))
        .get_matches();


    let (subject,sub_args) = clargs.subcommand().expect("Failed to parse subject");
    let (command, com_args) = sub_args.subcommand().expect("Failed to parse command");
    let args_map: HashMap<String, String> = com_args.ids().filter_map(|id| {
        com_args.get_one::<String>(id.as_str()).map(|v| (String::from(id.as_str()), v.clone()))
    }).collect();

    let stream = UnixStream::connect(socket_path).await.expect("Failed to connect to daemon");
    let (read_socket,write_socket)=split(stream);


    let length_delimited_write = FramedWrite::new(write_socket,LengthDelimitedCodec::new());
    let mut serializer = tokio_serde::SymmetricallyFramed::new(length_delimited_write,SymmetricalJson::<Value>::default());
    serializer.send(json!({
        "subject":subject,
        "command":command,
        "params":args_map
    }))
        .await.unwrap();


    let length_delimited_read = FramedRead::new(read_socket, LengthDelimitedCodec::new());
    let mut deserializer = tokio_serde::SymmetricallyFramed::new(length_delimited_read, SymmetricalJson::<Value>::default());
    block_on(async move {
        let msg=deserializer.try_next().await.unwrap().unwrap();
        println!("{}",msg.get("lines").unwrap().as_str().unwrap());
    });

}
