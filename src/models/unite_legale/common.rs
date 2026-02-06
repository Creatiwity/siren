use super::super::schema::unite_legale;
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Float4, Nullable, Text, VarChar};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

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
    pub search_denomination: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UniteLegaleSortField {
    DateCreation,
    DateDebut,
    Relevance,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub enum EtatAdministratif {
    A,
    F,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct UniteLegaleSearchParams {
    pub q: Option<String>,
    pub etat_administratif: Option<EtatAdministratif>,
    pub activite_principale: Option<String>,
    pub categorie_juridique: Option<String>,
    pub categorie_entreprise: Option<String>,
    pub date_creation: Option<NaiveDate>,
    pub date_debut: Option<NaiveDate>,
    pub sort: Option<UniteLegaleSortField>,
    pub direction: Option<SortDirection>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, QueryableByName, Serialize, ToSchema)]
pub struct UniteLegaleSearchResult {
    #[diesel(sql_type = VarChar)]
    pub siren: String,
    #[diesel(sql_type = VarChar)]
    pub etat_administratif: String,
    #[diesel(sql_type = Nullable<diesel::sql_types::Date>)]
    pub date_creation: Option<NaiveDate>,
    #[diesel(sql_type = Nullable<Text>)]
    pub denomination: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub denomination_usuelle_1: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub denomination_usuelle_2: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub denomination_usuelle_3: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub activite_principale: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub categorie_juridique: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub categorie_entreprise: Option<String>,
    #[diesel(sql_type = Nullable<Float4>)]
    pub score: Option<f32>,
    #[diesel(sql_type = BigInt)]
    pub total: i64,
}

pub struct UniteLegaleSearchOutput {
    pub results: Vec<UniteLegaleSearchResult>,
    pub limit: i64,
    pub offset: i64,
    pub sort: UniteLegaleSortField,
    pub direction: SortDirection,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UniteLegaleSearchResponse {
    pub unites_legales: Vec<UniteLegaleSearchResultResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub sort: UniteLegaleSortField,
    pub direction: SortDirection,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UniteLegaleSearchResultResponse {
    pub siren: String,
    pub etat_administratif: String,
    pub date_creation: Option<NaiveDate>,
    pub denomination: Option<String>,
    pub denomination_usuelle_1: Option<String>,
    pub denomination_usuelle_2: Option<String>,
    pub denomination_usuelle_3: Option<String>,
    pub activite_principale: Option<String>,
    pub categorie_juridique: Option<String>,
    pub categorie_entreprise: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}
