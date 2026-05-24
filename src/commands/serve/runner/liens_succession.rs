use super::common::{Context, LiensSuccessionResponse};
use super::error::Error;
use crate::models;
use axum::{
    Json,
    extract::{Path, State},
};
use std::sync::Arc;
use tracing::Span;
use utoipa_axum::{router::OpenApiRouter, routes};

/// Get liens successions by SIRET
#[utoipa::path(
    get,
    path = "/{siret}",
    params(
        ("siret" = String, Path, description = "SIRET number")
    ),
    responses(
        (status = 200, description = "LiensSuccession response", body = LiensSuccessionResponse),
        (status = 400, description = "Invalid SIRET")
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn get_liens_succession_by_siret(
    State(context): State<Arc<Context>>,
    Path(siret): Path<String>,
) -> Result<Json<LiensSuccessionResponse>, Error> {
    if siret.len() != 14 {
        return Err(Error::InvalidData);
    }

    let current_span = Span::current();
    tokio::task::spawn_blocking(move || {
        let _enter = current_span.enter();

        let connectors = context.builders.create();
        let mut connection = connectors
            .local
            .pool
            .get()
            .map_err(|e| Error::LocalConnectionFailed { source: e })?;

        let liens_succession = models::lien_succession::get(&mut connection, &siret)?;

        Ok(Json(LiensSuccessionResponse { liens_succession }))
    })
    .await
    .unwrap_or_else(|_| Err(Error::BlockingTaskPanicked))
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new().routes(routes!(get_liens_succession_by_siret))
}
