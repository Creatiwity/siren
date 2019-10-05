use crate::connectors::{Connectors, ConnectorsBuilders};
use crate::models;
use crate::models::etablissement::common::Etablissement;
use crate::models::unite_legale::common::UniteLegale;
use rocket::config::Config;
use rocket::State;
use rocket_contrib::json::Json;
use serde::Serialize;

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
fn unites_legales(state: State<Context>, siren: String) -> Json<UniteLegaleResponse> {
    let unite_legale = models::unite_legale::get(&state.connectors, &siren).unwrap();
    let etablissements = models::etablissement::get_with_siren(&state.connectors, &siren).unwrap();
    let etablissement_siege = etablissements
        .iter()
        .find(|&e| e.etablissement_siege)
        .unwrap()
        .clone();

    Json(UniteLegaleResponse {
        unite_legale: UniteLegaleInnerResponse {
            unite_legale,
            etablissements,
            etablissement_siege,
        },
    })
}

#[get("/etablissements/<siret>")]
fn etablissements(state: State<Context>, siret: String) -> Json<EtablissementResponse> {
    let etablissement = models::etablissement::get(&state.connectors, &siret).unwrap();
    let unite_legale = models::unite_legale::get(&state.connectors, &etablissement.siren).unwrap();
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&state.connectors, &etablissement.siren)
            .unwrap();

    Json(EtablissementResponse {
        etablissement: EtablissementInnerResponse {
            etablissement,
            unite_legale: UniteLegaleEtablissementInnerResponse {
                unite_legale,
                etablissement_siege,
            },
        },
    })
}

pub fn run(config: Config, builders: ConnectorsBuilders) {
    rocket::custom(config)
        .mount("/", routes![index, unites_legales, etablissements])
        .manage(Context {
            connectors: builders.create(),
        })
        .launch();
}
