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

    // Before the subscriber exists: a failure goes to stderr.
    let exporter = match span_exporter() {
        Ok(exporter) => exporter,
        Err(e) => {
            eprintln!("OTLP export disabled: {e}");
            return None;
        }
    };

    let provider = sdktrace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(provider.clone());
    let _ = PROVIDER.set(provider.clone());

    Some(provider)
}

fn span_exporter()
-> Result<opentelemetry_otlp::SpanExporter, opentelemetry_otlp::ExporterBuildError> {
    opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .build()
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

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use opentelemetry::trace::{Tracer, TracerProvider as _};
    use opentelemetry_otlp::WithExportConfig;

    use super::*;

    /// Without an HTTP client feature the exporter does not build, and
    /// `init_otlp` would export nothing.
    #[test]
    fn the_span_exporter_builds() {
        span_exporter().expect("opentelemetry-otlp needs an HTTP client feature");
    }

    /// A span reaches an OTLP/HTTP endpoint through the batch processor.
    #[test]
    fn spans_are_exported() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1/traces", listener.local_addr().unwrap());
        let received = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let n = stream.read(&mut buf).unwrap();
            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
                .unwrap();
            String::from_utf8_lossy(&buf[..n]).into_owned()
        });

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build()
            .unwrap();
        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();
        provider.tracer("test").in_span("siren.test", |_| {});
        provider.force_flush().unwrap();

        let request = received.join().unwrap();
        assert!(request.starts_with("POST /v1/traces "), "{request}");
        provider.shutdown().unwrap();
    }
}
