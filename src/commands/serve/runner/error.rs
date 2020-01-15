use crate::models::{etablissement, unite_legale};
use custom_error::custom_error;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, content, Responder, Response};
use serde::Serialize;

custom_error! { pub Error
    InvalidData = "Invalid data",
    UniteLegaleError {source: unite_legale::error::Error} = "[UniteLegale] {source}",
    EtablissementError {source: etablissement::error::Error} = "[Etablissement] {source}",
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

impl<'r> Responder<'r> for Error {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        // Log error
        println!("{}", self);

        let status = match &self {
            Error::InvalidData => Status::BadRequest,
            Error::UniteLegaleError { source } => match source {
                unite_legale::error::Error::UniteLegaleNotFound => Status::NotFound,
                _ => Status::InternalServerError,
            },
            Error::EtablissementError { source } => match source {
                etablissement::error::Error::EtablissementNotFound => Status::NotFound,
                _ => Status::InternalServerError,
            },
        };

        let error_response = ErrorResponse {
            message: self.to_string(),
        };

        let json_result = serde_json::to_string(&error_response)
            .map(|string| content::Json(string).respond_to(req).unwrap())
            .map_err(|e| {
                eprintln!("JSON failed to serialize: {:?}", e);
                Status::InternalServerError
            });

        match json_result {
            Ok(json) => Response::build_from(json).status(status).ok(),
            Err(status) => Err(status),
        }
    }
}
