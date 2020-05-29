use custom_error::custom_error;
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use std::env;

custom_error! { pub Error
    TokenNetworkError { source: reqwest::Error } = "Unable to retrieve INSEE token (network error: {source})",
    TokenApiError = "Unable to retrieve INSEE token",
    TokenMalformedError {source: serde_json::Error} = "Unable to read INSEE token ({source})",
}

pub struct Connector {
    token: String,
}

pub struct ConnectorBuilder {
    pub credentials: String,
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
            .body("grant_type=client_credentials&validity_period=86400")
            .send()?
            .json()?;

        println!("{:?}", response.access_token.clone());
        Ok(response.access_token)
    }
}

impl Connector {
    pub fn get_daily_unites_legales(&self) {}
    pub fn get_daily_etablissements(&self) {}
}
