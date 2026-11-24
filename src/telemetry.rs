//! Telemetry outputs:
//!
//! - Sentry (`SENTRY_DSN`): errors, transactions, Crons.
//! - OpenTelemetry (`OTEL_EXPORTER_OTLP_ENDPOINT`, plus the standard `OTEL_*`
//!   variables): the spans and the logs, exported over OTLP/HTTP. A log
//!   carries the trace and span it was written in. `OTEL_LOGS_EXPORTER=none`
//!   keeps the logs out of it.
//! - stdout (`LOG_FORMAT`): `text` (default), `json`, or `none` when the OTLP
//!   export is enough. Keep it on in Kubernetes: `kubectl logs` reads it, and
//!   it still works when the collector does not.
//!
//! `RUST_LOG` filters all of them.

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::process;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::filter::{EnvFilter, filter_fn};
use tracing_subscriber::{Layer, prelude::*};

/// The stdout output of the logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Text,
    Json,
    None,
}

impl LogFormat {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.map(str::trim) {
            None | Some("") | Some("text") => Ok(Self::Text),
            Some("json") => Ok(Self::Json),
            Some("none") => Ok(Self::None),
            Some(other) => Err(format!(
                "LOG_FORMAT must be text, json or none, not {other:?}: text used"
            )),
        }
    }
}

struct Otlp {
    traces: SdkTracerProvider,
    logs: Option<SdkLoggerProvider>,
}

/// Kept for [`exit`] and [`shutdown`], which cannot reach the providers
/// otherwise.
static OTLP: OnceLock<Otlp> = OnceLock::new();

/// Sets the global subscriber: stdout, Sentry, and OTLP when an endpoint is
/// configured. Needs a Tokio runtime (batch export).
pub fn init() {
    let format = LogFormat::parse(std::env::var("LOG_FORMAT").ok().as_deref());
    let otlp = init_otlp();
    let traces = otlp.map(|o| tracing_opentelemetry::layer().with_tracer(o.traces.tracer("siren")));
    let logs = otlp.and_then(|o| o.logs.as_ref()).map(|provider| {
        OpenTelemetryTracingBridge::new(provider)
            .with_filter(filter_fn(|meta| !is_exporter_target(meta.target())))
    });
    let stdout = match format.clone().unwrap_or(LogFormat::Text) {
        LogFormat::Text => Some(tracing_subscriber::fmt::layer().boxed()),
        LogFormat::Json => Some(tracing_subscriber::fmt::layer().json().boxed()),
        LogFormat::None => None,
    };
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(stdout)
        .with(sentry::integrations::tracing::layer())
        .with(traces)
        .with(logs)
        .init();

    if let Err(e) = &format {
        warn!("{e}");
    }
    let on = |enabled: bool| if enabled { "on" } else { "off" };
    info!(
        "siren {}: Sentry {}, OTLP traces {}, OTLP logs {}, stdout {:?}",
        env!("CARGO_PKG_VERSION"),
        on(sentry::Hub::current()
            .client()
            .is_some_and(|c| c.is_enabled())),
        on(otlp.is_some()),
        on(otlp.is_some_and(|o| o.logs.is_some())),
        format.unwrap_or(LogFormat::Text),
    );
}

/// The crates the exporters use: their own logs, exported, would feed the
/// exporter forever.
fn is_exporter_target(target: &str) -> bool {
    ["opentelemetry", "hyper", "h2", "reqwest", "rustls"]
        .iter()
        .any(|prefix| target.starts_with(prefix))
}

/// The OTLP exporters, only when an endpoint is configured. Run before the
/// subscriber exists: a failure goes to stderr, not to `tracing`.
fn init_otlp() -> Option<&'static Otlp> {
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok()?;
    global::set_text_map_propagator(TraceContextPropagator::new());
    let exporter = match span_exporter() {
        Ok(exporter) => exporter,
        Err(e) => {
            eprintln!("OTLP export disabled: {e}");
            return None;
        }
    };
    let traces = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();
    global::set_tracer_provider(traces.clone());

    let logs = if std::env::var("OTEL_LOGS_EXPORTER").is_ok_and(|v| v.trim() == "none") {
        None
    } else {
        match log_exporter() {
            Ok(exporter) => Some(
                SdkLoggerProvider::builder()
                    .with_batch_exporter(exporter)
                    .build(),
            ),
            Err(e) => {
                eprintln!("OTLP log export disabled: {e}");
                None
            }
        }
    };
    Some(OTLP.get_or_init(|| Otlp { traces, logs }))
}

