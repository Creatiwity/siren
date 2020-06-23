use super::error::InseeUpdateError;
use super::types::{
    etablissement::InseeEtablissementResponse, unite_legale::InseeUniteLegaleResponse,
    InseeQueryParams, InseeResponse,
};
use super::Connector;
use crate::models::etablissement::common::Etablissement;
use crate::models::unite_legale::common::UniteLegale;
use chrono::{Duration, NaiveDateTime, Utc};
use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION};

const MAX_CALL: u8 = 20;
const MAX_DURATION: std::time::Duration = std::time::Duration::from_secs(60);
const BASE_URL: &str = "https://api.insee.fr/entreprises/sirene/V3";
pub const INITIAL_CURSOR: &str = "*";

struct EndpointConfig {
    token: String,
    route: String,
    query_field: String,
}

impl Connector {
    async fn wait_for_insee_limitation(&mut self) {
        if self.calls == 0 {
            self.started_at = std::time::Instant::now();
        }

        self.calls += 1;

        let elapsed = self.started_at.elapsed();
        if self.calls > MAX_CALL && elapsed < MAX_DURATION {
            tokio::time::delay_for(MAX_DURATION).await;
            self.calls = 0;
        } else if elapsed >= MAX_DURATION {
            self.calls = 0;
        }
    }

    pub async fn get_daily_unites_legales(
        &mut self,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, Vec<UniteLegale>), InseeUpdateError> {
        self.wait_for_insee_limitation().await;

        let (next_cursor, response) = get_daily_data::<InseeUniteLegaleResponse>(
            EndpointConfig {
                token: self.token.clone(),
                route: String::from("siren"),
                query_field: String::from("dateDernierTraitementUniteLegale"),
            },
            start_timestamp,
            cursor,
        )
        .await?;

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

    pub async fn get_daily_etablissements(
        &mut self,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, Vec<Etablissement>), InseeUpdateError> {
        self.wait_for_insee_limitation().await;

        let (next_cursor, response) = get_daily_data::<InseeEtablissementResponse>(
            EndpointConfig {
                token: self.token.clone(),
                route: String::from("siret"),
                query_field: String::from("dateDernierTraitementEtablissement"),
            },
            start_timestamp,
            cursor,
        )
        .await?;

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

async fn get_daily_data<T: InseeResponse>(
    config: EndpointConfig,
    start_timestamp: NaiveDateTime,
    cursor: String,
) -> Result<(Option<String>, Option<T>), InseeUpdateError> {
    let client = reqwest::Client::new();

    let url = format!("{}/{}", BASE_URL, config.route);

    let response = match client
        .get(&url)
        .header(AUTHORIZATION, format!("Bearer {}", config.token))
        .header(ACCEPT, HeaderValue::from_static("application/json"))
        .query(&InseeQueryParams {
            q: format!(
                "{}:[{} TO *]",
                config.query_field,
                get_minimum_timestamp_for_request(start_timestamp).format("%Y-%m-%dT%H:%M:%S")
            ),
            nombre: 1000,
            curseur: cursor,
            tri: format!("{} asc", config.query_field),
        })
        .send()
        .await?
        .error_for_status()
    {
        Ok(response) => response.json::<T>().await.map_err(|error| error.into()),
        Err(error) => {
            // Insee returns 404 for empty data
            if let Some(status) = error.status() {
                if status == reqwest::StatusCode::NOT_FOUND {
                    return Ok((None, None));
                }
            }

            Err(error)
        }
    }?;

    let header = response.header();
    let next_cursor = if header.curseur == header.curseur_suivant {
        None
    } else {
        Some(header.curseur_suivant)
    };

    Ok((next_cursor, Some(response)))
}
