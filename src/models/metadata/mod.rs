pub mod common;

use super::schema::group_metadata::dsl;
use crate::connectors::Connectors;
use chrono::{DateTime, Utc};
use common::GroupType;
use custom_error::custom_error;
use diesel::prelude::*;

custom_error! { pub Error
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
    MetadataNotFound = "Metadata not found.",
    DatabaseError{diesel_error: diesel::result::Error} = "Unable to run some operations on metadata ({diesel_error}).",
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        match error {
            diesel::result::Error::NotFound => Error::MetadataNotFound,
            _ => Error::DatabaseError {
                diesel_error: error,
            },
        }
    }
}

#[derive(Queryable)]
pub struct Metadata {
    pub id: i32,
    pub group_type: GroupType,
    pub insee_name: String,
    pub file_name: String,
    pub last_imported_timestamp: Option<DateTime<Utc>>,
    pub last_file_timestamp: Option<DateTime<Utc>>,
    pub staging_imported_timestamp: Option<DateTime<Utc>>,
    pub staging_file_timestamp: Option<DateTime<Utc>>,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub fn get(connectors: &Connectors, group_type: GroupType) -> Result<Metadata, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::group_metadata
        .filter(dsl::group_type.eq(group_type))
        .first::<Metadata>(&connection)
        .map_err(|error| error.into())
}

pub fn set_staging_file_timestamp(
    connectors: &Connectors,
    group_type: GroupType,
    timestamp: DateTime<Utc>,
) -> Result<bool, Error> {
    let connection = connectors.local.pool.get()?;
    diesel::update(dsl::group_metadata.filter(dsl::group_type.eq(group_type)))
        .set(dsl::staging_file_timestamp.eq(timestamp))
        .execute(&connection)
        .map(|count| count > 0)
        .map_err(|error| error.into())
}
