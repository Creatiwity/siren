use super::error::InseeUpdateError;
use super::types::{
    etablissement::InseeEtablissementResponse, unite_legale::InseeUniteLegaleResponse,
    InseeCountQueryParams, InseeCountResponse, InseeQueryParams, InseeResponse,
};
use super::Connector;
use crate::models::etablissement::common::Etablissement;
use crate::models::unite_legale::common::UniteLegale;
use chrono::NaiveDateTime;

const MAX_CALL: u8 = 20;
const MAX_DURATION: std::time::Duration = std::time::Duration::from_secs(60);
const BASE_URL: &str = "https://api.insee.fr/entreprises/sirene/V3";
pub const INITIAL_CURSOR: &str = "*";

struct EndpointConfig {
    route: &'static str,
    query_field: &'static str,
}

const UNITES_LEGALES_ENDPOINT: EndpointConfig = EndpointConfig {
    route: "siren",
    query_field: "dateDernierTraitementUniteLegale",
};

const ETABLISSEMENTS_ENDPOINT: EndpointConfig = EndpointConfig {
    route: "siret",
    query_field: "dateDernierTraitementEtablissement",
};

impl Connector {
    async fn wait_for_insee_limitation(&mut self) {
        if self.calls == 0 {
            self.started_at = std::time::Instant::now();
        }

        self.calls += 1;

        let elapsed = self.started_at.elapsed();
        if self.calls > MAX_CALL && elapsed < MAX_DURATION {
            tokio::time::sleep(MAX_DURATION).await;
            self.calls = 0;
        } else if elapsed >= MAX_DURATION {
            self.calls = 0;
        }
    }

    pub async fn get_total_unites_legales(
        &mut self,
        start_timestamp: NaiveDateTime,
    ) -> Result<u32, InseeUpdateError> {
        self.wait_for_insee_limitation().await;

        get_total(&self.client, &UNITES_LEGALES_ENDPOINT, start_timestamp).await
    }

    pub async fn get_total_etablissements(
        &mut self,
        start_timestamp: NaiveDateTime,
    ) -> Result<u32, InseeUpdateError> {
        self.wait_for_insee_limitation().await;

        get_total(&self.client, &ETABLISSEMENTS_ENDPOINT, start_timestamp).await
    }

    pub async fn get_daily_unites_legales(
        &mut self,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, Vec<UniteLegale>), InseeUpdateError> {
        self.wait_for_insee_limitation().await;

        let (next_cursor, response) = get_daily_data::<InseeUniteLegaleResponse>(
            &self.client,
            &UNITES_LEGALES_ENDPOINT,
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
            &self.client,
            &ETABLISSEMENTS_ENDPOINT,
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

async fn get_daily_data<T: InseeResponse>(
    client: &reqwest::Client,
    config: &EndpointConfig,
    start_timestamp: NaiveDateTime,
    cursor: String,
) -> Result<(Option<String>, Option<T>), InseeUpdateError> {
    let url = format!("{}/{}", BASE_URL, config.route);

    let response = match client
        .get(&url)
        .query(&InseeQueryParams {
            q: format!(
                "{}:[{} TO *]",
                config.query_field,
                start_timestamp.format("%Y-%m-%dT%H:%M:%S")
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

async fn get_total(
    client: &reqwest::Client,
    config: &EndpointConfig,
    start_timestamp: NaiveDateTime,
) -> Result<u32, InseeUpdateError> {
    let url = format!("{}/{}", BASE_URL, config.route);

    let response = client
        .get(&url)
        .query(&InseeCountQueryParams {
            q: format!(
                "{}:[{} TO *]",
                config.query_field,
                start_timestamp.format("%Y-%m-%dT%H:%M:%S")
            ),
            nombre: 1,
            champs: config.route,
        })
        .send()
        .await?
        .json::<InseeCountResponse>()
        .await?;

    Ok(response.header.total)
}
