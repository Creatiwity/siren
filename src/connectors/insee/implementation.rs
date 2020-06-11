use super::error::InseeUpdateError;
use super::types::InseeUniteLegaleResponse;
use super::Connector;
use crate::models::unite_legale::common::UniteLegale;
use chrono::{Duration, NaiveDateTime, Utc};
use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION};

const BASE_URL: &str = "https://api.insee.fr/entreprises/sirene/V3";
pub const INITIAL_CURSOR: &str = "*";

impl Connector {
    fn get_minimum_timestamp_for_request(&self, timestamp: NaiveDateTime) -> NaiveDateTime {
        timestamp.max(Utc::now().naive_local() - Duration::days(31))
    }

    pub fn get_daily_unites_legales(
        &self,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, Vec<UniteLegale>), InseeUpdateError> {
        let client = reqwest::blocking::Client::new();

        let url = format!(
            "{}/siren?q=dateDernierTraitementUniteLegale:%7B{} TO *]&nombre=1000&curseur={}",
            BASE_URL,
            self.get_minimum_timestamp_for_request(start_timestamp)
                .format("%Y-%m-%dT%H:%M:%S"),
            cursor
        );

        let response = match client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, HeaderValue::from_static("application/json"))
            .send()?
            .error_for_status()
        {
            Ok(response) => response
                .json::<InseeUniteLegaleResponse>()
                .map_err(|error| error.into()),
            Err(error) => {
                // Insee returns 404 for empty data
                if let Some(status) = error.status() {
                    if status == reqwest::StatusCode::NOT_FOUND {
                        return Ok((None, vec![]));
                    }
                }

                Err(error)
            }
        }?;

        let next_cursor = if response.header.curseur == response.header.curseur_suivant {
            None
        } else {
            Some(response.header.curseur_suivant)
        };

        Ok((
            next_cursor,
            response
                .unites_legales
                .iter()
                .filter_map(|u| u.into())
                .collect(),
        ))
    }

    pub fn get_daily_etablissements(&self) -> Result<String, InseeUpdateError> {
        Ok(String::from("Etablissement"))
    }
}
