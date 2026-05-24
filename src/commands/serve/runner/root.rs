use super::common::{Context, MetadataResponse};
use super::error::Error;
use crate::models;
use axum::{Json, extract::State};
use std::sync::Arc;
use tracing::Span;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

/// Health check
#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Metadata response", body = MetadataResponse)
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn get_health_check_status(
    State(context): State<Arc<Context>>,
) -> Result<Json<MetadataResponse>, Error> {
    let current_span = Span::current();
    tokio::task::spawn_blocking(move || {
        let _enter = current_span.enter();

        let connectors = context.builders.create();
        let update_metadata = models::update_metadata::last_success_update(&connectors)?;

        let reply = match update_metadata {
            Some(metadata) => MetadataResponse {
                launched_timestamp: Some(metadata.launched_timestamp),
                finished_timestamp: metadata.finished_timestamp,
            },
            None => MetadataResponse {
                launched_timestamp: None,
                finished_timestamp: None,
            },
        };

        Ok(Json(reply))
    })
    .await
    .unwrap_or_else(|_| Err(Error::BlockingTaskPanicked))
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new().routes(routes!(get_health_check_status))
}
