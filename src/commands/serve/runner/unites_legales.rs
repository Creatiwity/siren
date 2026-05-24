use super::common::{Context, UniteLegaleInnerResponse, UniteLegaleResponse};
use super::error::Error;
use crate::models;
use crate::models::unite_legale::common::{
    UniteLegaleSearchParams, UniteLegaleSearchResponse, UniteLegaleSearchResultResponse,
    UniteLegaleSortField,
};
use crate::models::unite_legale::error::Error as UniteLegaleModelError;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use std::sync::Arc;
use tracing::Span;
use utoipa_axum::{router::OpenApiRouter, routes};

/// Get unit legale by SIREN
#[utoipa::path(
    get,
    path = "/{siren}",
    params(
        ("siren" = String, Path, description = "SIREN number")
    ),
    responses(
        (status = 200, description = "UniteLegale response", body = UniteLegaleResponse),
        (status = 400, description = "Invalid SIREN"),
        (status = 301, description = "Redirect to canonical SIREN"),
        (status = 404, description = "UniteLegale not found")
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn get_unite_legale_by_siren(
    State(context): State<Arc<Context>>,
    Path(siren): Path<String>,
) -> Result<Json<UniteLegaleResponse>, Error> {
    if siren.len() != 9 {
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

        let unite_legale = match models::unite_legale::get(&mut connection, &siren) {
            Ok(u) => u,
            Err(UniteLegaleModelError::UniteLegaleNotFound) => {
                match models::siren_doublon::find_canonical_siren(&mut connection, &siren) {
                    Ok(Some(canonical_siren)) => {
                        return Err(Error::SirenDoublonRedirect {
                            location: format!("/v3/unites_legales/{}", canonical_siren),
                        });
                    }
                    Ok(None) => {}
                    Err(e) => return Err(Error::SirenDoublonLookup { source: e }),
                }
                return Err(Error::UniteLegale {
                    source: UniteLegaleModelError::UniteLegaleNotFound,
                });
            }
            Err(e) => return Err(Error::UniteLegale { source: e }),
        };
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
    })
    .await
    .unwrap_or_else(|_| Err(Error::BlockingTaskPanicked))
}

/// Search legal units
#[utoipa::path(
    get,
    path = "/",
    params(UniteLegaleSearchParams),
    responses(
        (status = 200, description = "Search results", body = UniteLegaleSearchResponse),
        (status = 400, description = "Invalid search parameters")
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn search_unites_legales(
    State(context): State<Arc<Context>>,
    Query(params): Query<UniteLegaleSearchParams>,
) -> Result<Json<UniteLegaleSearchResponse>, Error> {
    match params.sort {
        Some(UniteLegaleSortField::Relevance) if params.q.is_none() => {
            return Err(Error::InvalidSearchParams {
                message: "sort=relevance requires a q parameter".to_string(),
            });
        }
        _ => {}
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

        let output = models::unite_legale::search(&mut connection, &params)?;
        let total = output.total;

        Ok(Json(UniteLegaleSearchResponse {
            unites_legales: output
                .results
                .into_iter()
                .map(|r| UniteLegaleSearchResultResponse {
                    siren: r.siren,
                    etat_administratif: r.etat_administratif,
                    date_creation: r.date_creation,
                    denomination: r.denomination,
                    denomination_usuelle_1: r.denomination_usuelle_1,
                    denomination_usuelle_2: r.denomination_usuelle_2,
                    denomination_usuelle_3: r.denomination_usuelle_3,
                    activite_principale: r.activite_principale,
                    categorie_juridique: r.categorie_juridique,
                    categorie_entreprise: r.categorie_entreprise,
                    score: r.score,
                })
                .collect(),
            total,
            limit: output.limit,
            offset: output.offset,
            sort: output.sort,
            direction: output.direction,
        }))
    })
    .await
    .unwrap_or_else(|_| Err(Error::BlockingTaskPanicked))
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new()
        .routes(routes!(get_unite_legale_by_siren))
        .routes(routes!(search_unites_legales))
}
