use opentelemetry::global;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{self as sdktrace, SdkTracerProvider},
};

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

    Some(provider)
}
