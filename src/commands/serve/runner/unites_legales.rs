use super::common::{Context, UniteLegaleInnerResponse, UniteLegaleResponse};
use super::error::Error;
use crate::models;
use axum::{
    Json,
    extract::{Path, State},
};
use std::sync::Arc;
use tracing::{Level, span};
use utoipa_axum::{router::OpenApiRouter, routes};

/// Get unit legale by SIREN
#[utoipa::path(
    get,
    path = "/v3/unites_legales/{siren}",
    params(
        ("siren" = String, Path, description = "SIREN number")
    ),
    responses(
        (status = 200, description = "UniteLegale response", body = UniteLegaleResponse),
        (status = 400, description = "Invalid SIREN"),
        (status = 404, description = "UniteLegale not found")
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn get_unite_legale_by_siren(
    State(context): State<Arc<Context>>,
    Path(siren): Path<String>,
) -> Result<Json<UniteLegaleResponse>, Error> {
    let span = span!(Level::TRACE, "GET /unites_legales");
    let _enter = span.enter();

    if siren.len() != 9 {
        return Err(Error::InvalidData);
    }

    let connectors = context.builders.create();
    let mut connection = connectors
        .local
        .pool
        .get()
        .map_err(|e| Error::LocalConnectionFailed { source: e })?;

    let unite_legale = models::unite_legale::get(&mut connection, &siren)?;
    let etablissements = models::etablissement::get_with_siren(&mut connection, &siren)?;
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&mut connection, &unite_legale.siren)?;

    Ok(Json(UniteLegaleResponse {
        unite_legale: UniteLegaleInnerResponse {
            unite_legale,
            etablissements,
            etablissement_siege,
        },
    }))
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new().routes(routes!(get_unite_legale_by_siren))
}
