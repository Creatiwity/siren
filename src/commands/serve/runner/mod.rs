mod common;
mod error;

use crate::connectors::ConnectorsBuilders;
use crate::models;
use common::{
    Context, EtablissementInnerResponse, EtablissementResponse,
    UniteLegaleEtablissementInnerResponse, UniteLegaleInnerResponse, UniteLegaleResponse,
};
use error::Error;
use rocket::config::Config;
use rocket::State;
use rocket_contrib::json::Json;

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
        .mount("/v3", routes![index, unites_legales, etablissements])
        .manage(Context {
            connectors: builders.create(),
        })
        .launch();
}
