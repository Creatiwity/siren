use crate::connectors::Connectors;
use crate::models::etablissement::common::Etablissement;
use crate::models::unite_legale::common::UniteLegale;
use serde::Serialize;

pub struct Context {
    pub connectors: Connectors,
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
