mod admin;
mod error;
mod etablissements;
mod health;
mod liens_succession;
mod root;
mod trace;
mod unites_legales;

pub mod common;

use axum::body::Bytes;
use axum::http::{Method, header};
use axum::routing::get;
use axum::{Router, middleware};
use common::Context;
use sentry::integrations::tower::{NewSentryLayer, SentryHttpLayer};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::info;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Siren API",
        description = "API providing information about all companies in France",
        contact(name = "Julien Blatecky", email = "contact@creatiwity.net"),
        license(name = "MIT", url = "https://opensource.org/licenses/MIT")
    ),
    tags(
        (name = common::PUBLIC_TAG, description = "Public endpoint"),
        (name = common::ADMIN_TAG, description = "Admin endpoints")
    )
)]
struct ApiDoc;

pub async fn run(addr: SocketAddr, context: Context, shutdown_delay: Duration) {
    let shutting_down = context.shutting_down.clone();
    let shared_context = Arc::new(context);

    let (health_router, health_api) = health::router().split_for_parts();

    let (router, mut api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/admin", admin::router())
        .nest("/v3/etablissements", etablissements::router())
        .nest(
            "/v3/etablissements/liens_succession",
            liens_succession::router(),
        )
        .nest("/v3/unites_legales", unites_legales::router())
        .merge(root::router())
        .split_for_parts();
    api.merge(health_api);

    // Serialized once: the spec never changes after startup.
    let spec = Bytes::from(api.to_json().expect("Unable to serialize the OpenAPI spec"));
    let openapi_router = Router::new()
        .route(
            "/openapi.json",
            get(move || async move { ([(header::CONTENT_TYPE, "application/json")], spec) }),
        )
        .layer(cors_layer())
        .layer(tower_http::compression::CompressionLayer::new());

    let app = router
        .layer(cors_layer())
        .merge(Scalar::with_url("/scalar", api))
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(SentryHttpLayer::new().enable_transaction())
        .layer(middleware::from_fn(trace::siren_context_middleware))
        .layer(NewSentryLayer::new_from_top())
        .layer(middleware::from_fn(trace::traceparent_middleware))
        // Merged after the layers on purpose: probes and the static spec stay
        // out of Sentry transactions and request traces.
        .merge(health_router)
        .merge(openapi_router)
        .with_state(shared_context);

    axum::serve(
        tokio::net::TcpListener::bind(&addr).await.unwrap(),
        app.into_make_service(),
    )
    .with_graceful_shutdown(shutdown_signal(shutting_down, shutdown_delay))
    .await
    .unwrap();
}

fn cors_layer() -> tower_http::cors::CorsLayer {
    tower_http::cors::CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE])
        .allow_origin(tower_http::cors::Any)
}

/// Resolves when the server should stop accepting connections.
///
/// On SIGTERM (the orchestrator), readiness starts failing and the server keeps
/// serving for `delay`, the time for the pod to leave the load balancer; then
/// in-flight requests are drained. Ctrl+C stops right away.
async fn shutdown_signal(shutting_down: Arc<AtomicBool>, delay: Duration) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Unable to listen for Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Unable to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Ctrl+C received, shutting down");
            shutting_down.store(true, Ordering::Relaxed);
        }
        _ = terminate => {
            info!("SIGTERM received, failing readiness for {:?} before shutting down", delay);
            shutting_down.store(true, Ordering::Relaxed);
            tokio::time::sleep(delay).await;
        }
    }
}
