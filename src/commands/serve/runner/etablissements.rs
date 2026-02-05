use super::common::{
    Context, EtablissementInnerResponse, EtablissementResponse,
    UniteLegaleEtablissementInnerResponse,
};
use super::error::Error;
use crate::models;
use axum::{
    Json,
    extract::{Path, State},
};
use std::sync::Arc;
use tracing::{Level, span};
use utoipa_axum::{router::OpenApiRouter, routes};

/// Get establishment by SIRET
#[utoipa::path(
    get,
    path = "/v3/etablissements/{siret}",
    params(
        ("siret" = String, Path, description = "SIRET number")
    ),
    responses(
        (status = 200, description = "Etablissement response", body = EtablissementResponse),
        (status = 400, description = "Invalid SIRET"),
        (status = 404, description = "Etablissement not found")
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn get_etablissement_by_siret(
    State(context): State<Arc<Context>>,
    Path(siret): Path<String>,
) -> Result<Json<EtablissementResponse>, Error> {
    let span = span!(Level::TRACE, "GET /etablissements");
    let _enter = span.enter();

    if siret.len() != 14 {
        return Err(Error::InvalidData);
    }

    let connectors = context.builders.create();
    let mut connection = connectors
        .local
        .pool
        .get()
        .map_err(|e| Error::LocalConnectionFailed { source: e })?;

    let etablissement = models::etablissement::get(&mut connection, &siret)?;
    let unite_legale = models::unite_legale::get(&mut connection, &etablissement.siren)?;
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&mut connection, &etablissement.siren)?;

    Ok(Json(EtablissementResponse {
        etablissement: EtablissementInnerResponse {
            etablissement,
            unite_legale: UniteLegaleEtablissementInnerResponse {
                unite_legale,
                etablissement_siege,
            },
        },
    }))
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new().routes(routes!(get_etablissement_by_siret))
}
