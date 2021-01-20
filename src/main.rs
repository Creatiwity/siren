#[macro_use]
extern crate clap;
#[cfg(any(target_os = "unix", target_os = "linux"))]
extern crate openssl; // Should be before diesel
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod commands;
mod connectors;
mod models;
mod update;

use connectors::ConnectorsBuilders;
use dotenv::dotenv;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Load configuration
    dotenv().ok();

    // Load Tracing
    tracing_subscriber::fmt::init();

    // Load database
    let connectors_builders = ConnectorsBuilders::new();

    // Run command
    commands::run(connectors_builders).await;
}
