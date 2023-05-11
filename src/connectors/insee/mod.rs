mod implementation;
mod types;

pub mod error;

use error::InseeTokenError;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{Duration, Instant};

pub use implementation::INITIAL_CURSOR;

#[derive(Clone)]
pub struct Connector {
    client: reqwest::Client,
    calls: u8,
    started_at: Instant,
}

#[derive(Clone)]
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
}

impl ConnectorBuilder {
    pub fn new() -> Option<ConnectorBuilder> {
        env::var("INSEE_CREDENTIALS")
            .ok()
            .and_then(|credentials| match credentials.len() {
                0 => None,
                _ => Some(ConnectorBuilder { credentials }),
            })
    }

    pub async fn create(&self) -> Result<Connector, InseeTokenError> {
        self.generate_token().await.and_then(|token| {
            // Build headers
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(format!("Bearer {token}").as_str())?,
            );
            headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

            // Build client
            let client = reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(20))
                .default_headers(headers)
                .build()?;

            Ok(Connector {
                client,
                calls: 0,
                started_at: Instant::now(),
            })
        })
    }

    async fn generate_token(&self) -> Result<String, InseeTokenError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .build()?;

        let response = client
            .post("https://api.insee.fr/token")
            .header(AUTHORIZATION, format!("Basic {}", self.credentials))
            .form(&InseeTokenParameters {
                grant_type: String::from("client_credentials"),
                validity_period: 86400,
            })
            .send()
            .await?
            .json::<InseeTokenResponse>()
            .await?;

        Ok(response.access_token)
    }
}