fn span_exporter()
-> Result<opentelemetry_otlp::SpanExporter, opentelemetry_otlp::ExporterBuildError> {
    opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .build()
}

fn log_exporter() -> Result<opentelemetry_otlp::LogExporter, opentelemetry_otlp::ExporterBuildError>
{
    opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .build()
}

/// Sends what the OTLP exporters still buffer.
pub fn shutdown() {
    if let Some(otlp) = OTLP.get() {
        // Spans first: their shutdown may still log.
        if let Err(e) = otlp.traces.shutdown() {
            warn!("OTLP shutdown error: {e}");
        }
        if let Some(logs) = &otlp.logs
            && let Err(e) = logs.shutdown()
        {
            eprintln!("OTLP log shutdown error: {e}");
        }
    }
}

/// Exits the process after sending what is still buffered.
///
/// `process::exit` runs no destructor: the Sentry guard held by `main` would
/// never flush, and an error captured just before exiting would be lost.
pub fn exit(code: i32) -> ! {
    if let Some(client) = sentry::Hub::current().client() {
        client.flush(Some(FLUSH_TIMEOUT));
    }
    shutdown();
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
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    use opentelemetry::trace::{TraceContextExt, Tracer};
    use opentelemetry_otlp::WithExportConfig;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    use super::*;

    /// One OTLP/HTTP request on a local endpoint: its request line and body.
    fn receive_one(listener: TcpListener) -> (String, Vec<u8>) {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        reader
            .into_inner()
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
            .unwrap();
        (request_line, body)
    }

    #[test]
    fn log_format() {
        assert_eq!(LogFormat::parse(None), Ok(LogFormat::Text));
        assert_eq!(LogFormat::parse(Some(" json ")), Ok(LogFormat::Json));
        assert_eq!(LogFormat::parse(Some("none")), Ok(LogFormat::None));
        assert!(LogFormat::parse(Some("yaml")).is_err());
    }

    #[test]
    fn the_exporters_own_logs_are_not_exported() {
        assert!(is_exporter_target("hyper_util::client::legacy"));
        assert!(is_exporter_target("opentelemetry_sdk"));
        assert!(!is_exporter_target("sirene::update"));
    }

    #[test]
    fn the_log_exporter_builds() {
        log_exporter().expect("opentelemetry-otlp needs an HTTP client feature");
    }

    /// A log reaches an OTLP/HTTP endpoint with the trace of the span it
    /// was written in.
    #[test]
    fn logs_are_exported_with_their_trace() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1/logs", listener.local_addr().unwrap());
        let received = std::thread::spawn(move || receive_one(listener));

        let exporter = opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build()
            .unwrap();
        let logs = SdkLoggerProvider::builder()
            .with_batch_exporter(exporter)
            .build();
        let traces = SdkTracerProvider::builder().build();
        let subscriber = tracing_subscriber::registry()
            .with(tracing_opentelemetry::layer().with_tracer(traces.tracer("test")))
            .with(OpenTelemetryTracingBridge::new(&logs));

        let trace_id = tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("update");
            let _entered = span.enter();
            info!("stock up to date");
            span.context().span().span_context().trace_id()
        });
        logs.force_flush().unwrap();

        let (request_line, body) = received.join().unwrap();
        assert!(request_line.starts_with("POST /v1/logs "), "{request_line}");
        assert_ne!(trace_id, opentelemetry::trace::TraceId::INVALID);
        assert!(body.windows(16).any(|w| w == trace_id.to_bytes()));
        assert!(body.windows(16).any(|w| w == b"stock up to date"));
        logs.shutdown().unwrap();
    }

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
