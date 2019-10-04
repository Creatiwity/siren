pub mod common;
mod error;

use super::schema::group_metadata::dsl;
use crate::connectors::Connectors;
use chrono::{DateTime, Utc};
use common::{GroupType, Metadata};
use diesel::prelude::*;
use error::Error;

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
