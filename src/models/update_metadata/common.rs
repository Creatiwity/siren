use super::super::group_metadata::common::GroupType;
use super::super::schema::update_metadata;
use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::{Jsonb, Text};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Insertable)]
#[table_name = "update_metadata"]
pub struct LaunchUpdateMetadata {
    pub synthetic_group_type: SyntheticGroupType,
    pub force: bool,
    pub data_only: bool,
    pub launched_timestamp: DateTime<Utc>,
}

#[derive(AsChangeset)]
#[table_name = "update_metadata"]
#[changeset_options(treat_none_as_null = "true")]
pub struct FinishedUpdateMetadata {
    pub status: UpdateStatus,
    pub summary: UpdateSummary,
    pub finished_timestamp: DateTime<Utc>,
}

#[derive(AsChangeset)]
#[table_name = "update_metadata"]
#[changeset_options(treat_none_as_null = "true")]
pub struct ErrorUpdateMetadata {
    pub status: UpdateStatus,
    pub error: String,
    pub finished_timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub enum SyntheticGroupType {
    UnitesLegales,
    Etablissements,
    All,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub enum UpdateStatus {
    Launched,
    Finished,
    Error,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateGroupSummary {
    pub group_type: GroupType,
    pub updated: bool,
    pub status_label: String,
    pub started_timestamp: DateTime<Utc>,
    pub finished_timestamp: DateTime<Utc>,
    pub planned_count: u32,
    pub done_count: u32,
    pub reference_timestamp: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub enum Step {
    DownloadFile,
    UnzipFile,
    InsertData,
    SwapData,
    CleanFile,
    SyncInsee,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateStepSummary {
    pub step: Step,
    pub updated: bool,
    pub started_timestamp: DateTime<Utc>,
    pub finished_timestamp: DateTime<Utc>,
    pub groups: Vec<UpdateGroupSummary>,
}

#[derive(FromSqlRow, AsExpression, Deserialize, Serialize, Clone, Debug)]
#[sql_type = "Jsonb"]
pub struct UpdateSummary {
    pub updated: bool,
    pub started_timestamp: DateTime<Utc>,
    pub finished_timestamp: DateTime<Utc>,
    pub steps: Vec<UpdateStepSummary>,
}

impl FromSql<Jsonb, Pg> for UpdateSummary {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, Pg>>::from_sql(bytes)?;
        Ok(serde_json::from_value(value)?)
    }
}

impl ToSql<Jsonb, Pg> for UpdateSummary {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        let value = serde_json::to_value(self)?;
        <serde_json::Value as ToSql<Jsonb, Pg>>::to_sql(&value, out)
    }
}

// Type conversion
impl From<SyntheticGroupType> for Vec<GroupType> {
    fn from(group: SyntheticGroupType) -> Self {
        match group {
            SyntheticGroupType::UnitesLegales => vec![GroupType::UnitesLegales],
            SyntheticGroupType::Etablissements => vec![GroupType::Etablissements],
            SyntheticGroupType::All => vec![GroupType::UnitesLegales, GroupType::Etablissements],
        }
    }
}

// SQL conversion
impl ToSql<Text, Pg> for SyntheticGroupType {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        match *self {
            SyntheticGroupType::UnitesLegales => out.write_all(b"unites_legales")?,
            SyntheticGroupType::Etablissements => out.write_all(b"etablissements")?,
            SyntheticGroupType::All => out.write_all(b"all")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Pg> for SyntheticGroupType {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"unites_legales" => Ok(SyntheticGroupType::UnitesLegales),
            b"etablissements" => Ok(SyntheticGroupType::Etablissements),
            b"all" => Ok(SyntheticGroupType::All),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl std::fmt::Display for SyntheticGroupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyntheticGroupType::UnitesLegales => write!(f, "unités légales"),
            SyntheticGroupType::Etablissements => write!(f, "établissements"),
            SyntheticGroupType::All => write!(f, "all"),
        }
    }
}

// SQL conversion
impl ToSql<Text, Pg> for UpdateStatus {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        match *self {
            UpdateStatus::Launched => out.write_all(b"launched")?,
            UpdateStatus::Finished => out.write_all(b"finished")?,
            UpdateStatus::Error => out.write_all(b"error")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Pg> for UpdateStatus {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match not_none!(bytes) {
            b"launched" => Ok(UpdateStatus::Launched),
            b"finished" => Ok(UpdateStatus::Finished),
            b"error" => Ok(UpdateStatus::Error),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl std::fmt::Display for UpdateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateStatus::Launched => write!(f, "launched"),
            UpdateStatus::Finished => write!(f, "finished"),
            UpdateStatus::Error => write!(f, "error"),
        }
    }
}
