use custom_error::custom_error;
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::env;

custom_error! { pub Error
    TokenNetworkError { source: reqwest::Error } = "Unable to retrieve INSEE token (network error: {source})",
    TokenApiError = "Unable to retrieve INSEE token",
    TokenMalformedError {source: serde_json::Error} = "Unable to read INSEE token ({source})",
}

pub struct Connector {
    pub token: String,
}

pub struct ConnectorBuilder {
    pub credentials: String,
}

#[derive(Serialize)]
struct InseeTokenParameters {
    pub grant_type: String,
    pub validity_period: u32,
}

#[derive(Deserialize)]
struct InseeTokenResponse {
    pub access_token: String,
    pub scope: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl ConnectorBuilder {
    pub fn new() -> Option<ConnectorBuilder> {
        let credentials = env::var("INSEE_CREDENTIALS").ok();

        if let Some(credentials) = credentials {
            Some(ConnectorBuilder { credentials })
        } else {
            None
        }
    }

    pub fn create(&self) -> Result<Connector, Error> {
        self.generate_token().map(|token| Connector { token })
    }

    fn generate_token(&self) -> Result<String, Error> {
        let client = reqwest::blocking::Client::new();
        let response: InseeTokenResponse = client
            .post("https://api.insee.fr/token")
            .header(AUTHORIZATION, format!("Basic {}", self.credentials))
            .form(&InseeTokenParameters {
                grant_type: String::from("client_credentials"),
                validity_period: 86400,
            })
            .send()?
            .json()?;

        Ok(response.access_token)
    }
}

custom_error! { pub InseeError
    NetworkError {source: reqwest::Error} = "Unable to retrieve INSEE data (network error: {source})",
}

fn default_as_false() -> bool {
    false
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Header {
    total: u32,
    debut: u32,
    nombre: u32,
    curseur: String,
    curseurSuivant: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct PeriodeUniteLegale {
    dateFin: Option<String>,
    dateDebut: String,
    etatAdministratifUniteLegale: String,
    changementEtatAdministratifUniteLegale: bool,
    nomUniteLegale: Option<String>,
    changementNomUniteLegale: bool,
    nomUsageUniteLegale: Option<String>,
    changementNomUsageUniteLegale: bool,
    denominationUniteLegale: Option<String>,
    changementDenominationUniteLegale: bool,
    denominationUsuelle1UniteLegale: Option<String>,
    denominationUsuelle2UniteLegale: Option<String>,
    denominationUsuelle3UniteLegale: Option<String>,
    changementDenominationUsuelleUniteLegale: bool,
    categorieJuridiqueUniteLegale: String,
    changementCategorieJuridiqueUniteLegale: bool,
    activitePrincipaleUniteLegale: Option<String>,
    nomenclatureActivitePrincipaleUniteLegale: Option<String>,
    changementActivitePrincipaleUniteLegale: bool,
    nicSiegeUniteLegale: String,
    changementNicSiegeUniteLegale: bool,
    economieSocialeSolidaireUniteLegale: Option<String>,
    changementEconomieSocialeSolidaireUniteLegale: bool,
    caractereEmployeurUniteLegale: Option<String>,
    changementCaractereEmployeurUniteLegale: bool,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct UniteLegale {
    siren: String,
    statutDiffusionUniteLegale: String,

    #[serde(default = "default_as_false")]
    unitePurgeeUniteLegale: bool,

    dateCreationUniteLegale: String,
    sigleUniteLegale: Option<String>,
    sexeUniteLegale: Option<String>,
    prenom1UniteLegale: Option<String>,
    prenom2UniteLegale: Option<String>,
    prenom3UniteLegale: Option<String>,
    prenom4UniteLegale: Option<String>,
    prenomUsuelUniteLegale: Option<String>,
    pseudonymeUniteLegale: Option<String>,
    identifiantAssociationUniteLegale: Option<String>,
    trancheEffectifsUniteLegale: String,
    anneeEffectifsUniteLegale: Option<String>,
    dateDernierTraitementUniteLegale: String,
    nombrePeriodesUniteLegale: u32,
    categorieEntreprise: String,
    anneeCategorieEntreprise: String,
    periodesUniteLegale: Vec<PeriodeUniteLegale>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct InseeUniteLegaleResponse {
    header: Header,
    unitesLegales: Vec<UniteLegale>,
}

impl Connector {
    pub fn get_daily_unites_legales(&self) -> Result<String, InseeError> {
        let client = reqwest::blocking::Client::new();
        let response: InseeUniteLegaleResponse = client
            .get("https://api.insee.fr/entreprises/sirene/V3/siren?q=dateDernierTraitementUniteLegale:[2020-06-04 TO *]&nombre=3&curseur=*")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/json")
            .send()?
            .error_for_status()?
            .json()?;

        println!("{:#?}", response);

        Ok(String::from("UniteLegale"))
    }

    pub fn get_daily_etablissements(&self) -> Result<String, InseeError> {
        Ok(String::from("Etablissement"))
    }
}
