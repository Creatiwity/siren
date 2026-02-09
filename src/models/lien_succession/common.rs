use super::super::schema::lien_succession;
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Insertable, Queryable, Selectable, ToSchema, Serialize, Clone, Debug)]
#[diesel(table_name = lien_succession)]
pub struct LienSuccession {
    pub siret_etablissement_predecesseur: String,
    pub siret_etablissement_successeur: String,
    pub date_lien_succession: NaiveDate,
    pub transfert_siege: bool,
    pub continuite_economique: bool,
    pub date_dernier_traitement_lien_succession: Option<NaiveDateTime>,
}
