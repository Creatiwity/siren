mod implementation;
mod types;

pub mod error;

use error::InseeTokenError;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use std::env;
use std::time::{Duration, Instant};

pub use implementation::INITIAL_CURSOR;

#[derive(Clone)]
pub struct Connector {
    client: reqwest::Client,
    calls: u8,
    started_at: Instant,
}

#[derive(Clone, Debug)]
pub struct ConnectorBuilder {
    pub credentials: String,
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

    pub fn create(&self) -> Result<Connector, InseeTokenError> {
        // Build headers
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-INSEE-Api-Key-Integration",
            HeaderValue::from_str(self.credentials.as_str())?,
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
    }
}
