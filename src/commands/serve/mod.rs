mod runner;

use super::common::FolderOptions;
use crate::connectors::ConnectorsBuilders;
use rocket::config::{Config, Environment};
use std::env;

#[derive(Clap, Debug)]
pub struct ServeFlags {
    /// Listen this port
    #[clap(long = "env")]
    environment: Option<CmdEnvironment>,

    /// Listen this port
    #[clap(short = "p", long = "port")]
    port: Option<u16>,

    /// Listen this host
    #[clap(short = "h", long = "host")]
    host: Option<String>,
}

arg_enum! {
    #[derive(Debug)]
    enum CmdEnvironment {
        Development,
        Staging,
        Production
    }
}

impl From<CmdEnvironment> for Environment {
    fn from(env: CmdEnvironment) -> Self {
        match env {
            CmdEnvironment::Development => Environment::Development,
            CmdEnvironment::Staging => Environment::Staging,
            CmdEnvironment::Production => Environment::Production,
        }
    }
}

impl CmdEnvironment {
    pub fn from_str(s: String) -> Option<CmdEnvironment> {
        match s.as_str() {
            "development" => Some(CmdEnvironment::Development),
            "staging" => Some(CmdEnvironment::Staging),
            "production" => Some(CmdEnvironment::Production),
            _ => None,
        }
    }
}

pub fn run(flags: ServeFlags, folder_options: FolderOptions, builders: ConnectorsBuilders) {
    let env = flags.environment.unwrap_or_else(|| {
        CmdEnvironment::from_str(env::var("SIRENE_ENV").expect("Missing SIRENE_ENV"))
            .expect("Invalid SIRENE_ENV")
    });

    let port = flags.port.unwrap_or_else(|| {
        env::var("PORT")
            .expect("Missing PORT")
            .parse()
            .expect("Invalid PORT")
    });

    let host = flags
        .host
        .unwrap_or_else(|| env::var("HOST").expect("Missing HOST"));

    let config = Config::build(env.into())
        .address(host)
        .port(port)
        .finalize();

    runner::run(config.unwrap(), folder_options, builders)
}
