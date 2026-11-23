use opentelemetry::global;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{self as sdktrace, SdkTracerProvider},
};
use std::process;
use std::sync::OnceLock;
use std::time::Duration;

pub fn init_otlp() -> Option<SdkTracerProvider> {
    // Only init if an endpoint is explicitly configured
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;

    global::set_text_map_propagator(TraceContextPropagator::new());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .build()
        .ok()?;

    let provider = sdktrace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(provider.clone());
    let _ = PROVIDER.set(provider.clone());

    Some(provider)
}

/// Kept for `exit`, which cannot reach the provider owned by `main`.
static PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

/// Exits the process after sending what is still buffered.
///
/// `process::exit` runs no destructor: the Sentry guard held by `main` would
/// never flush, and an error captured just before exiting would be lost.
pub fn exit(code: i32) -> ! {
    if let Some(client) = sentry::Hub::current().client() {
        client.flush(Some(FLUSH_TIMEOUT));
    }

    if let Some(provider) = PROVIDER.get()
        && let Err(e) = provider.force_flush()
    {
        tracing::warn!("OTLP flush error: {e}");
    }

    process::exit(code)
}

const FLUSH_TIMEOUT: Duration = Duration::from_secs(2);

/// `spawn_blocking` keeping the caller's span and Sentry hub: without them,
/// the spans opened in the closure (geocoder-core's `geocoder.search`,
/// `geocoder.open`) start transactions of their own instead of nesting in
/// the request's or the job's.
#[cfg(feature = "geocoding")]
pub fn spawn_blocking<F, R>(f: F) -> tokio::task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let span = tracing::Span::current();
    let hub = sentry::Hub::current();
    tokio::task::spawn_blocking(move || sentry::Hub::run(hub, || span.in_scope(f)))
}
