pub mod etablissement;
pub mod lien_succession;
pub mod unite_legale;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fmt::Display;
use std::str::FromStr;
use tracing::error;

#[derive(Serialize)]
pub struct InseeQueryParams {
    pub q: String,
    pub nombre: u16,
    pub curseur: String,
    pub tri: Option<String>,
}

#[derive(Serialize)]
pub struct InseeCountQueryParams {
    pub q: String,
    pub nombre: u16,
    pub champs: &'static str,
}

pub trait InseeResponse: DeserializeOwned {
    fn header(&self) -> Header;
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub curseur: String,
    pub curseur_suivant: String,
}

#[derive(Deserialize, Debug)]
pub struct CountHeader {
    #[serde(default = "default_as_zero")]
    pub total: u32,
}

#[derive(Deserialize, Debug)]
pub struct InseeCountResponse {
    pub header: CountHeader,
}

fn from_str_optional<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: serde::Deserializer<'de>,
{
    let deser_res: Result<serde_json::Value, _> = serde::Deserialize::deserialize(deserializer);
    match deser_res {
        Ok(serde_json::Value::String(s)) => T::from_str(&s)
            .map_err(serde::de::Error::custom)
            .map(Option::from),
        Ok(serde_json::Value::Null) => Ok(None),
        Ok(v) => {
            error!("string expected but found something else: {}", v);
            Ok(None)
        }
        Err(_) => Ok(None),
    }
}

fn default_as_false() -> bool {
    false
}

fn default_as_zero() -> u32 {
    0
}
