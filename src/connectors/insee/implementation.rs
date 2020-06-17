use super::error::InseeUpdateError;
use super::types::{
    etablissement::InseeEtablissementResponse, unite_legale::InseeUniteLegaleResponse,
    InseeResponse,
};
use super::Connector;
use crate::models::etablissement::common::Etablissement;
use crate::models::unite_legale::common::UniteLegale;
use chrono::{Duration, NaiveDateTime, Utc};
use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION};

const BASE_URL: &str = "https://api.insee.fr/entreprises/sirene/V3";
pub const INITIAL_CURSOR: &str = "*";

struct EndpointConfig {
    token: String,
    route: String,
    query_field: String,
}

impl Connector {
    pub fn get_daily_unites_legales(
        &self,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, Vec<UniteLegale>), InseeUpdateError> {
        let (next_cursor, response) = get_daily_data::<InseeUniteLegaleResponse>(
            EndpointConfig {
                token: self.token.clone(),
                route: String::from("siren"),
                query_field: String::from("dateDernierTraitementUniteLegale"),
            },
            start_timestamp,
            cursor,
        )?;

        Ok((
            next_cursor,
            match response {
                Some(resp) => resp
                    .unites_legales
                    .iter()
                    .filter_map(|u| u.into())
                    .collect(),
                None => vec![],
            },
        ))
    }

    pub fn get_daily_etablissements(
        &self,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, Vec<Etablissement>), InseeUpdateError> {
        let (next_cursor, response) = get_daily_data::<InseeEtablissementResponse>(
            EndpointConfig {
                token: self.token.clone(),
                route: String::from("siret"),
                query_field: String::from("dateDernierTraitementEtablissement"),
            },
            start_timestamp,
            cursor,
        )?;

        Ok((
            next_cursor,
            match response {
                Some(resp) => resp
                    .etablissements
                    .iter()
                    .filter_map(|u| u.into())
                    .collect(),
                None => vec![],
            },
        ))
    }
}

fn get_minimum_timestamp_for_request(timestamp: NaiveDateTime) -> NaiveDateTime {
    timestamp.max(Utc::now().naive_local() - Duration::days(31))
}

fn get_daily_data<T: InseeResponse>(
    config: EndpointConfig,
    start_timestamp: NaiveDateTime,
    cursor: String,
) -> Result<(Option<String>, Option<T>), InseeUpdateError> {
    // let client = reqwest::blocking::Client::new();

    let url = format!(
        "{}/{}?q={}:%7B{} TO *]&nombre=1000&curseur={}",
        BASE_URL,
        config.route,
        config.query_field,
        get_minimum_timestamp_for_request(start_timestamp).format("%Y-%m-%dT%H:%M:%S"),
        cursor
    );

    // let response = match client
    //     .get(&url)
    //     .header(AUTHORIZATION, format!("Bearer {}", config.token))
    //     .header(ACCEPT, HeaderValue::from_static("application/json"))
    //     .send()?
    //     .error_for_status()
    // {
    //     Ok(response) => response.json::<T>().map_err(|error| error.into()),
    //     Err(error) => {
    //         // Insee returns 404 for empty data
    //         if let Some(status) = error.status() {
    //             if status == reqwest::StatusCode::NOT_FOUND {
    //                 return Ok((None, None));
    //             }
    //         }

    //         Err(error)
    //     }
    // }?;

    // let header = response.header();
    // let next_cursor = if header.curseur == header.curseur_suivant {
    //     None
    // } else {
    //     Some(header.curseur_suivant)
    // };

    Ok((None, None))
}
