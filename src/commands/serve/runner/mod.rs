mod admin;
mod error;
mod etablissements;
mod liens_succession;
mod root;
mod trace;
mod unites_legales;

pub mod common;

use axum::http::{Method, header};
use axum::middleware;
use common::Context;
use sentry::integrations::tower::{NewSentryLayer, SentryHttpLayer};
use std::net::SocketAddr;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_scalar::{Scalar, Servable};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Siren API",
        description = "API providing information about all companies in France",
        contact(name = "Julien Blatecky", email = "contact@creatiwity.net"),
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        version = "5.1.1"
    ),
    tags(
        (name = common::PUBLIC_TAG, description = "Public endpoint"),
        (name = common::ADMIN_TAG, description = "Admin endpoints")
    )
)]
struct ApiDoc;

pub async fn run(addr: SocketAddr, context: Context) {
    let shared_context = Arc::new(context);

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/admin", admin::router())
        .nest("/v3/etablissements", etablissements::router())
        .nest(
            "/v3/etablissements/liens_succession",
            liens_succession::router(),
        )
        .nest("/v3/unites_legales", unites_legales::router())
        .merge(root::router())
        .split_for_parts();

    let app = router
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::CONTENT_TYPE])
                .allow_origin(tower_http::cors::Any),
        )
        .merge(Scalar::with_url("/scalar", api))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(SentryHttpLayer::new().enable_transaction())
        .layer(middleware::from_fn(trace::siren_context_middleware))
        .layer(NewSentryLayer::new_from_top())
        .layer(middleware::from_fn(trace::traceparent_middleware))
        .with_state(shared_context);

    axum::serve(
        tokio::net::TcpListener::bind(&addr).await.unwrap(),
        app.into_make_service(),
    )
    .await
    .unwrap();
}
