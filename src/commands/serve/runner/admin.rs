use super::common::{Context, StatusQueryString, UpdateOptions};
use super::error::Error;
use crate::models;
use crate::models::update_metadata::common::UpdateMetadata;
use crate::update::{common::Config as DataConfig, update as update_data};
use axum::{
    Json,
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

impl From<UpdateMetadata> for StatusCode {
    fn from(metadata: UpdateMetadata) -> Self {
        match metadata.status.as_str() {
            "launched" => StatusCode::ACCEPTED,
            "error" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::OK,
        }
    }
}

/// Update data
#[utoipa::path(
    post,
    path = "/update",
    request_body = UpdateOptions,
    responses(
        (status = 202, description = "Update launched"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 500, description = "Internal server error")
    ),
    tag = super::common::ADMIN_TAG
)]
async fn post_update(
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

/// Get update status
#[utoipa::path(
    get,
    path = "/update/status",
    params(
        ("api_key" = String, Query, description = "API key")
    ),
    responses(
        (status = 200, description = "Update finished"),
        (status = 202, description = "Update in progress"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 500, description = "Internal server error")
    ),
    tag = super::common::ADMIN_TAG
)]
async fn get_update_status(
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

/// Set update status to error
#[utoipa::path(
    post,
    path = "/update/status/error",
    request_body = StatusQueryString,
    responses(
        (status = 200, description = "Status error response"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 500, description = "Internal server error")
    ),
    tag = super::common::ADMIN_TAG
)]
async fn post_update_status_to_error(
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

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new()
        .routes(routes!(post_update, get_update_status))
        .routes(routes!(post_update_status_to_error))
}
