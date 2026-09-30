mod runner;

use crate::connectors::ConnectorsBuilders;
use runner::common::Context;
use std::net::ToSocketAddrs;
use std::time::Duration;
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

    /// On SIGTERM, keep serving for this many seconds with readiness failing,
    /// so the load balancer stops routing here before the listener closes
    #[clap(
        long = "shutdown-delay",
        env = "SHUTDOWN_DELAY_SECONDS",
        default_value_t = 0
    )]
    shutdown_delay: u64,
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
            shutting_down: Default::default(),
        },
        Duration::from_secs(flags.shutdown_delay),
    )
    .await;
}
