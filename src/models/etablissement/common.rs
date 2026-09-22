use super::super::schema::etablissement;
use super::super::search::Facets;
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Float4, Float8, Nullable, Text, VarChar};
use postgis_diesel::sql_types::Geography;
use postgis_diesel::types::Point;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::{IntoParams, ToSchema};

#[derive(ToSchema, Serialize, Clone, Debug)]
pub struct EtablissementPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Insertable, Serialize, Clone, Debug)]
#[diesel(table_name = etablissement)]
pub struct EtablissementInsertable {
    pub siren: String,
    pub nic: String,
    pub siret: String,
    pub statut_diffusion: String,
    pub date_creation: Option<NaiveDate>,
    pub tranche_effectifs: Option<String>,
    pub annee_effectifs: Option<i32>,
    pub activite_principale_registre_metiers: Option<String>,
    pub date_dernier_traitement: Option<NaiveDateTime>,
    pub etablissement_siege: bool,
    pub nombre_periodes: Option<i32>,
    pub complement_adresse: Option<String>,
    pub numero_voie: Option<String>,
    pub indice_repetition: Option<String>,
    pub dernier_numero_voie: Option<String>,
    pub indice_repetition_dernier_numero_voie: Option<String>,
    pub type_voie: Option<String>,
    pub libelle_voie: Option<String>,
    pub code_postal: Option<String>,
    pub libelle_commune: Option<String>,
    pub libelle_commune_etranger: Option<String>,
    pub distribution_speciale: Option<String>,
    pub code_commune: Option<String>,
    pub code_cedex: Option<String>,
    pub libelle_cedex: Option<String>,
    pub code_pays_etranger: Option<String>,
    pub libelle_pays_etranger: Option<String>,
    pub identifiant_adresse: Option<String>,
    pub coordonnee_lambert_x: Option<String>,
    pub coordonnee_lambert_y: Option<String>,
    pub complement_adresse2: Option<String>,
    pub numero_voie_2: Option<String>,
    pub indice_repetition_2: Option<String>,
    pub type_voie_2: Option<String>,
    pub libelle_voie_2: Option<String>,
    pub code_postal_2: Option<String>,
    pub libelle_commune_2: Option<String>,
    pub libelle_commune_etranger_2: Option<String>,
    pub distribution_speciale_2: Option<String>,
    pub code_commune_2: Option<String>,
    pub code_cedex_2: Option<String>,
    pub libelle_cedex_2: Option<String>,
    pub code_pays_etranger_2: Option<String>,
    pub libelle_pays_etranger_2: Option<String>,
    pub date_debut: Option<NaiveDate>,
    pub etat_administratif: String,
    pub enseigne_1: Option<String>,
    pub enseigne_2: Option<String>,
    pub enseigne_3: Option<String>,
    pub denomination_usuelle: Option<String>,
    pub activite_principale: Option<String>,
    pub nomenclature_activite_principale: Option<String>,
    pub caractere_employeur: Option<String>,
}

