mod runner;

use super::common::FolderOptions;
use crate::connectors::ConnectorsBuilders;
use runner::common::Context;
use std::env;
use std::net::ToSocketAddrs;
use tracing::info;

#[derive(Clap, Debug)]
pub struct ServeFlags {
    /// Configure log level, you can set in environment variable as SIRENE_ENV
    #[clap(arg_enum, long = "env")]
    environment: Option<CmdEnvironment>,

    /// Listen this port, you can set in environment variable as PORT
    #[clap(short = 'p', long = "port")]
    port: Option<u16>,

    /// Listen this host, you can set in environment variable as HOST
    #[clap(short = 'h', long = "host")]
    host: Option<String>,

    /// API key needed to allow maintenance operation from HTTP, you can set in environment variable as API_KEY
    #[clap(short = 'k', long = "api-key")]
    api_key: Option<String>,

    /// Base URL needed to configure asynchronous polling for updates, you can set in environment variable as BASE_URL
    #[clap(short = 'b', long = "base-url")]
    base_url: Option<String>,
}

#[derive(Clap, Debug)]
enum CmdEnvironment {
    Development,
    Staging,
    Production,
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
        .expect("No address available");

    let api_key = match flags.api_key {
        Some(key) => Some(key),
        None => env::var("API_KEY").ok(),
    };

    let base_url = match flags.base_url {
        Some(key) => Some(key),
        None => env::var("BASE_URL").ok(),
    };

    info!("Configuring for {:#?}", env);

    runner::run(
        addr,
        Context {
            builders,
            api_key,
            folder_options,
            base_url,
        },
    )
    .await;
}
