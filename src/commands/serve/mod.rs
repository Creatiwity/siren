mod runner;

use super::common::FolderOptions;
use crate::connectors::ConnectorsBuilders;
use rocket::config::{Config, Environment};
use std::env;
use std::net::ToSocketAddrs;

#[derive(Clap, Debug)]
pub struct ServeFlags {
    /// production, staging or development, will change log level, you can set in environment variable as SIRENE_ENV
    #[clap(arg_enum, long = "env")]
    environment: Option<CmdEnvironment>,

    /// Listen this port, you can set in environment variable as PORT
    #[clap(short = "p", long = "port")]
    port: Option<u16>,

    /// Listen this host, you can set in environment variable as HOST
    #[clap(short = "h", long = "host")]
    host: Option<String>,

    /// API key needed to allow maintenance operation from HTTP, you can set in environment variable as API_KEY
    #[clap(short = "k", long = "api-key")]
    api_key: Option<String>,
}

#[derive(Clap, Debug)]
enum CmdEnvironment {
    Development,
    Staging,
    Production,
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

pub async fn run(flags: ServeFlags, folder_options: FolderOptions, builders: ConnectorsBuilders) {
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

    let addr = format!("{}:{}", host, port)
        .to_socket_addrs()
        .expect("Unable to resolve domain")
        .next()
        .expect("No addresse available");

    let api_key = match flags.api_key {
        Some(key) => Some(key),
        None => env::var("API_KEY").ok(),
    };

    // let config = Config::build(env.into())
    //     .address(host)
    //     .port(port)
    //     .finalize();

    println!("Launching on {:#?} env", env);

    runner::run(addr, api_key, folder_options, builders).await;
}
