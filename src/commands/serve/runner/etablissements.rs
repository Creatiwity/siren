use super::common::{
    Context, EtablissementInnerResponse, EtablissementResponse,
    UniteLegaleEtablissementInnerResponse,
};
use super::error::Error;
use crate::models;
use crate::models::etablissement::common::{
    EtablissementSearchOutput, EtablissementSearchParams, EtablissementSearchResponse,
    EtablissementSearchResultResponse, EtablissementSortField,
};
#[cfg(feature = "geocoding")]
use crate::models::etablissement::common::{DEFAULT_GEOCODING_MIN_SCORE, DEFAULT_GEOCODING_RADIUS};
use crate::models::etablissement::error::Error as EtablissementModelError;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use std::sync::Arc;
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
        (status = 301, description = "Redirect to siege of canonical SIREN"),
        (status = 404, description = "Etablissement not found")
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn get_etablissement_by_siret(
    State(context): State<Arc<Context>>,
    Path(siret): Path<String>,
) -> Result<Json<EtablissementResponse>, Error> {
    if siret.len() != 14 {
        return Err(Error::InvalidData);
    }

    let connectors = context.builders.create();
    let mut connection = connectors.local.pool.get().await?;

    let etablissement = match models::etablissement::get(&mut connection, &siret).await {
        Ok(e) => e,
        Err(EtablissementModelError::EtablissementNotFound) => {
            let siren = &siret.as_str()[..9];
            match models::siren_doublon::find_canonical_siren(&mut connection, siren).await {
                Ok(Some(canonical_siren)) => {
                    let siege = models::etablissement::get_siege_with_siren(
                        &mut connection,
                        &canonical_siren,
                    )
                    .await?;
                    return Err(Error::SirenDoublonRedirect {
                        location: format!("/v3/etablissements/{}", siege.siret),
                    });
                }
                Ok(None) => {}
                Err(e) => return Err(Error::SirenDoublonLookup { source: e }),
            }
            return Err(Error::Etablissement {
                source: EtablissementModelError::EtablissementNotFound,
            });
        }
        Err(e) => return Err(Error::Etablissement { source: e }),
    };
    let unite_legale = models::unite_legale::get(&mut connection, &etablissement.siren).await?;
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&mut connection, &etablissement.siren).await?;

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
    if params.address.is_some() && (params.lat.is_some() || params.lng.is_some()) {
        return Err(Error::InvalidSearchParams {
            message: "address and lat/lng are mutually exclusive".to_string(),
        });
    }

    // Resolve address → lat/lng before geo validation
    let params = resolve_geocoding(params, &context)?;

    let has_any_geo = params.lat.is_some() || params.lng.is_some() || params.radius.is_some();
    let has_all_geo = params.lat.is_some() && params.lng.is_some() && params.radius.is_some();
    if has_any_geo && !has_all_geo {
        return Err(Error::InvalidSearchParams {
            message: "lat, lng, and radius must all be provided together".to_string(),
        });
    }

    // The same three rules are stated once in `models::search`; the model layer
    // degrades gracefully instead, the HTTP layer prefers to say so.
    for check in [
        models::search::check_query_length(params.q.as_deref()),
        models::search::check_facets(
            params.facette.as_deref(),
            models::search::ETABLISSEMENT_FACET_FIELDS,
        ),
        models::search::check_cursor(
            params.cursor.as_deref(),
            matches!(params.sort, Some(EtablissementSortField::Siret)),
            "siret",
            params.offset,
        ),
    ] {
        check.map_err(|message| Error::InvalidSearchParams { message })?;
    }

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
    let mut connection = connectors.local.pool.get().await?;

    let output = models::etablissement::search(&mut connection, &params).await?;
    let EtablissementSearchOutput {
        results,
        total,
        total_capped,
        limit,
        offset,
        sort,
        direction,
        suggestion,
        facettes,
        next_cursor,
    } = output;

    Ok(Json(EtablissementSearchResponse {
        etablissements: results
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
                position: r.position,
                meter_distance: r.meter_distance,
                score: r.score,
            })
            .collect(),
        total,
        total_capped,
        limit,
        offset,
        sort,
        direction,
        suggestion,
        facettes,
        next_cursor,
    }))
}

/// If `address` is set, geocode it and fill in lat/lng/radius.
/// Returns the params unchanged when address is absent.
fn resolve_geocoding(
    params: EtablissementSearchParams,
    context: &Context,
) -> Result<EtablissementSearchParams, Error> {
    let Some(address) = params.address.clone() else {
        return Ok(params);
    };

    #[cfg(not(feature = "geocoding"))]
    {
        let _ = (address, context);
        return Err(Error::InvalidSearchParams {
            message: "geocoding support is not compiled in this build".to_string(),
        });
    }

    #[cfg(feature = "geocoding")]
    {
        let mut params = params;
        let Some(ref geocoder) = context.geocoder else {
            return Err(Error::InvalidSearchParams {
                message: "geocoding index not loaded — start the server with --geocoding-index-path"
                    .to_string(),
            });
        };

        let min_score = params.geocoding_min_score.unwrap_or(DEFAULT_GEOCODING_MIN_SCORE);
        let results = geocoder.search(&address, geocoder_core::SearchOpts::default());
        let best = results.into_iter().find(|r| r.score >= min_score);

        match best {
            None => {
                // No result above threshold → radius=0 ensures empty DB results
                params.lat = Some(0.0);
                params.lng = Some(0.0);
                params.radius = Some(0.0);
            }
            Some(result) => {
                let lat = result.doc.get("lat").and_then(|v| v.as_f64());
                let lon = result.doc.get("lon").and_then(|v| v.as_f64());
                match (lat, lon) {
                    (Some(lat), Some(lon)) => {
                        params.lat = Some(lat);
                        params.lng = Some(lon);
                        params.radius = Some(params.radius.unwrap_or(DEFAULT_GEOCODING_RADIUS));
                    }
                    _ => {
                        return Err(Error::InvalidSearchParams {
                            message: "geocoding result has no coordinates".to_string(),
                        });
                    }
                }
            }
        }

        Ok(params)
    }
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new()
        .routes(routes!(get_etablissement_by_siret))
        .routes(routes!(search_etablissements))
}
