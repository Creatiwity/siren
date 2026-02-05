use crate::connectors::Error as ConnectorError;
use crate::models::{etablissement, unite_legale, update_metadata};
use crate::update::error::Error as InternalUpdate;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use custom_error::custom_error;
use serde::Serialize;
use tracing::error;

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

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::InvalidData => (StatusCode::BAD_REQUEST, self.to_string()),
            Error::MissingApiKey => (StatusCode::UNAUTHORIZED, self.to_string()),
            Error::ApiKey => (StatusCode::UNAUTHORIZED, self.to_string()),
            Error::MissingBaseUrlForAsync => (StatusCode::BAD_REQUEST, self.to_string()),
            Error::LocalConnectionFailed { source: _ } => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            Error::UpdateConnector { source: _ } => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            Error::Update { source: _ } => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Error::UniteLegale { ref source } => match source {
                unite_legale::error::Error::UniteLegaleNotFound => {
                    (StatusCode::NOT_FOUND, self.to_string())
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            },
            Error::Etablissement { ref source } => match source {
                etablissement::error::Error::EtablissementNotFound => {
                    (StatusCode::NOT_FOUND, self.to_string())
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            },
            Error::Status { ref source } => match source {
                update_metadata::error::Error::MetadataNotFound => {
                    (StatusCode::NOT_FOUND, self.to_string())
                }
                _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            },
        };

        if status == StatusCode::INTERNAL_SERVER_ERROR {
            error!("[InternalServerError] {}", message);
        }

        Json(ErrorResponse {
            code: status.as_u16(),
            message,
        })
        .into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
}
