#![cfg_attr(feature = "dev", allow(unstable_features))]

#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", allow(items_after_statements))]

#![cfg_attr(feature = "trace", feature(custom_attribute, plugin))]
#![cfg_attr(feature = "trace", plugin(trace))]


#[macro_use] extern crate log;
extern crate log4rs;

extern crate compat;
extern crate fs;

pub mod release;

use compat::getpid;
use fs::server::config::Config;
use fs::server::Server;
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
        level: trace

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


fn main() {
    init_logger();

    info!("Irbis FS server v. {}; compiled with {}", env!("CARGO_PKG_VERSION"), RUSTC_VERSION);

    let config = Config::new();

    args().nth(1).then(|f| Config.load(f).map_err(|err| {exit(1); err} ));

    let (port, daemonize) = (config.port, config.daemonize);
    
    let mut server = match Server::new(config) {
        Ok(server) => server,
        Err(error) => {
            error!("{:?}", error);
            panic!("{:?}", error);
        },
    };

    let run_id = {
        let mut db = server.db.lock().unwrap();
        db.version = env!("CARGO_PKG_VERSION");
        db.rustc_version = RUSTC_VERSION;
        db.run_id
    };

    if !daemonize {
        info!("run ID: {}; PID: {}, port: {}", run_id, getpid(), port);
    }

    server.run();
}
