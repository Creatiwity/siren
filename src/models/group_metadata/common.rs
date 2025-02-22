use super::super::common::UpdatableModel;
use super::super::etablissement::EtablissementModel;
use super::super::schema::group_metadata;
use super::super::unite_legale::UniteLegaleModel;
use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::{AsExpression, FromSqlRow, prelude::*};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Queryable)]
#[allow(dead_code)]
pub struct Metadata {
    pub id: i32,
    pub group_type: GroupType,
    pub insee_name: String,
    pub file_name: String,
    pub staging_file_timestamp: Option<DateTime<Utc>>,
    pub staging_csv_file_timestamp: Option<DateTime<Utc>>,
    pub staging_imported_timestamp: Option<DateTime<Utc>>,
    pub last_imported_timestamp: Option<DateTime<Utc>>,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_insee_synced_timestamp: Option<DateTime<Utc>>,
}

#[derive(AsChangeset)]
#[diesel(table_name = group_metadata)]
#[diesel(treat_none_as_null = true)]
pub struct MetadataTimestamps {
    pub staging_file_timestamp: Option<DateTime<Utc>>,
    pub staging_csv_file_timestamp: Option<DateTime<Utc>>,
    pub staging_imported_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, FromSqlRow, AsExpression)]
#[diesel(sql_type = Text)]
pub enum GroupType {
    UnitesLegales,
    Etablissements,
}

impl GroupType {
    pub fn get_updatable_model(&self) -> Box<dyn UpdatableModel> {
        match self {
            GroupType::UnitesLegales => Box::new(UniteLegaleModel {}),
            GroupType::Etablissements => Box::new(EtablissementModel {}),
        }
    }
}

// SQL conversion
impl ToSql<Text, Pg> for GroupType {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        match *self {
            GroupType::UnitesLegales => out.write_all(b"unites_legales")?,
            GroupType::Etablissements => out.write_all(b"etablissements")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Pg> for GroupType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"unites_legales" => Ok(GroupType::UnitesLegales),
            b"etablissements" => Ok(GroupType::Etablissements),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl std::fmt::Display for GroupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupType::UnitesLegales => write!(f, "unités légales"),
            GroupType::Etablissements => write!(f, "établissements"),
        }
    }
}
