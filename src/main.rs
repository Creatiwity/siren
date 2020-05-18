#![feature(proc_macro_hygiene, decl_macro)]

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
#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate zip;

mod commands;
mod connectors;
mod models;
mod update;

use chrono::Utc;
use connectors::ConnectorsBuilders;
use dotenv::dotenv;
use tokio::prelude::*;

#[tokio::main]
async fn main() {
    // Load configuration
    dotenv().ok();

    // Load database
    let connectors_builders = ConnectorsBuilders::new();

    // Close running updates
    let connectors = connectors_builders.create(false);
    models::update_metadata::error_update(
        &connectors,
        String::from("Program unexpectedly closed"),
        Utc::now(),
    )
    .unwrap(); // Fail launch in case of error

    // Run command
    commands::run(connectors_builders);
}
