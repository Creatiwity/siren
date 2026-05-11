#![recursion_limit = "256"]

mod commands;
mod connectors;
mod models;
mod update;

use connectors::ConnectorsBuilders;
use diesel::connection::{InstrumentationEvent, set_default_instrumentation};
use dotenv::dotenv;
use sentry::SentryFutureExt;
use tracing_subscriber::{EnvFilter, prelude::*};

fn main() {
    // Load configuration
    dotenv().ok();

    // Initialize Sentry
    let sentry_dsn = std::env::var("SENTRY_DSN").ok();
    let sirene_env = std::env::var("SIRENE_ENV").unwrap_or("development".to_string());

    let _guard = sentry::init((
        sentry_dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            environment: Some(sirene_env.into()),
            // Capture all traces and spans. Set to a lower value in production
            traces_sample_rate: 1.0,
            enable_logs: true,
            ..sentry::ClientOptions::default()
        },
    ));

    // Load Tracing
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(sentry::integrations::tracing::layer())
        .init();

    // Log Diesel SQL queries via tracing (activate with RUST_LOG=diesel::query=debug)
    set_default_instrumentation(|| {
        Some(Box::new(|event: InstrumentationEvent<'_>| {
            if let InstrumentationEvent::StartQuery { query, .. } = event {
                tracing::debug!(target: "diesel::query", sql = %query);
            }
        }))
    })
    .expect("Failed to set diesel instrumentation");

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Cannot build Tokio runtime")
        .block_on(async {
            // Futures should to be bound to a Hub
            // Learn more at https://docs.rs/sentry-core/latest/sentry_core/#parallelism-concurrency-and-async
            launch().bind_hub(sentry::Hub::current()).await;
        });
}

#[tracing::instrument]
async fn launch() {
    // Load database
    let connectors_builders = ConnectorsBuilders::new();

    // Run command
    commands::run(connectors_builders).await;
}
