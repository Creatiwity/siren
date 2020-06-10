use super::error::InseeUpdateError;
use super::types::InseeUniteLegaleResponse;
use super::Connector;
use crate::models::unite_legale::common::UniteLegale;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION};

const BASE_URL: &str = "https://api.insee.fr/entreprises/sirene/V3";

impl Connector {
    pub fn get_daily_unites_legales(
        &self,
        _start_timestamp: DateTime<Utc>,
    ) -> Result<Vec<UniteLegale>, InseeUpdateError> {
        let mut unites_legales: Vec<UniteLegale> = vec![];

        let client = reqwest::blocking::Client::new();
        let mut has_data = true;
        let mut current_cursor = String::from("*");

        while has_data {
            println!("Current cursor: {}", current_cursor);

            let url = format!(
                "{}/siren?q=dateDernierTraitementUniteLegale:[2020-06-09 TO *]&nombre=1000&curseur={}",
                BASE_URL, current_cursor
            );

            let response: InseeUniteLegaleResponse = client
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {}", self.token))
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .send()?
                .error_for_status()?
                .json()?;

            unites_legales.extend(response.unites_legales.iter().filter_map(|u| u.into()));

            has_data = response.header.curseur != response.header.curseur_suivant;
            current_cursor = response.header.curseur_suivant;
        }

        Ok(unites_legales)
    }

    pub fn get_daily_etablissements(&self) -> Result<String, InseeUpdateError> {
        Ok(String::from("Etablissement"))
    }
}
