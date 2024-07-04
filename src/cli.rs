use crate::config::{Config, Project};
use crate::daemon::Daemon;
use clap::{arg, command, Arg, Command};
use futures::executor::block_on;
use futures::sink::SinkExt;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::io::{split, AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio_serde::formats::SymmetricalJson;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub struct Cli {}

pub async fn run(cli: Cli) {
    let home: String = env::var("HOME").unwrap();
    let socket_path: String = home.clone() + "/.cache/project-pilot.socket";

    let mut command_line = command!()
        // .next_line_help(true)
        .subcommand(
            Command::new("project")
                .about("work with projects")
                .subcommands([
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
                    Command::new("list").about("list the defined projects"),
                ]),
        )
        .subcommand(
            Command::new("event")
                .about("work with events")
                .subcommands([
                    Command::new("trigger")
                        .about("trigger an event")
                        .arg(Arg::new("event-name").required(true)),
                    Command::new("list").about("list the possible events"),
                ]),
        )
        .subcommand(
            Command::new("plugin")
                .about("work with plugins")
                .arg(Arg::new("plugin").required(true))
                .subcommands([
                    Command::new("run")
                        .about("run a plugin action")
                        .arg(Arg::new("action").required(true))
                        .arg(Arg::new("project-name")),
                    Command::new("list-actions")
                        .about("list the available actions for this plugin"),
                ]),
        )
        .subcommand(
            Command::new("daemon")
                .about("work with the background process")
                .subcommands([
                    Command::new("start")
                        // for now only in foreground
                        .about("start the daemon porcess")
                        .arg(
                            Arg::new("force")
                                .long("force")
                                .action(clap::ArgAction::SetTrue)
                                .help("rebind the socket if it's found"),
                        ),
                    Command::new("status").about("get info about the running daemon porcess"),
                    Command::new("stop").about("gracefully closes the daemon process"),
                ]),
        );

    let clargs = command_line.clone().get_matches();

    let (subject, sub_args) = if let Some(res) = clargs.subcommand() {
        res
    } else {
        command_line.print_help().unwrap();
        return;
    };
    let (command, com_args) = if let Some(res) = sub_args.subcommand() {
        res
    } else {
        command_line
            .find_subcommand(subject)
            .unwrap()
            .clone()
            .print_help()
            .unwrap();
        return;
    };
    if subject == "daemon" && command == "start" {
        println!("Starting daemon");
        let config_path = PathBuf::from(home.clone() + "/.config/project-pilot/config.toml");
        let mut daemon = Daemon::new(config_path);
        let force: bool = *(com_args.get_one("force").unwrap());
        // let force: bool=match force_arg {
        //     Some(x) => *x,
        //     None => false,
        // };
        daemon.start(&socket_path,force).await;
        return;
    }

    fn collect_args(args: &clap::ArgMatches) -> impl Iterator<Item = (String, String)> + '_ {
        args
            .ids()
            .filter_map(|id| {
                args
                    .get_one::<String>(id.as_str())
                    .map(|v| (String::from(id.as_str()), v.clone()))
            })
    }
    let args_map: HashMap<String, String> = collect_args(sub_args).chain(collect_args(com_args)).collect();

    let stream = UnixStream::connect(socket_path)
        .await
        .expect("Failed to connect to daemon");
    let (read_socket, write_socket) = split(stream);

    let length_delimited_write = FramedWrite::new(write_socket, LengthDelimitedCodec::new());
    let mut serializer = tokio_serde::SymmetricallyFramed::new(
        length_delimited_write,
        SymmetricalJson::<Value>::default(),
    );
    serializer
        .send(json!({
            "subject":subject,
            "command":command,
            "params":args_map
        }))
        .await
        .unwrap();

    let length_delimited_read = FramedRead::new(read_socket, LengthDelimitedCodec::new());
    let mut deserializer = tokio_serde::SymmetricallyFramed::new(
        length_delimited_read,
        SymmetricalJson::<Value>::default(),
    );
    block_on(async move {
        let msg = deserializer.try_next().await.unwrap().unwrap();
        println!("{}", msg.get("lines").unwrap().as_str().unwrap());
    });
}
