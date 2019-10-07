use crate::connectors::{Connectors, ConnectorsBuilders};
use crate::models;
use crate::models::etablissement::common::Etablissement;
use crate::models::unite_legale::common::UniteLegale;
use crate::models::{etablissement, unite_legale};
use custom_error::custom_error;
use rocket::config::Config;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket::State;
use rocket_contrib::json::Json;
use serde::Serialize;

custom_error! { pub Error
    UniteLegaleError {source: unite_legale::error::Error} = "Error on UniteLegale model: {source}.",
    EtablissementError {source: etablissement::error::Error} = "Error on Etablissement model: {source}.",
}

impl<'r> Responder<'r> for Error {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        // Log error
        println!("{}", self);

        match self {
            Error::UniteLegaleError { source } => match source {
                models::unite_legale::error::Error::UniteLegaleNotFound => Err(Status::NotFound),
                _ => Err(Status::InternalServerError),
            },
            Error::EtablissementError { source } => match source {
                models::etablissement::error::Error::EtablissementNotFound => Err(Status::NotFound),
                _ => Err(Status::InternalServerError),
            },
        }
    }
}

struct Context {
    connectors: Connectors,
}

#[derive(Serialize)]
struct UniteLegaleResponse {
    unite_legale: UniteLegaleInnerResponse,
}

#[derive(Serialize)]
struct UniteLegaleInnerResponse {
    #[serde(flatten)]
    unite_legale: UniteLegale,
    etablissements: Vec<Etablissement>,
    etablissement_siege: Etablissement,
}

#[derive(Serialize)]
struct EtablissementResponse {
    etablissement: EtablissementInnerResponse,
}

#[derive(Serialize)]
struct EtablissementInnerResponse {
    #[serde(flatten)]
    etablissement: Etablissement,
    unite_legale: UniteLegaleEtablissementInnerResponse,
}

#[derive(Serialize)]
struct UniteLegaleEtablissementInnerResponse {
    #[serde(flatten)]
    unite_legale: UniteLegale,
    etablissement_siege: Etablissement,
}

#[get("/")]
fn index() -> &'static str {
    "SIRENE API v3"
}

#[get("/unites_legales/<siren>")]
fn unites_legales(
    state: State<Context>,
    siren: String,
) -> Result<Json<UniteLegaleResponse>, Error> {
    let unite_legale = models::unite_legale::get(&state.connectors, &siren)?;
    let etablissements = models::etablissement::get_with_siren(&state.connectors, &siren)?;
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&state.connectors, &unite_legale.siren)?;

    Ok(Json(UniteLegaleResponse {
        unite_legale: UniteLegaleInnerResponse {
            unite_legale,
            etablissements,
            etablissement_siege,
        },
    }))
}

#[get("/etablissements/<siret>")]
fn etablissements(
    state: State<Context>,
    siret: String,
) -> Result<Json<EtablissementResponse>, Error> {
    let etablissement = models::etablissement::get(&state.connectors, &siret)?;
    let unite_legale = models::unite_legale::get(&state.connectors, &etablissement.siren)?;
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&state.connectors, &etablissement.siren)?;

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

pub fn run(config: Config, builders: ConnectorsBuilders) {
    rocket::custom(config)
        .mount("/", routes![index, unites_legales, etablissements])
        .manage(Context {
            connectors: builders.create(),
        })
        .launch();
}
