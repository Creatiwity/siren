use super::super::super::common::FolderOptions;
use crate::connectors::ConnectorsBuilders;
use crate::models::etablissement::common::Etablissement;
use crate::models::unite_legale::common::UniteLegale;
use crate::models::update_metadata::common::{SyntheticGroupType, UpdateSummary};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Context {
    pub builders: ConnectorsBuilders,
    pub api_key: Option<String>,
    pub folder_options: FolderOptions,
    pub base_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateOptions {
    pub api_key: String,
    pub group_type: SyntheticGroupType,
    pub force: bool,
    pub asynchronous: bool,
}

#[derive(Deserialize)]
pub struct StatusQueryString {
    pub api_key: String,
}

#[derive(Serialize)]
pub struct UpdateResponse {
    pub summary: UpdateSummary,
}

#[derive(Serialize)]
pub struct UniteLegaleResponse {
    pub unite_legale: UniteLegaleInnerResponse,
}

#[derive(Serialize)]
pub struct UniteLegaleInnerResponse {
    #[serde(flatten)]
    pub unite_legale: UniteLegale,
    pub etablissements: Vec<Etablissement>,
    pub etablissement_siege: Etablissement,
}

#[derive(Serialize)]
pub struct EtablissementResponse {
    pub etablissement: EtablissementInnerResponse,
}

#[derive(Serialize)]
pub struct EtablissementInnerResponse {
    #[serde(flatten)]
    pub etablissement: Etablissement,
    pub unite_legale: UniteLegaleEtablissementInnerResponse,
}

#[derive(Serialize)]
pub struct UniteLegaleEtablissementInnerResponse {
    #[serde(flatten)]
    pub unite_legale: UniteLegale,
    pub etablissement_siege: Etablissement,
}
