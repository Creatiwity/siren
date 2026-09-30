//! Address endpoints (feature `geocoding`): `/v3/adresses/autocomplete` for
//! type-ahead, `/v3/adresses/search` to geocode a full address.

use super::common::Context;
use super::error::Error;
use crate::geocoding::address::Adresse;
use axum::{
    Json,
    extract::{Query, State},
};
use geocoder_core::{SearchOpts, SearchResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

const DEFAULT_LIMIT: usize = 5;
const MAX_LIMIT: usize = 20;

/// Geocode off the async runtime: the index is memory mapped, a query may
/// fault pages in.
pub async fn geocode(
    context: &Context,
    query: String,
    opts: SearchOpts,
) -> Result<Vec<SearchResult>, Error> {
    let Some(geocoder) = context.geocoder.get() else {
        return Err(match context.geocoder.path() {
            Some(_) => Error::GeocodingUnavailable,
            None => Error::GeocodingDisabled,
        });
    };
    tokio::task::spawn_blocking(move || geocoder.search(&query, opts))
        .await
        .map_err(|_| Error::GeocodingUnavailable)
}

/// Comma-separated list into filter values.
pub fn filter_values(value: Option<&str>) -> Option<Vec<String>> {
    let values: Vec<String> = value?
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .collect();
    (!values.is_empty()).then_some(values)
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct AdresseParams {
    /// Address, or its beginning for autocomplete (3 to 200 characters).
    pub q: String,
    /// Number of results, 1 to 20 (default 5).
    pub limit: Option<usize>,
    /// Favour addresses around this point (with `lng`).
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    /// Comma-separated: `housenumber`, `street`, `locality`, `municipality`.
    #[serde(rename = "type")]
    #[param(rename = "type")]
    pub kind: Option<String>,
    /// Comma-separated postal codes.
    pub postcode: Option<String>,
    /// Comma-separated INSEE codes.
    pub citycode: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AdresseResponse {
    /// Best first.
    pub adresses: Vec<Adresse>,
}

fn search_opts(params: &AdresseParams, autocomplete: bool) -> Result<SearchOpts, Error> {
    let invalid = |message: &str| Error::InvalidSearchParams {
        message: message.to_string(),
    };
    let q = params.q.trim();
    if q.chars().count() < 3 || q.chars().count() > 200 {
        return Err(invalid("q must contain between 3 and 200 characters"));
    }
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(invalid("limit must be between 1 and 20"));
    }
    if params.lat.is_some() != params.lng.is_some() {
        return Err(invalid("lat and lng must be provided together"));
    }
    let mut filters = HashMap::new();
    for (key, value) in [
        ("type", &params.kind),
        ("postcode", &params.postcode),
        ("citycode", &params.citycode),
    ] {
        if let Some(values) = filter_values(value.as_deref()) {
            filters.insert(key.to_string(), values);
        }
    }
    Ok(SearchOpts {
        limit: Some(limit),
        lat: params.lat,
        lon: params.lng,
        filters,
        autocomplete: Some(autocomplete),
    })
}

async fn run(
    context: &Context,
    params: AdresseParams,
    autocomplete: bool,
) -> Result<Json<AdresseResponse>, Error> {
    let opts = search_opts(&params, autocomplete)?;
    let results = geocode(context, params.q.trim().to_string(), opts).await?;
    Ok(Json(AdresseResponse {
        adresses: results.iter().map(Adresse::from).collect(),
    }))
}

/// Autocomplete an address
///
/// For type-ahead: the last word may be incomplete. Results come from the
/// Base Adresse Nationale, ranked like the BAN API.
#[utoipa::path(
    get,
    path = "/autocomplete",
    params(AdresseParams),
    responses(
        (status = 200, description = "Matching addresses, best first", body = AdresseResponse),
        (status = 400, description = "Invalid parameters"),
        (status = 503, description = "Geocoding index not loaded yet")
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn autocomplete_adresses(
    State(context): State<Arc<Context>>,
    Query(params): Query<AdresseParams>,
) -> Result<Json<AdresseResponse>, Error> {
    run(&context, params, true).await
}

/// Geocode an address
///
/// For a complete address: returns its position and normalized parts.
#[utoipa::path(
    get,
    path = "/search",
    params(AdresseParams),
    responses(
        (status = 200, description = "Matching addresses, best first", body = AdresseResponse),
        (status = 400, description = "Invalid parameters"),
        (status = 503, description = "Geocoding index not loaded yet")
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn search_adresses(
    State(context): State<Arc<Context>>,
    Query(params): Query<AdresseParams>,
) -> Result<Json<AdresseResponse>, Error> {
    run(&context, params, false).await
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new()
        .routes(routes!(autocomplete_adresses))
        .routes(routes!(search_adresses))
}
