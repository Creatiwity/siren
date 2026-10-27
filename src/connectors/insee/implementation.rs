use super::Connector;
use super::error::InseeUpdate;
use super::types::{
    InseeCountQueryParams, InseeCountResponse, InseeQueryParams, InseeResponse,
    etablissement::InseeEtablissementResponse, unite_legale::InseeUniteLegaleResponse,
};
use crate::connectors::insee::types::lien_succession::InseeLienSuccessionResponse;
use crate::models::etablissement::common::EtablissementInsertable;
use crate::models::lien_succession::common::LienSuccession;
use crate::models::unite_legale::common::UniteLegaleInsertable;
use chrono::NaiveDateTime;
use reqwest::StatusCode;
use std::time::Duration;
use tracing::warn;

const MAX_CALL: u8 = 20;
const MAX_DURATION: std::time::Duration = std::time::Duration::from_secs(60);
const MAX_RETRIES: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(10);
const BASE_URL: &str = "https://api.insee.fr/api-sirene/3.11";
pub const INITIAL_CURSOR: &str = "*";

struct EndpointConfig {
    route: &'static str,
    query_field: &'static str,
    sorting: bool,
}

const UNITES_LEGALES_ENDPOINT: EndpointConfig = EndpointConfig {
    route: "siren",
    query_field: "dateDernierTraitementUniteLegale",
    sorting: true,
};

const ETABLISSEMENTS_ENDPOINT: EndpointConfig = EndpointConfig {
    route: "siret",
    query_field: "dateDernierTraitementEtablissement",
    sorting: true,
};

const LIENS_SUCCESSION_ENDPOINT: EndpointConfig = EndpointConfig {
    route: "siret/liensSuccession",
    query_field: "dateDernierTraitementLienSuccession",
    sorting: false,
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
    ) -> Result<u32, InseeUpdate> {
        self.wait_for_insee_limitation().await;

        get_total(&self.client, &UNITES_LEGALES_ENDPOINT, start_timestamp).await
    }

    pub async fn get_total_etablissements(
        &mut self,
        start_timestamp: NaiveDateTime,
    ) -> Result<u32, InseeUpdate> {
        self.wait_for_insee_limitation().await;

        get_total(&self.client, &ETABLISSEMENTS_ENDPOINT, start_timestamp).await
    }

    pub async fn get_total_liens_succession(
        &mut self,
        start_timestamp: NaiveDateTime,
    ) -> Result<u32, InseeUpdate> {
        self.wait_for_insee_limitation().await;

        get_total(&self.client, &LIENS_SUCCESSION_ENDPOINT, start_timestamp).await
    }

    pub async fn get_daily_unites_legales(
        &mut self,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, Vec<UniteLegaleInsertable>), InseeUpdate> {
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
    ) -> Result<(Option<String>, Vec<EtablissementInsertable>), InseeUpdate> {
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

    pub async fn get_daily_liens_succession(
        &mut self,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, Vec<LienSuccession>), InseeUpdate> {
        self.wait_for_insee_limitation().await;

        let (next_cursor, response) = get_daily_data::<InseeLienSuccessionResponse>(
            &self.client,
            &LIENS_SUCCESSION_ENDPOINT,
            start_timestamp,
            cursor,
        )
        .await?;

        Ok((
            next_cursor,
            match response {
                Some(resp) => resp.liens_succession.iter().map(|u| u.into()).collect(),
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
) -> Result<(Option<String>, Option<T>), InseeUpdate> {
    with_retry(RETRY_DELAY, || {
        fetch_daily_data::<T>(client, config, start_timestamp, &cursor)
    })
    .await
}

async fn fetch_daily_data<T: InseeResponse>(
    client: &reqwest::Client,
    config: &EndpointConfig,
    start_timestamp: NaiveDateTime,
    cursor: &str,
) -> Result<(Option<String>, Option<T>), InseeUpdate> {
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
            curseur: cursor.to_string(),
            tri: config
                .sorting
                .then_some(format!("{} asc", config.query_field)),
        })
        .send()
        .await?
        .error_for_status()
    {
        Ok(response) => response.json::<T>().await,
        Err(error) => {
            // Insee returns 404 for empty data
            if let Some(status) = error.status()
                && status == reqwest::StatusCode::NOT_FOUND
            {
                return Ok((None, None));
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
) -> Result<u32, InseeUpdate> {
    with_retry(RETRY_DELAY, || fetch_total(client, config, start_timestamp)).await
}

async fn fetch_total(
    client: &reqwest::Client,
    config: &EndpointConfig,
    start_timestamp: NaiveDateTime,
) -> Result<u32, InseeUpdate> {
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

fn is_transient(error: &reqwest::Error) -> bool {
    error
        .status()
        .is_none_or(|status| status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS)
}

async fn with_retry<T, F, Fut>(delay: Duration, mut call: F) -> Result<T, InseeUpdate>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, InseeUpdate>>,
{
    let mut attempt = 0;

    loop {
        match call().await {
            Err(InseeUpdate::Network { source })
                if attempt < MAX_RETRIES && is_transient(&source) =>
            {
                attempt += 1;
                warn!("INSEE request failed ({source}), retry {attempt}/{MAX_RETRIES}");
                tokio::time::sleep(delay * attempt).await;
            }
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::time::Duration;

    async fn connection_refused() -> reqwest::Error {
        reqwest::Client::new()
            .get("http://127.0.0.1:9/")
            .send()
            .await
            .expect_err("nothing listens on port 9")
    }

    #[tokio::test]
    async fn with_retry_recovers_from_transient_network_errors() {
        let calls = Cell::new(0);

        let result = with_retry(Duration::ZERO, || async {
            calls.set(calls.get() + 1);
            if calls.get() < 3 {
                Err(InseeUpdate::Network {
                    source: connection_refused().await,
                })
            } else {
                Ok(calls.get())
            }
        })
        .await;

        assert_eq!(result.unwrap(), 3);
    }

    #[tokio::test]
    async fn with_retry_gives_up_after_max_retries() {
        let calls = Cell::new(0);

        let result: Result<(), InseeUpdate> = with_retry(Duration::ZERO, || async {
            calls.set(calls.get() + 1);
            Err(InseeUpdate::Network {
                source: connection_refused().await,
            })
        })
        .await;

        assert!(result.is_err());
        assert_eq!(calls.get(), MAX_RETRIES + 1);
    }
}
