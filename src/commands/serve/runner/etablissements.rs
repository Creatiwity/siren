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

    // Resolve address → lat/lng/radius or code_commune before geo validation
    #[cfg_attr(not(feature = "geocoding"), allow(unused_variables))]
    let (params, adresse) = resolve_address(params, &context).await?;

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
        #[cfg(feature = "geocoding")]
        adresse,
    }))
}

#[cfg(not(feature = "geocoding"))]
async fn resolve_address(
    params: EtablissementSearchParams,
    _context: &Context,
) -> Result<(EtablissementSearchParams, ()), Error> {
    if params.address.is_some() {
        return Err(Error::GeocodingDisabled);
    }
    Ok((params, ()))
}

/// If `address` is set, geocode it and narrow the search accordingly (see
/// `geocoding::address::filter_for`). The chosen address is returned for the
/// response.
#[cfg(feature = "geocoding")]
async fn resolve_address(
    mut params: EtablissementSearchParams,
    context: &Context,
) -> Result<
    (
        EtablissementSearchParams,
        Option<crate::geocoding::address::AddressMatch>,
    ),
    Error,
> {
    use crate::geocoding::address::{
        AddressFilter, AddressMatch, Adresse, DEFAULT_MIN_SCORE, filter_for, select,
    };

    let Some(address) = params.address.clone() else {
        return Ok((params, None));
    };
    let mut filters = std::collections::HashMap::new();
    if let Some(kinds) = super::adresses::filter_values(params.geocoding_type.as_deref()) {
        filters.insert("type".to_string(), kinds);
    }
    let opts = geocoder_core::SearchOpts {
        limit: Some(1),
        autocomplete: Some(false),
        filters,
        ..Default::default()
    };
    let results = super::adresses::geocode(context, address, opts).await?;

    let min_score = params.geocoding_min_score.unwrap_or(DEFAULT_MIN_SCORE);
    let mode = params.geocoding_mode.unwrap_or_default();
    let nothing = |params: &mut EtablissementSearchParams| {
        // Matches no establishment.
        params.lat = Some(0.0);
        params.lng = Some(0.0);
        params.radius = Some(0.0);
    };

    let Some((best, meets_min_score)) = select(&results, mode, min_score) else {
        let rejected = results.first().map(|r| AddressMatch {
            adresse: Adresse::from(r),
            min_score,
            meets_min_score: false,
            filter: None,
        });
        nothing(&mut params);
        return Ok((params, rejected));
    };

    let adresse = Adresse::from(best);
    // An explicit code_commune already narrows to municipalities: then a
    // municipality address narrows by distance instead.
    let mut filter = filter_for(&adresse, params.radius);
    if params.code_commune.is_some() && matches!(filter, AddressFilter::Commune { .. }) {
        filter = filter_for(
            &adresse,
            Some(crate::geocoding::address::RADIUS_MUNICIPALITY),
        );
    }
    match &filter {
        AddressFilter::Radius { lat, lng, radius } => {
            params.lat = Some(*lat);
            params.lng = Some(*lng);
            params.radius = Some(*radius);
        }
        AddressFilter::Commune { code_commune } => {
            params.code_commune = Some(code_commune.join(","));
        }
    }
    Ok((
        params,
        Some(AddressMatch {
            adresse,
            min_score,
            meets_min_score,
            filter: Some(filter),
        }),
    ))
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new()
        .routes(routes!(get_etablissement_by_siret))
        .routes(routes!(search_etablissements))
}
