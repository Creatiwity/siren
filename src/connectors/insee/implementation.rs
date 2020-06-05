use super::error::InseeError;
use super::types::InseeUniteLegaleResponse;
use super::Connector;
use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION};

impl Connector {
    pub fn get_daily_unites_legales(&self) -> Result<String, InseeError> {
        let client = reqwest::blocking::Client::new();
        let response: InseeUniteLegaleResponse = client
            .get("https://api.insee.fr/entreprises/sirene/V3/siren?q=dateDernierTraitementUniteLegale:[2020-06-04 TO *]&nombre=3&curseur=*")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, HeaderValue::from_static("application/json"))
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
