use crate::models::unite_legale::common::UniteLegale;
use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;
use std::fmt::Display;
use std::str::FromStr;

fn default_as_false() -> bool {
    false
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InseeUniteLegaleResponse {
    pub header: Header,
    pub unites_legales: Vec<InseeUniteLegale>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    total: u32,
    debut: u32,
    nombre: u32,
    pub curseur: String,
    pub curseur_suivant: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InseeUniteLegaleInner {
    pub siren: String,
    pub statut_diffusion_unite_legale: String,

    #[serde(default = "default_as_false")]
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

    #[serde(deserialize_with = "from_str_optional")]
    pub annee_effectifs_unite_legale: Option<i32>,

    pub date_dernier_traitement_unite_legale: Option<NaiveDateTime>,
    pub nombre_periodes_unite_legale: Option<i32>,
    pub categorie_entreprise: Option<String>,

    #[serde(deserialize_with = "from_str_optional")]
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

    pub changement_etat_administratif_unite_legale: bool,
    pub nom_unite_legale: Option<String>,
    pub changement_nom_unite_legale: bool,
    pub nom_usage_unite_legale: Option<String>,
    pub changement_nom_usage_unite_legale: bool,
    pub denomination_unite_legale: Option<String>,
    pub changement_denomination_unite_legale: bool,
    pub denomination_usuelle1_unite_legale: Option<String>,
    pub denomination_usuelle2_unite_legale: Option<String>,
    pub denomination_usuelle3_unite_legale: Option<String>,
    pub changement_denomination_usuelle_unite_legale: bool,
    pub categorie_juridique_unite_legale: Option<String>,
    pub changement_categorie_juridique_unite_legale: bool,
    pub activite_principale_unite_legale: Option<String>,
    pub nomenclature_activite_principale_unite_legale: Option<String>,
    pub changement_activite_principale_unite_legale: bool,
    pub nic_siege_unite_legale: Option<String>,
    pub changement_nic_siege_unite_legale: bool,
    pub economie_sociale_solidaire_unite_legale: Option<String>,
    pub changement_economie_sociale_solidaire_unite_legale: bool,
    pub caractere_employeur_unite_legale: Option<String>,
    pub changement_caractere_employeur_unite_legale: bool,
}

impl From<&InseeUniteLegale> for Option<UniteLegale> {
    fn from(u: &InseeUniteLegale) -> Self {
        match u
            .periodes_unite_legale
            .iter()
            .find(|p| p.date_fin.is_none())
        {
            Some(periode) => {
                // Convert
                Some(
                    InseeUniteLegaleWithPeriode {
                        content: u.content.clone(),
                        periode: periode.clone(),
                    }
                    .into(),
                )
            }
            None => None,
        }
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
            caractere_employeur: u.periode.caractere_employeur_unite_legale,
        }
    }
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

fn deserialize_etat_administratif<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or(String::from("C")))
}
