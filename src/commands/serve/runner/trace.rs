use axum::http::{HeaderName, HeaderValue};
use axum::{extract::Request, middleware::Next, response::Response};
use sentry::protocol::{SpanId, TraceId, User};

pub async fn traceparent_middleware(mut request: Request, next: Next) -> Response {
    let incoming = request
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_traceparent);

    let (trace_id, parent_span_id, flags) = match incoming {
        Some((t, p, f)) => (t, Some(p), f),
        None => (TraceId::default(), None, 1u8),
    };

    let our_span_id = SpanId::default();

    // Propagate into sentry-trace so SentryHttpLayer links this request to the incoming trace
    let sampled = if flags & 0x01 != 0 { "1" } else { "0" };
    let sentry_trace = match parent_span_id {
        Some(ref p) => format!("{trace_id}-{p}-{sampled}"),
        None => format!("{trace_id}-{our_span_id}-{sampled}"),
    };
    if let Ok(value) = HeaderValue::from_str(&sentry_trace) {
        request
            .headers_mut()
            .insert(HeaderName::from_static("sentry-trace"), value);
    }

    let mut response = next.run(request).await;

    let traceparent = format!("00-{trace_id}-{our_span_id}-{flags:02x}");
    if let Ok(value) = HeaderValue::from_str(&traceparent) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("traceparent"), value);
    }

    response
}

pub async fn siren_context_middleware(request: Request, next: Next) -> Response {
    let client_id = request
        .headers()
        .get("x-siren-client-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    let tier = request
        .headers()
        .get("x-siren-tier")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    if client_id.is_some() || tier.is_some() {
        sentry::configure_scope(|scope| {
            if let Some(ref id) = client_id {
                scope.set_user(Some(User {
                    id: Some(id.clone()),
                    ..Default::default()
                }));
            }
            if let Some(ref t) = tier {
                scope.set_tag("siren.tier", t);
            }
        });
    }

    next.run(request).await
}

fn parse_traceparent(header: &str) -> Option<(TraceId, SpanId, u8)> {
    let mut parts = header.splitn(4, '-');
    if parts.next()? != "00" {
        return None;
    }
    let trace_id = parts.next()?.parse().ok()?;
    let span_id = parts.next()?.parse().ok()?;
    let flags = u8::from_str_radix(parts.next()?, 16).ok()?;
    Some((trace_id, span_id, flags))
}
