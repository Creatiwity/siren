pub mod common;
pub mod error;

use super::schema::group_metadata::dsl;
use crate::connectors::Connectors;
use chrono::{DateTime, Utc};
use common::{GroupType, Metadata};
use diesel::prelude::*;
use error::Error;

pub fn get(connectors: &Connectors, group_type: GroupType) -> Result<Metadata, Error> {
    let mut connection = connectors.local.pool.get()?;
    dsl::group_metadata
        .filter(dsl::group_type.eq(group_type))
        .first::<Metadata>(&mut connection)
        .map_err(|error| error.into())
}

pub fn set_staging_imported_timestamp(
    connectors: &Connectors,
    group_type: GroupType,
    timestamp: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut connection = connectors.local.pool.get()?;
    diesel::update(dsl::group_metadata.filter(dsl::group_type.eq(group_type)))
        .set(dsl::staging_imported_timestamp.eq(timestamp))
        .execute(&mut connection)
        .map(|count| count > 0)
        .map_err(|error| error.into())
}

pub fn set_last_imported_timestamp(
    connectors: &Connectors,
    group_type: GroupType,
    timestamp: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut connection = connectors.local.pool.get()?;
    diesel::update(dsl::group_metadata.filter(dsl::group_type.eq(group_type)))
        .set(dsl::staging_imported_timestamp.eq(timestamp))
        .execute(&mut connection)
        .map(|count| count > 0)
        .map_err(|error| error.into())
}

pub fn set_last_insee_synced_timestamp(
    connectors: &Connectors,
    group_type: GroupType,
    timestamp: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut connection = connectors.local.pool.get()?;
    diesel::update(dsl::group_metadata.filter(dsl::group_type.eq(group_type)))
        .set(dsl::last_insee_synced_timestamp.eq(timestamp))
        .execute(&mut connection)
        .map(|count| count > 0)
        .map_err(|error| error.into())
}
