#![recursion_limit = "256"]

mod commands;
mod connectors;
mod diesel_instrumentation;
mod models;
mod sentry_crons;
mod telemetry;
mod update;

use connectors::ConnectorsBuilders;
use dotenv::dotenv;
use opentelemetry::trace::TracerProvider as _;
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

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Cannot build Tokio runtime")
        .block_on(async {
            // Init OTLP before the subscriber so the tracer is ready when layers are built
            let otlp_provider = telemetry::init_otlp();

            let otel_layer = otlp_provider
                .as_ref()
                .map(|p| tracing_opentelemetry::layer().with_tracer(p.tracer("siren")));

            tracing_subscriber::registry()
                .with(EnvFilter::from_default_env())
                .with(tracing_subscriber::fmt::layer())
                .with(sentry::integrations::tracing::layer())
                .with(otel_layer)
                .init();

            // Futures should to be bound to a Hub
            // Learn more at https://docs.rs/sentry-core/latest/sentry_core/#parallelism-concurrency-and-async
            launch().bind_hub(sentry::Hub::current()).await;

            if let Some(provider) = otlp_provider
                && let Err(e) = provider.shutdown()
            {
                tracing::warn!("OTLP shutdown error: {e}");
            }
        });
}

#[tracing::instrument]
async fn launch() {
    // Load database
    let connectors_builders = ConnectorsBuilders::new();

    // Run command
    commands::run(connectors_builders).await;
}
