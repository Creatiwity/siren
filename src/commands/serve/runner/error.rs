use crate::connectors::Error as ConnectorError;
use crate::models::{etablissement, unite_legale};
use crate::update::error::Error as InternalUpdateError;
use custom_error::custom_error;
use serde::Serialize;
use std::convert::Infallible;
use warp::{http::StatusCode, Rejection, Reply};

custom_error! { pub Error
    InvalidData = "Invalid data",
    MissingApiKeyError = "[Admin] Missing API key in configuration",
    ApiKeyError = "[Admin] Wrong API key",
    UpdateConnectorError {source: ConnectorError} = "[Update] Error while creating connector: {source}",
    UpdateError {source: InternalUpdateError} = "[Update] {source}",
    UniteLegaleError {source: unite_legale::error::Error} = "[UniteLegale] {source}",
    EtablissementError {source: etablissement::error::Error} = "[Etablissement] {source}",
}

impl warp::reject::Reject for Error {}

impl From<Error> for Rejection {
    fn from(e: Error) -> Self {
        warp::reject::custom(e)
    }
}

impl From<ConnectorError> for Rejection {
    fn from(e: ConnectorError) -> Self {
        let error: Error = e.into();
        error.into()
    }
}

impl From<InternalUpdateError> for Rejection {
    fn from(e: InternalUpdateError) -> Self {
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

#[derive(Debug, Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let (code, message) = if let Some(e) = err.find::<Error>() {
        eprintln!("[Warp][Error] {:?}", e);

        (
            match e {
                Error::InvalidData => StatusCode::BAD_REQUEST,
                Error::MissingApiKeyError => StatusCode::UNAUTHORIZED,
                Error::ApiKeyError => StatusCode::UNAUTHORIZED,
                Error::UpdateConnectorError { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
                Error::UpdateError { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
                Error::UniteLegaleError { source } => match source {
                    unite_legale::error::Error::UniteLegaleNotFound => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                },
                Error::EtablissementError { source } => match source {
                    etablissement::error::Error::EtablissementNotFound => StatusCode::NOT_FOUND,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                },
            },
            e.to_string(),
        )
    } else if let Some(body_error) = err.find::<warp::body::BodyDeserializeError>() {
        eprintln!("[Warp][Json] {}", body_error);

        (StatusCode::BAD_REQUEST, body_error.to_string())
    } else if let Some(e) = err.find::<warp::reject::MethodNotAllowed>() {
        eprintln!("[Warp][Method] {}", e);

        (StatusCode::NOT_FOUND, String::from("Not found"))
    } else {
        eprintln!("[Warp][Rejection] Unhandled error {:?}", err);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("Internal server error"),
        )
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&ErrorResponse { code: code.as_u16(), message }),
        code,
    ))
}
