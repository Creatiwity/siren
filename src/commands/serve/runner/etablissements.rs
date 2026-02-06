use super::common::{
    Context, EtablissementInnerResponse, EtablissementResponse,
    UniteLegaleEtablissementInnerResponse,
};
use super::error::Error;
use crate::models;
use crate::models::etablissement::common::{
    EtablissementSearchParams, EtablissementSearchResponse, EtablissementSearchResultResponse,
    EtablissementSortField,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use std::sync::Arc;
use tracing::{Level, span};
use utoipa_axum::{router::OpenApiRouter, routes};

/// Get establishment by SIRET
#[utoipa::path(
    get,
    path = "/{siret}",
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

/// Search establishments
#[utoipa::path(
    get,
    path = "/",
    params(EtablissementSearchParams),
    responses(
        (status = 200, description = "Search results", body = EtablissementSearchResponse),
        (status = 400, description = "Invalid search parameters")
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn search_etablissements(
    State(context): State<Arc<Context>>,
    Query(params): Query<EtablissementSearchParams>,
) -> Result<Json<EtablissementSearchResponse>, Error> {
    let span = span!(Level::TRACE, "GET /etablissements (search)");
    let _enter = span.enter();

    // Validate geographic params: all-or-none
    let has_any_geo = params.lat.is_some() || params.lng.is_some() || params.radius.is_some();
    let has_all_geo = params.lat.is_some() && params.lng.is_some() && params.radius.is_some();
    if has_any_geo && !has_all_geo {
        return Err(Error::InvalidSearchParams {
            message: "lat, lng, and radius must all be provided together".to_string(),
        });
    }

    // Validate sort constraints (parse field name before ':' direction suffix)
    match params.sort {
        Some(EtablissementSortField::Distance) if !has_all_geo => {
            return Err(Error::InvalidSearchParams {
                message: "sort=distance requires lat, lng, and radius parameters".to_string(),
            });
        }
        Some(EtablissementSortField::Relevance) if params.q.is_none() => {
            return Err(Error::InvalidSearchParams {
                message: "sort=relevance requires a q parameter".to_string(),
            });
        }
        _ => {}
    }

    let connectors = context.builders.create();
    let mut connection = connectors
        .local
        .pool
        .get()
        .map_err(|e| Error::LocalConnectionFailed { source: e })?;

    let output = models::etablissement::search(&mut connection, &params)?;

    let total = output.results.first().map(|r| r.total).unwrap_or(0);

    Ok(Json(EtablissementSearchResponse {
        etablissements: output
            .results
            .into_iter()
            .map(|r| EtablissementSearchResultResponse {
                siret: r.siret,
                siren: r.siren,
                etat_administratif: r.etat_administratif,
                date_creation: r.date_creation,
                denomination_usuelle: r.denomination_usuelle,
                enseigne_1: r.enseigne_1,
                enseigne_2: r.enseigne_2,
                enseigne_3: r.enseigne_3,
                code_postal: r.code_postal,
                libelle_commune: r.libelle_commune,
                activite_principale: r.activite_principale,
                etablissement_siege: r.etablissement_siege,
                meter_distance: r.meter_distance,
                score: r.score,
            })
            .collect(),
        total,
        limit: output.limit,
        offset: output.offset,
        sort: output.sort,
        direction: output.direction,
    }))
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new()
        .routes(routes!(get_etablissement_by_siret))
        .routes(routes!(search_etablissements))
}
