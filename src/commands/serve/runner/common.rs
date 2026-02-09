use crate::connectors::ConnectorsBuilders;
use crate::models::etablissement::common::Etablissement;
use crate::models::lien_succession::common::LienSuccession;
use crate::models::unite_legale::common::UniteLegale;
use crate::models::update_metadata::common::SyntheticGroupType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const ADMIN_TAG: &str = "admin";
pub const PUBLIC_TAG: &str = "public";

#[derive(Clone, Debug)]
pub struct Context {
    pub builders: ConnectorsBuilders,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(ToSchema, Deserialize)]
pub struct UpdateOptions {
    pub api_key: String,
    pub group_type: SyntheticGroupType,
    pub force: bool,
    pub asynchronous: bool,
}

#[derive(ToSchema, Deserialize)]
pub struct StatusQueryString {
    pub api_key: String,
}

#[derive(ToSchema, Serialize)]
pub struct UniteLegaleResponse {
    pub unite_legale: UniteLegaleInnerResponse,
}

#[derive(ToSchema, Serialize)]
pub struct MetadataResponse {
    pub launched_timestamp: Option<DateTime<Utc>>,
    pub finished_timestamp: Option<DateTime<Utc>>,
}

#[derive(ToSchema, Serialize)]
pub struct UniteLegaleInnerResponse {
    #[serde(flatten)]
    pub unite_legale: UniteLegale,
    pub etablissements: Vec<Etablissement>,
    pub etablissement_siege: Etablissement,
}

#[derive(ToSchema, Serialize)]
pub struct EtablissementResponse {
    pub etablissement: EtablissementInnerResponse,
}

#[derive(ToSchema, Serialize)]
pub struct EtablissementInnerResponse {
    #[serde(flatten)]
    pub etablissement: Etablissement,
    pub unite_legale: UniteLegaleEtablissementInnerResponse,
}

#[derive(ToSchema, Serialize)]
pub struct UniteLegaleEtablissementInnerResponse {
    #[serde(flatten)]
    pub unite_legale: UniteLegale,
    pub etablissement_siege: Etablissement,
}

#[derive(ToSchema, Serialize)]
pub struct LiensSuccessionResponse {
    pub liens_succession: Vec<LienSuccession>,
}
