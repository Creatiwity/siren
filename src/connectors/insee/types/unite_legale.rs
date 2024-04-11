use super::{Header, InseeResponse};
use crate::models::unite_legale::common::UniteLegale;
use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InseeUniteLegaleResponse {
    pub header: Header,
    pub unites_legales: Vec<InseeUniteLegale>,
}

impl InseeResponse for InseeUniteLegaleResponse {
    fn header(&self) -> Header {
        self.header.clone()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InseeUniteLegaleInner {
    pub siren: String,
    pub statut_diffusion_unite_legale: String,

    #[serde(default = "super::default_as_false")]
    pub unite_purgee_unite_legale: bool,

    pub date_creation_unite_legale: Option<NaiveDate>,
    pub sigle_unite_legale: Option<String>,
    pub sexe_unite_legale: Option<String>,
    pub prenom1_unite_legale: Option<String>,
    pub prenom2_unite_legale: Option<String>,
    pub prenom3_unite_legale: Option<String>,
    pub prenom4_unite_legale: Option<String>,
    pub prenom_usuel_unite_legale: Option<String>,
    pub pseudonyme_unite_legale: Option<String>,
    pub identifiant_association_unite_legale: Option<String>,
    pub tranche_effectifs_unite_legale: Option<String>,

    #[serde(deserialize_with = "super::from_str_optional")]
    pub annee_effectifs_unite_legale: Option<i32>,

    pub date_dernier_traitement_unite_legale: Option<NaiveDateTime>,
    pub nombre_periodes_unite_legale: Option<i32>,
    pub categorie_entreprise: Option<String>,

    #[serde(deserialize_with = "super::from_str_optional")]
    pub annee_categorie_entreprise: Option<i32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InseeUniteLegale {
    #[serde(flatten)]
    pub content: InseeUniteLegaleInner,
    pub periodes_unite_legale: Vec<PeriodeInseeUniteLegale>,
}

#[derive(Debug)]
pub struct InseeUniteLegaleWithPeriode {
    pub content: InseeUniteLegaleInner,
    pub periode: PeriodeInseeUniteLegale,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PeriodeInseeUniteLegale {
    pub date_fin: Option<NaiveDate>,
    pub date_debut: Option<NaiveDate>,

    #[serde(deserialize_with = "deserialize_etat_administratif")]
    pub etat_administratif_unite_legale: String,

    pub nom_unite_legale: Option<String>,
    pub nom_usage_unite_legale: Option<String>,
    pub denomination_unite_legale: Option<String>,
    pub denomination_usuelle1_unite_legale: Option<String>,
    pub denomination_usuelle2_unite_legale: Option<String>,
    pub denomination_usuelle3_unite_legale: Option<String>,
    pub categorie_juridique_unite_legale: Option<String>,
    pub activite_principale_unite_legale: Option<String>,
    pub nomenclature_activite_principale_unite_legale: Option<String>,
    pub nic_siege_unite_legale: Option<String>,
    pub economie_sociale_solidaire_unite_legale: Option<String>,
    pub societe_mission_unite_legale: Option<String>,
}

impl From<&InseeUniteLegale> for Option<UniteLegale> {
    fn from(u: &InseeUniteLegale) -> Self {
        u.periodes_unite_legale
            .iter()
            .find(|p| p.date_fin.is_none())
            .map(|periode| {
                InseeUniteLegaleWithPeriode {
                    content: u.content.clone(),
                    periode: periode.clone(),
                }
                .into()
            })
    }
}

impl From<InseeUniteLegaleWithPeriode> for UniteLegale {
    fn from(u: InseeUniteLegaleWithPeriode) -> Self {
        UniteLegale {
            siren: u.content.siren,
            statut_diffusion: u.content.statut_diffusion_unite_legale,
            unite_purgee: Some(u.content.unite_purgee_unite_legale.to_string()),
            date_creation: u.content.date_creation_unite_legale,
            sigle: u.content.sigle_unite_legale,
            sexe: u.content.sexe_unite_legale,
            prenom_1: u.content.prenom1_unite_legale,
            prenom_2: u.content.prenom2_unite_legale,
            prenom_3: u.content.prenom3_unite_legale,
            prenom_4: u.content.prenom4_unite_legale,
            prenom_usuel: u.content.prenom_usuel_unite_legale,
            pseudonyme: u.content.pseudonyme_unite_legale,
            identifiant_association: u.content.identifiant_association_unite_legale,
            tranche_effectifs: u.content.tranche_effectifs_unite_legale,
            annee_effectifs: u.content.annee_effectifs_unite_legale,
            date_dernier_traitement: u.content.date_dernier_traitement_unite_legale,
            nombre_periodes: u.content.nombre_periodes_unite_legale,
            categorie_entreprise: u.content.categorie_entreprise,
            annee_categorie_entreprise: u.content.annee_categorie_entreprise,
            date_debut: u.periode.date_debut,
            etat_administratif: u.periode.etat_administratif_unite_legale,
            nom: u.periode.nom_unite_legale,
            nom_usage: u.periode.nom_usage_unite_legale,
            denomination: u.periode.denomination_unite_legale,
            denomination_usuelle_1: u.periode.denomination_usuelle1_unite_legale,
            denomination_usuelle_2: u.periode.denomination_usuelle2_unite_legale,
            denomination_usuelle_3: u.periode.denomination_usuelle3_unite_legale,
            categorie_juridique: u.periode.categorie_juridique_unite_legale,
            activite_principale: u.periode.activite_principale_unite_legale,
            nomenclature_activite_principale: u
                .periode
                .nomenclature_activite_principale_unite_legale,
            nic_siege: u.periode.nic_siege_unite_legale,
            economie_sociale_solidaire: u.periode.economie_sociale_solidaire_unite_legale,
            societe_mission: u.periode.societe_mission_unite_legale,
            caractere_employeur: None,
        }
    }
}

fn deserialize_etat_administratif<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_else(|| String::from("C")))
}