#[derive(Queryable, Selectable, ToSchema, Serialize, Clone, Debug)]
#[diesel(table_name = etablissement)]
pub struct Etablissement {
    pub siren: String,
    pub nic: String,
    pub siret: String,
    pub statut_diffusion: String,
    pub date_creation: Option<NaiveDate>,
    pub tranche_effectifs: Option<String>,
    pub annee_effectifs: Option<i32>,
    pub activite_principale_registre_metiers: Option<String>,
    pub date_dernier_traitement: Option<NaiveDateTime>,
    pub etablissement_siege: bool,
    pub nombre_periodes: Option<i32>,
    pub complement_adresse: Option<String>,
    pub numero_voie: Option<String>,
    pub indice_repetition: Option<String>,
    pub dernier_numero_voie: Option<String>,
    pub indice_repetition_dernier_numero_voie: Option<String>,
    pub type_voie: Option<String>,
    pub libelle_voie: Option<String>,
    pub code_postal: Option<String>,
    pub libelle_commune: Option<String>,
    pub libelle_commune_etranger: Option<String>,
    pub distribution_speciale: Option<String>,
    pub code_commune: Option<String>,
    pub code_cedex: Option<String>,
    pub libelle_cedex: Option<String>,
    pub code_pays_etranger: Option<String>,
    pub libelle_pays_etranger: Option<String>,
    pub identifiant_adresse: Option<String>,
    pub coordonnee_lambert_x: Option<String>,
    pub coordonnee_lambert_y: Option<String>,
    pub complement_adresse2: Option<String>,
    pub numero_voie_2: Option<String>,
    pub indice_repetition_2: Option<String>,
    pub type_voie_2: Option<String>,
    pub libelle_voie_2: Option<String>,
    pub code_postal_2: Option<String>,
    pub libelle_commune_2: Option<String>,
    pub libelle_commune_etranger_2: Option<String>,
    pub distribution_speciale_2: Option<String>,
    pub code_commune_2: Option<String>,
    pub code_cedex_2: Option<String>,
    pub libelle_cedex_2: Option<String>,
    pub code_pays_etranger_2: Option<String>,
    pub libelle_pays_etranger_2: Option<String>,
    pub date_debut: Option<NaiveDate>,
    pub etat_administratif: String,
    pub enseigne_1: Option<String>,
    pub enseigne_2: Option<String>,
    pub enseigne_3: Option<String>,
    pub denomination_usuelle: Option<String>,
    pub activite_principale: Option<String>,
    pub nomenclature_activite_principale: Option<String>,
    pub caractere_employeur: Option<String>,
    pub activite_principale_naf25: Option<String>,
    #[schema(value_type = Option<EtablissementPoint>)]
    pub position: Option<Point>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EtablissementSortField {
    DateCreation,
    DateDebut,
    Distance,
    Relevance,
    /// Primary-key order, the only sort accepting `cursor`: the only one where
    /// keyset resume is both exact and index-served, at constant cost per page.
    Siret,
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

#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct EtablissementSearchParams {
    pub q: Option<String>,
    /// Plain-text commune label ("paris", "saint etienne", "marseile"), resolved
    /// to a list of `code_commune` with prefix matching and typo tolerance.
    pub commune: Option<String>,
    pub etat_administratif: Option<EtatAdministratif>,
    /// Comma-separated values.
    pub code_postal: Option<String>,
    /// Exclusion. Establishments with no postal code are kept.
    pub code_postal_not: Option<String>,
    /// Comma-separated values.
    pub siren: Option<String>,
    /// Comma-separated values.
    pub code_commune: Option<String>,
    /// Exclusion. Establishments with no commune code are kept.
    pub code_commune_not: Option<String>,
    /// Comma-separated values.
    pub activite_principale: Option<String>,
    /// Exclusion. Establishments with no activity are kept.
    pub activite_principale_not: Option<String>,
    pub etablissement_siege: Option<bool>,
    /// Inclusive lower bound.
    pub date_creation_min: Option<NaiveDate>,
    /// Inclusive upper bound.
    pub date_creation_max: Option<NaiveDate>,
    /// Inclusive lower bound.
    pub date_debut_min: Option<NaiveDate>,
    /// Inclusive upper bound.
    pub date_debut_max: Option<NaiveDate>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub radius: Option<f64>,
    /// Comma-separated fields to facet on: `etat_administratif`, `code_postal`,
    /// `code_commune`, `activite_principale`, `etablissement_siege`.
    pub facette: Option<String>,
    pub sort: Option<EtablissementSortField>,
    pub direction: Option<SortDirection>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Resume position returned by `next_cursor`. Requires `sort=siret`, excludes
    /// `offset`, and walks past the 10,000-result offset ceiling.
    pub cursor: Option<String>,
}

#[derive(Debug, QueryableByName, Serialize, ToSchema)]
pub struct EtablissementSearchResult {
    #[diesel(sql_type = VarChar)]
    pub siret: String,
    #[diesel(sql_type = VarChar)]
    pub siren: String,
    #[diesel(sql_type = VarChar)]
    pub etat_administratif: String,
    #[diesel(sql_type = Nullable<diesel::sql_types::Date>)]
    pub date_creation: Option<NaiveDate>,
    #[diesel(sql_type = Nullable<Text>)]
    pub denomination_usuelle: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub enseigne_1: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub enseigne_2: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub enseigne_3: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub code_postal: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub libelle_commune: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub activite_principale: Option<String>,
    #[diesel(sql_type = Bool)]
    pub etablissement_siege: bool,
    #[diesel(sql_type = Nullable<Float8>)]
    pub meter_distance: Option<f64>,
    #[diesel(sql_type = Nullable<Float4>)]
    pub score: Option<f32>,
    #[schema(value_type = Option<EtablissementPoint>)]
    #[diesel(sql_type = Nullable<Geography>)]
    pub position: Option<Point>,
}

pub struct EtablissementSearchOutput {
    pub results: Vec<EtablissementSearchResult>,
    pub total: i64,
    pub total_capped: bool,
    pub limit: i64,
    pub offset: i64,
    pub sort: EtablissementSortField,
    pub direction: SortDirection,
    pub suggestion: Option<String>,
    pub facettes: Facets,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EtablissementSearchResponse {
    pub etablissements: Vec<EtablissementSearchResultResponse>,
    pub total: i64,
    /// `true` when `total` hit its ceiling: there are at least that many
    /// results, without the exact count being computed.
    pub total_capped: bool,
    pub limit: i64,
    pub offset: i64,
    pub sort: EtablissementSortField,
    pub direction: SortDirection,
    /// Present only when the search returns nothing and a close term exists in
    /// the corpus.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Counts per value for each field requested through `facette`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[schema(value_type = Object)]
    pub facettes: Facets,
    /// Position to pass as `cursor` for the next page. Absent on the last page,
    /// or when the sort does not support cursor resume.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EtablissementSearchResultResponse {
    pub siret: String,
    pub siren: String,
    pub etat_administratif: String,
    pub date_creation: Option<NaiveDate>,
    pub denomination_usuelle: Option<String>,
    pub enseigne_1: Option<String>,
    pub enseigne_2: Option<String>,
    pub enseigne_3: Option<String>,
    pub code_postal: Option<String>,
    pub libelle_commune: Option<String>,
    pub activite_principale: Option<String>,
    pub etablissement_siege: bool,
    #[schema(value_type = Option<EtablissementPoint>)]
    pub position: Option<Point>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_distance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}
