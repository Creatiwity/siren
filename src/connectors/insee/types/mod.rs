pub mod etablissement;
pub mod unite_legale;

use serde::{de::DeserializeOwned, Deserialize};
use std::fmt::Display;
use std::str::FromStr;

pub trait InseeResponse: DeserializeOwned {
    fn header(&self) -> Header;
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    total: u32,
    debut: u32,
    nombre: u32,
    pub curseur: String,
    pub curseur_suivant: String,
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
            println!("string expected but found something else: {}", v);
            Ok(None)
        }
        Err(_) => Ok(None),
    }
}

fn default_as_false() -> bool {
    false
}
