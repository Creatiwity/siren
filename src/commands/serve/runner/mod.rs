mod error;

pub mod common;

use crate::models;
use crate::models::update_metadata::common::UpdateMetadata;
use crate::update::{common::Config as DataConfig, update as update_data};
use chrono::Utc;
use common::{
    Context, EtablissementInnerResponse, EtablissementResponse, MetadataResponse,
    StatusQueryString, UniteLegaleEtablissementInnerResponse, UniteLegaleInnerResponse,
    UniteLegaleResponse, UpdateOptions,
};
use error::Error;
use serde::Serialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use tracing::{info, instrument, span, Level};
use warp::{
    http::{header, Method, StatusCode},
    Filter, Rejection, Reply,
};

impl From<UpdateMetadata> for StatusCode {
    fn from(metadata: UpdateMetadata) -> Self {
        match metadata.status.as_str() {
            "launched" => StatusCode::ACCEPTED,
            "error" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::OK,
        }
    }
}

async fn index(context: Context) -> Result<impl Reply, Rejection> {
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

    Ok(warp::reply::with_status(
        warp::reply::json(&reply),
        StatusCode::OK,
    ))
}

async fn update(options: UpdateOptions, context: Context) -> Result<impl Reply, Rejection> {
    let api_key = match &context.api_key {
        Some(key) => key,
        None => return Err(Error::MissingApiKey.into()),
    };

    if &options.api_key != api_key {
        return Err(Error::ApiKey.into());
    }

    if options.asynchronous && context.base_url.is_none() {
        return Err(Error::MissingBaseUrlForAsync.into());
    }

    let mut connectors = context.builders.create_with_insee()?;

    let update_metadata = update_data(
        options.group_type,
        DataConfig {
            force: options.force,
            asynchronous: options.asynchronous,
        },
        &mut connectors,
    )
    .await?;

    reply_with_update_metadata(update_metadata, context.base_url, api_key)
}

async fn status(query: StatusQueryString, context: Context) -> Result<impl Reply, Rejection> {
    let api_key = match &context.api_key {
        Some(key) => key,
        None => return Err(Error::MissingApiKey.into()),
    };

    if &query.api_key != api_key {
        return Err(Error::ApiKey.into());
    }

    let connectors = context.builders.create();

    let update_metadata = models::update_metadata::current_update(&connectors)?;

    reply_with_update_metadata(update_metadata, context.base_url, api_key)
}

fn reply_with_update_metadata<T: Serialize + Into<StatusCode>>(
    update_metadata: T,
    base_url: Option<String>,
    api_key: &str,
) -> Result<impl Reply + use<T>, Rejection> {
    Ok(warp::reply::with_status(
        warp::reply::with_header(
            warp::reply::with_header(
                warp::reply::json(&update_metadata),
                "Location",
                format!(
                    "{}/admin/update/status?api_key={}",
                    base_url.unwrap_or_default(),
                    api_key
                ),
            ),
            "Retry-After",
            "10",
        ),
        update_metadata.into(),
    ))
}

async fn set_status_to_error(
    query: StatusQueryString,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let api_key = match &context.api_key {
        Some(key) => key,
        None => return Err(Error::MissingApiKey.into()),
    };

    if &query.api_key != api_key {
        return Err(Error::ApiKey.into());
    }

    let connectors = context.builders.create();

    models::update_metadata::error_update(
        &connectors,
        String::from("Process stopped manually."),
        Utc::now(),
    )?;

    Ok(warp::reply())
}

#[instrument(level = "trace")]
async fn unites_legales(siren: String, context: Context) -> Result<impl Reply, Rejection> {
    let span = span!(Level::TRACE, "GET /unites_legales");
    let _enter = span.enter();

    if siren.len() != 9 {
        return Err(Error::InvalidData.into());
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

    Ok(warp::reply::json(&UniteLegaleResponse {
        unite_legale: UniteLegaleInnerResponse {
            unite_legale,
            etablissements,
            etablissement_siege,
        },
    }))
}

#[instrument(level = "trace")]
async fn etablissements(siret: String, context: Context) -> Result<impl Reply, Rejection> {
    let span = span!(Level::TRACE, "GET /etablissements");
    let _enter = span.enter();

    if siret.len() != 14 {
        return Err(Error::InvalidData.into());
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

    Ok(warp::reply::json(&EtablissementResponse {
        etablissement: EtablissementInnerResponse {
            etablissement,
            unite_legale: UniteLegaleEtablissementInnerResponse {
                unite_legale,
                etablissement_siege,
            },
        },
    }))
}

pub async fn run(addr: SocketAddr, context: Context) {
    // GET / -> OK
    let health_route = warp::get()
        .and(warp::path::end())
        .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));
    info!("Mount GET /");

    let v3_route = warp::path!("v3" / ..);

    // GET /v3 -> Status
    let v3_index = warp::path::end()
        .and(with_context(context.clone()))
        .and_then(index);
    info!("Mount GET /v3");

    // GET /unites_legales/<siren>
    let v3_unites_legales_route = warp::get()
        .and(warp::path!("unites_legales" / String))
        .and(with_context(context.clone()))
        .and_then(unites_legales);
    info!("Mount GET /v3/unites_legales/<siren>");

    // GET /etablissements/<siret>
    let v3_etablissement_route = warp::get()
        .and(warp::path!("etablissements" / String))
        .and(with_context(context.clone()))
        .and_then(etablissements);
    info!("Mount GET /v3/etablissements/<siret>");

    let admin_update_route = warp::path!("admin" / "update" / ..);

    // POST /admin/update {json}
    let update_route = warp::post()
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 32))
        .and(warp::body::json::<UpdateOptions>())
        .and(with_context(context.clone()))
        .and_then(update);
    info!("Mount POST /admin/update {{json}}");

    // GET /admin/update/status?api_key=""
    let status_route = warp::get()
        .and(warp::path!("status"))
        .and(warp::query::<StatusQueryString>())
        .and(with_context(context.clone()))
        .and_then(status);
    info!("Mount GET /admin/update/status?api_key=");

    // POST /admin/update/status/error { api_key }
    let status_error_route = warp::post()
        .and(warp::path!("status" / "error"))
        .and(warp::body::content_length_limit(1024 * 32))
        .and(warp::body::json::<StatusQueryString>())
        .and(with_context(context))
        .and_then(set_status_to_error);
    info!("Mount POST /admin/update/status/error {{api_key}}");

    // Cors
    let cors = warp::cors()
        .allow_methods(&[Method::GET, Method::POST])
        .allow_headers(vec![header::CONTENT_TYPE])
        .allow_any_origin();

    let routes = health_route
        .or(v3_route.and(
            v3_unites_legales_route
                .or(v3_etablissement_route)
                .or(v3_index),
        ))
        .or(admin_update_route.and(status_route.or(update_route).or(status_error_route)))
        .recover(error::handle_rejection)
        .with(warp::trace::request())
        .with(cors);

    warp::serve(routes).run(addr).await;
}

fn with_context(context: Context) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
    warp::any().map(move || context.clone())
}
