mod error;

pub mod common;

use crate::models;
use crate::update::{common::Config as DataConfig, update as update_data};
use common::{
    Context, EtablissementInnerResponse, EtablissementResponse,
    UniteLegaleEtablissementInnerResponse, UniteLegaleInnerResponse, UniteLegaleResponse,
    UpdateOptions, UpdateResponse,
};
use error::Error;
use std::convert::Infallible;
use std::net::SocketAddr;
use warp::{
    http::{header, Method},
    Filter, Rejection, Reply,
};

fn index() -> &'static str {
    "SIRENE API v3"
}

async fn update(options: UpdateOptions, context: Context) -> Result<impl Reply, Rejection> {
    let api_key = match &context.api_key {
        Some(key) => key,
        None => return Err(Error::MissingApiKeyError.into()),
    };

    if &options.api_key != api_key {
        return Err(Error::ApiKeyError.into());
    }

    let connectors = context.builders.create_with_insee()?;

    let summary = update_data(
        options.group_type,
        DataConfig {
            force: options.force,
            data_only: options.data_only,
            temp_folder: context.folder_options.temp.clone(),
            file_folder: context.folder_options.file.clone(),
            db_folder: context.folder_options.db.clone(),
        },
        &connectors,
    )?;

    Ok(warp::reply::json(&UpdateResponse { summary }))
}

async fn unites_legales(siren: String, context: Context) -> Result<impl Reply, Rejection> {
    if siren.len() != 9 {
        return Err(Error::InvalidData.into());
    }

    let connectors = context.builders.create();

    let unite_legale = models::unite_legale::get(&connectors, &siren)?;
    let etablissements = models::etablissement::get_with_siren(&connectors, &siren)?;
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&connectors, &unite_legale.siren)?;

    Ok(warp::reply::json(&UniteLegaleResponse {
        unite_legale: UniteLegaleInnerResponse {
            unite_legale,
            etablissements,
            etablissement_siege,
        },
    }))
}

async fn etablissements(siret: String, context: Context) -> Result<impl Reply, Rejection> {
    if siret.len() != 14 {
        return Err(Error::InvalidData.into());
    }

    let connectors = context.builders.create();

    let etablissement = models::etablissement::get(&connectors, &siret)?;
    let unite_legale = models::unite_legale::get(&connectors, &etablissement.siren)?;
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&connectors, &etablissement.siren)?;

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
    log::info!("[Warp] Mount GET /");

    let v3_route = warp::path!("v3" / ..);

    // GET /v3 -> "SIRENE API v3"
    let v3_index = warp::path::end().map(index);
    log::info!("[Warp] Mount GET /v3");

    // GET /unites_legales/<siren>
    let v3_unites_legales_route = warp::get()
        .and(warp::path!("unites_legales" / String))
        .and(with_context(context.clone()))
        .and_then(unites_legales);
    log::info!("[Warp] Mount GET /v3/unites_legales/<siren>");

    // GET /etablissements/<siret>
    let v3_etablissement_route = warp::get()
        .and(warp::path!("etablissements" / String))
        .and(with_context(context.clone()))
        .and_then(etablissements);
    log::info!("[Warp] Mount GET /v3/etablissements/<siret>");

    // POST /admin/update {json}
    let admin_update_route = warp::post()
        .and(warp::path!("admin" / "update"))
        .and(warp::body::json::<UpdateOptions>())
        .and(with_context(context))
        .and_then(update);
    log::info!("[Warp] Mount POST /admin/update {{json}}");

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
        .or(admin_update_route)
        .recover(error::handle_rejection)
        .with(cors);

    warp::serve(routes).run(addr).await;
}

fn with_context(context: Context) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
    warp::any().map(move || context.clone())
}
