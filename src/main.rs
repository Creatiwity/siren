#![recursion_limit = "256"]

mod commands;
mod connectors;
mod models;
mod update;

use connectors::ConnectorsBuilders;
use dotenv::dotenv;

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
