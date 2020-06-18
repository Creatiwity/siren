#![feature(proc_macro_hygiene, decl_macro, async_closure)]

extern crate chrono;
#[macro_use]
extern crate clap;
extern crate custom_error;
extern crate openssl; // Should be before diesel
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
extern crate dotenv;
extern crate r2d2;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate warp;
extern crate zip;

mod commands;
mod connectors;
mod models;
mod update;

use chrono::Utc;
use connectors::ConnectorsBuilders;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    // Load configuration
    dotenv().ok();

    // Load Logger
    pretty_env_logger::init();

    // Load database
    let connectors_builders = ConnectorsBuilders::new();

    // Close running updates
    let connectors = connectors_builders.create();
    models::update_metadata::error_update(
        &connectors,
        String::from("Program unexpectedly closed"),
        Utc::now(),
    )
    .unwrap(); // Fail launch in case of error

    // Run command
    commands::run(connectors_builders).await;
}
