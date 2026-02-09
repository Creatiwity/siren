use super::{Header, InseeResponse};
use crate::models::lien_succession::common::LienSuccession;
use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InseeLienSuccessionResponse {
    pub header: Header,
    pub liens_succession: Vec<InseeLienSuccession>,
}

impl InseeResponse for InseeLienSuccessionResponse {
    fn header(&self) -> Header {
        self.header.clone()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InseeLienSuccession {
    pub siret_etablissement_predecesseur: String,
    pub siret_etablissement_successeur: String,
    pub date_lien_succession: NaiveDate,
    pub transfert_siege: bool,
    pub continuite_economique: bool,
    pub date_dernier_traitement_lien_succession: Option<NaiveDateTime>,
}

impl From<&InseeLienSuccession> for LienSuccession {
    fn from(e: &InseeLienSuccession) -> Self {
        LienSuccession {
            siret_etablissement_predecesseur: e.siret_etablissement_predecesseur.clone(),
            siret_etablissement_successeur: e.siret_etablissement_successeur.clone(),
            date_lien_succession: e.date_lien_succession,
            transfert_siege: e.transfert_siege,
            continuite_economique: e.continuite_economique,
            date_dernier_traitement_lien_succession: e.date_dernier_traitement_lien_succession,
        }
    }
}
