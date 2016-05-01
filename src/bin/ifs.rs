#![cfg_attr(feature = "dev", allow(unstable_features))]

#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#![cfg_attr(feature = "trace", feature(custom_attribute, plugin))]
#![cfg_attr(feature = "trace", plugin(trace))]

#![cfg_attr(feature = "clippy", allow(items_after_statements))]

#![feature(question_mark)]


#[cfg(all(feature = "bench", test))]
extern crate test;

#[macro_use] extern crate log;
extern crate log4rs;

extern crate fs;

pub mod release;

use std::env;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use fs::client::config::{Config, Target};
use fs::client::Client;
use fs::proto::content::state::ContentState;
use release::*;


static CONFIG: &'static str = r#"
appenders:
    stdout:
        kind: console
        encoder:
            pattern: "{l:<10}{m}{n}"

root:
    level: info
    appenders:
        - stdout

loggers:
    fs:
        level: info

"#;


fn init_logger() {
    if log4rs::init_file("log.toml", Default::default()).is_err() {
        init_default();
    };

    fn init_default() {
        let creator = Default::default();
        let config = log4rs::file::Config::parse(CONFIG, log4rs::file::Format::Yaml, &creator)
            .expect("default config is valid")
            .into_config();

        log4rs::init_config(config).unwrap();
    }
}


fn help() {
    println!("

USAGE:
    ifs getinfo
        Get an info about the server.
    ifs copyfrom <path>
        Add a file from the local filesystem.
");
}


fn main() {
    init_logger();

    info!("Irbis FS client v. {}; compiled with {}", env!("CARGO_PKG_VERSION"), RUSTC_VERSION);

    let args: Vec<String> = env::args().collect();
//    let (self_command, args) = (&args[0], &args[1..]);
    let (_, args) = (&args[0], &args[1..]);

    if args.is_empty() {
        help();
        return;
    }
    let (command, args) = (&args[0], &args[1..]);

//    println!("{:?}", args);

    match command.as_str() {
        "getinfo"   => if !args.is_empty() { help(); return; },
        "copyfrom"  => if args.len() != 1 { help(); return; },
        _           => { println!("Unknown command {}", command); help(); return; },
    }

    let target = Target::Tcp(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1313));
    info!("  ::  Connecting to {:?}", target);

    let config = Config::new(target);
    let mut client = Client::new(config);

    let ifs = match client.connect() {
        Ok(ifs) => ifs,
        Err(error) => panic!(format!("{:?}", error)),
    };

    info!("  ::  Connected");


    match command.as_str() {
        "getinfo" => {
            match ifs.info().unwrap().wait().unwrap() {
                ContentState::GetInfo(ref info) => info!("Server info: {:?}", &info.response),
                _ => unreachable!(),
            };
        },
        "copyfrom" => {
            match ifs.copy_from(args[0].as_str()).unwrap().wait().unwrap() {
                ContentState::CopyFrom(ref copy_from) => info!("Copy result: {:?}", &copy_from.result),
                _ => unreachable!(),
            };
        },
        _ => unreachable!(),
    }
}
