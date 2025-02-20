use crate::connectors::Error as ConnectorError;
use crate::models::{etablissement, unite_legale, update_metadata};
use crate::update::error::Error as InternalUpdate;
use custom_error::custom_error;
use serde::Serialize;
use std::convert::Infallible;
use tracing::{debug, error, warn};
use warp::{Rejection, Reply, http::StatusCode};

custom_error! { pub Error
    InvalidData = "Invalid data",
    MissingApiKey = "[Admin] Missing API key in configuration",
    ApiKey = "[Admin] Wrong API key",
    MissingBaseUrlForAsync = "[Admin] No BASE_URL configured, needed for asynchronous updates",
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
    UpdateConnector {source: ConnectorError} = "[Update] Error while creating connector: {source}",
    Update {source: InternalUpdate} = "[Update] {source}",
    UniteLegale {source: unite_legale::error::Error} = "[UniteLegale] {source}",
    Etablissement {source: etablissement::error::Error} = "[Etablissement] {source}",
    Status {source: update_metadata::error::Error} = "[Status] {source}",
}

impl warp::reject::Reject for Error {}

impl From<ConnectorError> for Rejection {
    fn from(e: ConnectorError) -> Self {
        let error: Error = e.into();
        error.into()
    }
}

impl From<InternalUpdate> for Rejection {
    fn from(e: InternalUpdate) -> Self {
        let error: Error = e.into();
        error.into()
    }
}

impl From<unite_legale::error::Error> for Rejection {
    fn from(e: unite_legale::error::Error) -> Self {
        let error: Error = e.into();
        error.into()
    }
}

impl From<etablissement::error::Error> for Rejection {
    fn from(e: etablissement::error::Error) -> Self {
        let error: Error = e.into();
        error.into()
    }
}

impl From<update_metadata::error::Error> for Rejection {
    fn from(e: update_metadata::error::Error) -> Self {
        let error: Error = e.into();
        error.into()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let (code, message) = if err.is_not_found() {
        (StatusCode::NOT_FOUND, String::from("Not found"))
    } else if let Some(e) = err.find::<Error>() {
        debug!("[HandledError] {:?}", e);

        (
            match e {
                Error::InvalidData => StatusCode::BAD_REQUEST,
                Error::MissingApiKey => StatusCode::UNAUTHORIZED,
                Error::ApiKey => StatusCode::UNAUTHORIZED,
                Error::MissingBaseUrlForAsync => StatusCode::BAD_REQUEST,
                Error::LocalConnectionFailed { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
                Error::UpdateConnector { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
                Error::Update { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
                Error::UniteLegale { source } => match source {
                    unite_legale::error::Error::UniteLegaleNotFound => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                },
                Error::Etablissement { source } => match source {
                    etablissement::error::Error::EtablissementNotFound => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                },
                Error::Status { source } => match source {
                    update_metadata::error::Error::MetadataNotFound => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                },
            },
            e.to_string(),
        )
    } else if let Some(body_error) = err.find::<warp::body::BodyDeserializeError>() {
        warn!("[Json] {}", body_error);

        (StatusCode::BAD_REQUEST, body_error.to_string())
    } else if let Some(e) = err.find::<warp::reject::MethodNotAllowed>() {
        warn!("[Method] {}", e);

        (StatusCode::NOT_FOUND, String::from("Not found"))
    } else {
        warn!("[Rejection] Unhandled error {:?}", err);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("Internal server error"),
        )
    };

    if code == StatusCode::INTERNAL_SERVER_ERROR {
        error!("[InternalServerError] {}", message);
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&ErrorResponse {
            code: code.as_u16(),
            message,
        }),
        code,
    ))
}
