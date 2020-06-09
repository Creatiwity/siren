use crate::connectors::insee::error::InseeError;
use crate::connectors::Connectors;
use chrono::{DateTime, Utc};
use custom_error::custom_error;

pub trait UpdatableModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, Error>;
    fn count_staging(&self, connectors: &Connectors) -> Result<i64, Error>;
    fn insert_in_staging(&self, connectors: &Connectors, file_path: String) -> Result<bool, Error>;
    fn swap(&self, connectors: &Connectors) -> Result<(), Error>;
    fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<DateTime<Utc>>, Error>;
    fn update_daily_data(
        &self,
        connectors: &Connectors,
        start_timestamp: DateTime<Utc>,
    ) -> Result<(), Error>;
}

custom_error! { pub Error
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
    DatabaseError{source: diesel::result::Error} = "Unable to run some operations on updatable model ({source}).",
    UpdateError {source: InseeError} = "{source}",
    MissingInseeConnector = "Missing required Insee connector",
}
