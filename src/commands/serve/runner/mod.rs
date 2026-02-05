mod error;

pub mod common;

use crate::models;
use crate::models::update_metadata::common::UpdateMetadata;
use crate::update::{common::Config as DataConfig, update as update_data};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use common::{
    Context, EtablissementInnerResponse, EtablissementResponse, MetadataResponse,
    StatusQueryString, UniteLegaleEtablissementInnerResponse, UniteLegaleInnerResponse,
    UniteLegaleResponse, UpdateOptions,
};
use error::Error;
use sentry::integrations::tower::NewSentryLayer;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{Level, info, instrument, span};

impl From<UpdateMetadata> for StatusCode {
    fn from(metadata: UpdateMetadata) -> Self {
        match metadata.status.as_str() {
            "launched" => StatusCode::ACCEPTED,
            "error" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::OK,
        }
    }
}

async fn health_check() -> &'static str {
    "OK"
}

async fn index(State(context): State<Arc<Context>>) -> Result<Json<MetadataResponse>, Error> {
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

    Ok(Json(reply))
}

async fn update(
    State(context): State<Arc<Context>>,
    Json(options): Json<UpdateOptions>,
) -> Result<Response, Error> {
    let api_key = match &context.api_key {
        Some(key) => key,
        None => return Err(Error::MissingApiKey),
    };

    if &options.api_key != api_key {
        return Err(Error::ApiKey);
    }

    if options.asynchronous && context.base_url.is_none() {
        return Err(Error::MissingBaseUrlForAsync);
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

    reply_with_update_metadata(update_metadata, context.base_url.clone(), api_key)
}

async fn status(
    State(context): State<Arc<Context>>,
    Query(query): Query<StatusQueryString>,
) -> Result<Response, Error> {
    let api_key = match &context.api_key {
        Some(key) => key,
        None => return Err(Error::MissingApiKey),
    };

    if &query.api_key != api_key {
        return Err(Error::ApiKey);
    }

    let connectors = context.builders.create();

    let update_metadata = models::update_metadata::current_update(&connectors)?;

    reply_with_update_metadata(update_metadata, context.base_url.clone(), api_key)
}

fn reply_with_update_metadata<T: Serialize + Clone + Into<StatusCode>>(
    update_metadata: T,
    base_url: Option<String>,
    api_key: &str,
) -> Result<Response, Error> {
    let status_code = update_metadata.clone().into();
    let mut response = Json(update_metadata).into_response();
    *response.status_mut() = status_code;

    if let Some(base_url) = base_url {
        response.headers_mut().insert(
            "Location",
            format!("{}/admin/update/status?api_key={}", base_url, api_key)
                .parse()
                .unwrap(),
        );
        response
            .headers_mut()
            .insert("Retry-After", "10".parse().unwrap());
    }

    Ok(response)
}

async fn set_status_to_error(
    State(context): State<Arc<Context>>,
    Json(query): Json<StatusQueryString>,
) -> Result<Response, Error> {
    let api_key = match &context.api_key {
        Some(key) => key,
        None => return Err(Error::MissingApiKey),
    };

    if &query.api_key != api_key {
        return Err(Error::ApiKey);
    }

    let connectors = context.builders.create();

    models::update_metadata::error_update(
        &connectors,
        String::from("Process stopped manually."),
        Utc::now(),
    )?;

    Ok(Response::new(Body::empty()))
}

#[instrument(level = "trace")]
async fn unites_legales(
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

#[instrument(level = "trace")]
async fn etablissements(
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

pub async fn run(addr: SocketAddr, context: Context) {
    let shared_context = Arc::new(context);

    let app = Router::new()
        .route("/", get(health_check))
        .route("/v3", get(index))
        .route("/v3/unites_legales/{siren}", get(unites_legales))
        .route("/v3/etablissements/{siret}", get(etablissements))
        .route("/admin/update", post(update))
        .route("/admin/update/status", get(status))
        .route("/admin/update/status/error", post(set_status_to_error))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::CONTENT_TYPE])
                .allow_origin(tower_http::cors::Any),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(NewSentryLayer::new_from_top())
        .with_state(shared_context);

    info!("Mount GET /");
    info!("Mount GET /v3");
    info!("Mount GET /v3/unites_legales/<siren>");
    info!("Mount GET /v3/etablissements/<siret>");
    info!("Mount POST /admin/update {{json}}");
    info!("Mount GET /admin/update/status?api_key=");
    info!("Mount POST /admin/update/status/error {{api_key}}");

    axum::serve(
        tokio::net::TcpListener::bind(&addr).await.unwrap(),
        app.into_make_service(),
    )
    .await
    .unwrap();
}
