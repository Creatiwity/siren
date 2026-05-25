pub mod common;
pub mod error;

use super::schema::group_metadata::dsl;
use crate::connectors::Connectors;
use chrono::{DateTime, Utc};
use common::{GroupType, Metadata};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use error::Error;

pub async fn get(connectors: &Connectors, group_type: GroupType) -> Result<Metadata, Error> {
    let mut connection = connectors.local.pool.get().await?;
    dsl::group_metadata
        .filter(dsl::group_type.eq(group_type))
        .first::<Metadata>(&mut connection)
        .await
        .map_err(|error| error.into())
}

pub async fn set_staging_imported_timestamp(
    connectors: &Connectors,
    group_type: GroupType,
    timestamp: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut connection = connectors.local.pool.get().await?;
    diesel::update(dsl::group_metadata.filter(dsl::group_type.eq(group_type)))
        .set(dsl::staging_imported_timestamp.eq(timestamp))
        .execute(&mut connection)
        .await
        .map(|count| count > 0)
        .map_err(|error| error.into())
}

pub async fn set_last_imported_timestamp(
    connectors: &Connectors,
    group_type: GroupType,
    timestamp: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut connection = connectors.local.pool.get().await?;
    diesel::update(dsl::group_metadata.filter(dsl::group_type.eq(group_type)))
        .set(dsl::last_imported_timestamp.eq(timestamp))
        .execute(&mut connection)
        .await
        .map(|count| count > 0)
        .map_err(|error| error.into())
}

pub async fn set_last_insee_synced_timestamp(
    connectors: &Connectors,
    group_type: GroupType,
    timestamp: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut connection = connectors.local.pool.get().await?;
    diesel::update(dsl::group_metadata.filter(dsl::group_type.eq(group_type)))
        .set(dsl::last_insee_synced_timestamp.eq(timestamp))
        .execute(&mut connection)
        .await
        .map(|count| count > 0)
        .map_err(|error| error.into())
}
