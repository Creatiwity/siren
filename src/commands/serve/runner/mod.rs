mod common;
mod error;

use super::super::common::FolderOptions;
use crate::connectors::ConnectorsBuilders;
use crate::models;
use crate::update::{common::Config as DataConfig, update as update_data};
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
async fn update(
    state: State<Context>,
    options: Json<UpdateOptions>,
) -> Result<Json<UpdateResponse>, Error> {
    let api_key = match &state.api_key {
        Some(key) => key,
        None => return Err(Error::MissingApiKeyError),
    };

    if &options.api_key != api_key {
        return Err(Error::ApiKeyError);
    }

    let connectors = state.builders.create_with_insee().await;

    let summary = update_data(
        options.group_type,
        DataConfig {
            force: options.force,
            data_only: options.data_only,
            temp_folder: state.folder_options.temp.clone(),
            file_folder: state.folder_options.file.clone(),
            db_folder: state.folder_options.db.clone(),
        },
        &connectors,
    )?;

    Ok(Json(UpdateResponse { summary }))
}

#[get("/unites_legales/<siren>")]
fn unites_legales(
    state: State<Context>,
    siren: String,
) -> Result<Json<UniteLegaleResponse>, Error> {
    if siren.len() != 9 {
        return Err(Error::InvalidData);
    }

    let connectors = state.builders.create();

    let unite_legale = models::unite_legale::get(&connectors, &siren)?;
    let etablissements = models::etablissement::get_with_siren(&connectors, &siren)?;
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&connectors, &unite_legale.siren)?;

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

    let connectors = state.builders.create();

    let etablissement = models::etablissement::get(&connectors, &siret)?;
    let unite_legale = models::unite_legale::get(&connectors, &etablissement.siren)?;
    let etablissement_siege =
        models::etablissement::get_siege_with_siren(&connectors, &etablissement.siren)?;

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

pub fn run(
    config: Config,
    api_key: Option<String>,
    folder_options: FolderOptions,
    builders: ConnectorsBuilders,
) {
    rocket::custom(config)
        .mount("/v3", routes![index, unites_legales, etablissements])
        .mount("/admin", routes![update])
        .manage(Context {
            builders,
            api_key,
            folder_options,
        })
        .launch();
}
