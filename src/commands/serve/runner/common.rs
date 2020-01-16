use super::super::super::common::{CmdGroupType, FolderOptions};
use crate::connectors::Connectors;
use crate::models::etablissement::common::Etablissement;
use crate::models::unite_legale::common::UniteLegale;
use serde::{Deserialize, Serialize};

pub struct Context {
    pub connectors: Connectors,
    pub folder_options: FolderOptions,
}

#[derive(Deserialize)]
pub struct UpdateOptions {
    pub group_type: CmdGroupType,
}

#[derive(Serialize)]
pub struct UpdateResponse {}

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
