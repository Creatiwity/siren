mod runner;

use crate::connectors::ConnectorsBuilders;
use runner::common::Context;
use std::net::ToSocketAddrs;
use tracing::info;

#[derive(clap::Args, Debug)]
pub struct ServeFlags {
    /// Configure log level
    #[clap(value_enum, long = "env", env = "SIRENE_ENV")]
    environment: CmdEnvironment,

    /// Listen this port
    #[clap(long = "port", env)]
    port: u16,

    /// Listen this host
    #[clap(long = "host", env)]
    host: String,

    /// API key needed to allow maintenance operation from HTTP
    #[clap(long = "api-key", env)]
    api_key: Option<String>,

    /// Base URL needed to configure asynchronous polling for updates
    #[clap(long = "base-url", env)]
    base_url: Option<String>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum CmdEnvironment {
    Development,
    Staging,
    Production,
}

pub async fn run(flags: ServeFlags, builders: ConnectorsBuilders) {
    let addr = format!("{}:{}", flags.host, flags.port)
        .to_socket_addrs()
        .expect("Unable to resolve domain")
        .next()
        .expect("No address available");

    info!("Configuring for {:#?}", flags.environment);

    runner::run(
        addr,
        Context {
            builders,
            api_key: flags.api_key,
            base_url: flags.base_url,
        },
    )
    .await;
}
