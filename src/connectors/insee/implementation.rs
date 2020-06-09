use super::error::InseeError;
use super::types::InseeUniteLegaleResponse;
use super::Connector;
use crate::models::unite_legale::common::UniteLegale as DbUniteLegale;
use chrono::{NaiveDate, NaiveDateTime};
use reqwest::header::{HeaderValue, ACCEPT, AUTHORIZATION};

const BASE_URL: &str = "https://api.insee.fr/entreprises/sirene/V3";

impl Connector {
    pub fn get_daily_unites_legales(&self) -> Result<Vec<DbUniteLegale>, InseeError> {
        let unites_legales: Vec<DbUniteLegale> = vec![];

        let client = reqwest::blocking::Client::new();
        let mut has_data = true;
        let mut current_cursor = String::from("*");

        while has_data {
            let url = format!("{}/siren?q=dateDernierTraitementUniteLegale:[2020-06-08 TO *]&nombre=1000&curseur={}", BASE_URL, current_cursor);

            let response: InseeUniteLegaleResponse = client
                .get(&url)
                .header(AUTHORIZATION, format!("Bearer {}", self.token))
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .send()?
                .error_for_status()?
                .json()?;

            println!("{:#?}", response);

            for u in response.unites_legales.iter() {
                let current_periode = u
                    .periodes_unite_legale
                    .iter()
                    .find(|p| p.date_fin.is_none())
                    .ok_or_else(|| InseeError::MissingPeriodeError)?;

                unites_legales.push(DbUniteLegale {
                    siren: u.siren.clone(),
                    statut_diffusion: u.statut_diffusion_unite_legale.clone(),
                    unite_purgee: Some(u.unite_purgee_unite_legale.to_string()),

                    date_creation: NaiveDate::parse_from_str(
                        u.date_creation_unite_legale.as_str(),
                        "%Y-%m-%d",
                    )
                    .map_or(None, |v| Some(v)),

                    sigle: u.sigle_unite_legale.clone(),
                    sexe: u.sexe_unite_legale.clone(),
                    prenom_1: u.prenom1_unite_legale.clone(),
                    prenom_2: u.prenom2_unite_legale.clone(),
                    prenom_3: u.prenom3_unite_legale.clone(),
                    prenom_4: u.prenom4_unite_legale.clone(),
                    prenom_usuel: u.prenom_usuel_unite_legale.clone(),
                    pseudonyme: u.pseudonyme_unite_legale.clone(),
                    identifiant_association: u.identifiant_association_unite_legale.clone(),
                    tranche_effectifs: Some(u.tranche_effectifs_unite_legale.clone()),

                    annee_effectifs: u
                        .annee_effectifs_unite_legale
                        .clone()
                        .map(|annee| annee.parse::<i32>())
                        .map_or(None, |v| v.map_or(None, |v| Some(v))),

                    date_dernier_traitement: NaiveDateTime::parse_from_str(
                        u.date_dernier_traitement_unite_legale.as_str(),
                        "%Y-%m-%dT%H:%M:%S",
                    )
                    .map_or(None, |v| Some(v)),

                    nombre_periodes: Some(u.nombre_periodes_unite_legale),
                    categorie_entreprise: Some(u.categorie_entreprise.clone()),

                    annee_categorie_entreprise: u
                        .annee_categorie_entreprise
                        .parse::<i32>()
                        .map_or(None, |v| Some(v)),

                    date_debut: NaiveDate::parse_from_str(
                        current_periode.date_debut.as_str(),
                        "%Y-%m-%d",
                    )
                    .map_or(None, |v| Some(v)),

                    etat_administratif: current_periode.etat_administratif_unite_legale.clone(),
                    nom: current_periode.nom_unite_legale.clone(),
                    nom_usage: current_periode.nom_usage_unite_legale.clone(),
                    denomination: current_periode.denomination_unite_legale.clone(),

                    denomination_usuelle_1: current_periode
                        .denomination_usuelle1_unite_legale
                        .clone(),

                    denomination_usuelle_2: current_periode
                        .denomination_usuelle2_unite_legale
                        .clone(),

                    denomination_usuelle_3: current_periode
                        .denomination_usuelle3_unite_legale
                        .clone(),

                    categorie_juridique: Some(
                        current_periode.categorie_juridique_unite_legale.clone(),
                    ),

                    activite_principale: current_periode.activite_principale_unite_legale.clone(),

                    nomenclature_activite_principale: current_periode
                        .nomenclature_activite_principale_unite_legale
                        .clone(),

                    nic_siege: Some(current_periode.nic_siege_unite_legale.clone()),
                    economie_sociale_solidaire: current_periode
                        .economie_sociale_solidaire_unite_legale
                        .clone(),

                    caractere_employeur: current_periode.caractere_employeur_unite_legale.clone(),
                });
            }

            has_data = response.header.curseur != response.header.curseur_suivant;
            current_cursor = response.header.curseur_suivant;
        }

        Ok(unites_legales)
    }

    pub fn get_daily_etablissements(&self) -> Result<String, InseeError> {
        Ok(String::from("Etablissement"))
    }
}
