#![recursion_limit = "256"]

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

mod commands;
mod connectors;
mod diesel_instrumentation;
#[cfg(feature = "geocoding")]
mod geocoding;
mod models;
mod sentry_crons;
mod telemetry;
#[cfg(test)]
mod tests;
mod update;

use dotenv::dotenv;
use sentry::SentryFutureExt;

fn main() {
    // Load configuration
    dotenv().ok();

    // Initialize Sentry
    let sentry_dsn = std::env::var("SENTRY_DSN").ok();
    let sirene_env = std::env::var("SIRENE_ENV").unwrap_or("development".to_string());

    let _guard = sentry::init((
        sentry_dsn,
        sentry::ClientOptions::new()
            .maybe_release(sentry::release_name!())
            .environment(sirene_env)
            // Capture all traces and spans. Set to a lower value in production
            .traces_sample_rate(1.0),
    ));

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Cannot build Tokio runtime")
        .block_on(async {
            telemetry::init();

            // Futures should to be bound to a Hub
            // Learn more at https://docs.rs/sentry-core/latest/sentry_core/#parallelism-concurrency-and-async
            launch().bind_hub(sentry::Hub::current()).await;

            telemetry::shutdown();
        });
}

#[tracing::instrument]
async fn launch() {
    // Each command decides how to connect, and whether to migrate
    commands::run().await;
}
