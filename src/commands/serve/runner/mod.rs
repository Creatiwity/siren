mod common;
mod error;

use super::super::common::FolderOptions;
use super::super::update::runner::{update as update_data, Config as DataConfig};
use crate::connectors::ConnectorsBuilders;
use crate::models;
use common::{
    Context, EtablissementInnerResponse, EtablissementResponse,
    UniteLegaleEtablissementInnerResponse, UniteLegaleInnerResponse, UniteLegaleResponse,
    UpdateOptions, UpdateResponse,
};
use error::Error;
use rocket::config::Config;
use rocket::State;
use rocket_contrib::json::Json;

#[get("/")]
fn index() -> &'static str {
    "SIRENE API v3"
}

#[post("/update", format = "application/json", data = "<options>")]
fn update(
    state: State<Context>,
    options: Json<UpdateOptions>,
) -> Result<Json<UpdateResponse>, Error> {
    update_data(
        &options.group_type.into(),
        DataConfig {
            force: false,
            data_only: false,
            temp_folder: state.folder_options.temp.clone(),
            file_folder: state.folder_options.file.clone(),
            db_folder: state.folder_options.db.clone(),
        },
        &state.connectors,
    )?;

    Ok(Json(UpdateResponse {}))
}

#[get("/unites_legales/<siren>")]
fn unites_legales(
    state: State<Context>,
    siren: String,
) -> Result<Json<UniteLegaleResponse>, Error> {
    if siren.len() != 9 {
        return Err(Error::InvalidData);
    }

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
    if siret.len() != 14 {
        return Err(Error::InvalidData);
    }

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

pub fn run(config: Config, folder_options: FolderOptions, builders: ConnectorsBuilders) {
    rocket::custom(config)
        .mount(
            "/v3",
            routes![index, update, unites_legales, etablissements],
        )
        .manage(Context {
            connectors: builders.create(),
            folder_options,
        })
        .launch();
}
