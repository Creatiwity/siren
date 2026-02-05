use super::super::schema::unite_legale;
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Insertable, Queryable, ToSchema, Serialize, Clone, Debug)]
#[diesel(table_name = unite_legale)]
pub struct UniteLegale {
    pub siren: String,
    pub statut_diffusion: String,
    pub unite_purgee: Option<String>,
    pub date_creation: Option<NaiveDate>,
    pub sigle: Option<String>,
    pub sexe: Option<String>,
    pub prenom_1: Option<String>,
    pub prenom_2: Option<String>,
    pub prenom_3: Option<String>,
    pub prenom_4: Option<String>,
    pub prenom_usuel: Option<String>,
    pub pseudonyme: Option<String>,
    pub identifiant_association: Option<String>,
    pub tranche_effectifs: Option<String>,
    pub annee_effectifs: Option<i32>,
    pub date_dernier_traitement: Option<NaiveDateTime>,
    pub nombre_periodes: Option<i32>,
    pub categorie_entreprise: Option<String>,
    pub annee_categorie_entreprise: Option<i32>,
    pub date_debut: Option<NaiveDate>,
    pub etat_administratif: String,
    pub nom: Option<String>,
    pub nom_usage: Option<String>,
    pub denomination: Option<String>,
    pub denomination_usuelle_1: Option<String>,
    pub denomination_usuelle_2: Option<String>,
    pub denomination_usuelle_3: Option<String>,
    pub categorie_juridique: Option<String>,
    pub activite_principale: Option<String>,
    pub nomenclature_activite_principale: Option<String>,
    pub nic_siege: Option<String>,
    pub economie_sociale_solidaire: Option<String>,
    pub societe_mission: Option<String>,
    pub caractere_employeur: Option<String>,
    pub activite_principale_naf25: Option<String>,
}
