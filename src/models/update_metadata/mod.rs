pub mod common;
pub mod error;

use super::schema::update_metadata::dsl;
use crate::connectors::Connectors;
use chrono::{DateTime, Duration, Utc};
use common::{
    ErrorUpdateMetadata, FinishedUpdateMetadata, LaunchUpdateMetadata, SyntheticGroupType,
    UpdateMetadata, UpdateStatus, UpdateSummary,
};
use diesel::prelude::*;
use error::Error;

pub fn launch_update(
    connectors: &Connectors,
    synthetic_group_type: SyntheticGroupType,
    force: bool,
    data_only: bool,
) -> Result<DateTime<Utc>, Error> {
    let mut connection = connectors.local.pool.get()?;

    let launched_update_result = dsl::update_metadata
        .select(dsl::updated_at)
        .filter(dsl::status.eq(UpdateStatus::Launched))
        .first::<DateTime<Utc>>(&mut connection);

    if let Ok(launched_updated_at) = launched_update_result {
        if Utc::now().signed_duration_since(launched_updated_at) <= Duration::hours(1) {
            return Err(Error::AlreadyLaunched);
        }

        error_update(
            connectors,
            String::from("Process stopped automatically after being stucked."),
            Utc::now(),
        )?;
    }

    let launched_timestamp = Utc::now();

    match diesel::insert_into(dsl::update_metadata)
        .values(&LaunchUpdateMetadata {
            synthetic_group_type,
            force,
            data_only,
            launched_timestamp,
        })
        .execute(&mut connection)
    {
        Ok(count) => {
            if count > 0 {
                Ok(launched_timestamp)
            } else {
                Err(Error::UpdateNotRegistered)
            }
        }
        Err(error) => Err(error.into()),
    }
}

pub fn progress_update(connectors: &Connectors, summary: UpdateSummary) -> Result<bool, Error> {
    let mut connection = connectors.local.pool.get()?;

    diesel::update(dsl::update_metadata.filter(dsl::status.eq(UpdateStatus::Launched)))
        .set(dsl::summary.eq(summary))
        .execute(&mut connection)
        .map(|count| count > 0)
        .map_err(|error| error.into())
}

pub fn finished_update(connectors: &Connectors, summary: UpdateSummary) -> Result<bool, Error> {
    let mut connection = connectors.local.pool.get()?;
    let finished_timestamp = summary.finished_timestamp;

    diesel::update(dsl::update_metadata.filter(dsl::status.eq(UpdateStatus::Launched)))
        .set(&FinishedUpdateMetadata {
            status: UpdateStatus::Finished,
            summary,
            finished_timestamp,
        })
        .execute(&mut connection)
        .map(|count| count > 0)
        .map_err(|error| error.into())
}

pub fn error_update(
    connectors: &Connectors,
    error: String,
    finished_timestamp: DateTime<Utc>,
) -> Result<bool, Error> {
    let mut connection = connectors.local.pool.get()?;

    diesel::update(dsl::update_metadata.filter(dsl::status.eq(UpdateStatus::Launched)))
        .set(&ErrorUpdateMetadata {
            status: UpdateStatus::Error,
            error,
            finished_timestamp,
        })
        .execute(&mut connection)
        .map(|count| count > 0)
        .map_err(|error| error.into())
}

pub fn current_update(connectors: &Connectors) -> Result<UpdateMetadata, Error> {
    let mut connection = connectors.local.pool.get()?;

    dsl::update_metadata
        .order(dsl::launched_timestamp.desc())
        .first::<UpdateMetadata>(&mut connection)
        .map_err(|error| error.into())
}
