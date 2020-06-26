use crate::connectors::insee::error::InseeUpdateError;
use crate::connectors::Connectors;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use custom_error::custom_error;

#[async_trait]
pub trait UpdatableModel: Sync + Send {
    fn count(&self, connectors: &Connectors) -> Result<i64, Error>;
    fn count_staging(&self, connectors: &Connectors) -> Result<i64, Error>;
    fn insert_in_staging(&self, connectors: &Connectors, file_path: String) -> Result<bool, Error>;
    fn swap(&self, connectors: &Connectors) -> Result<(), Error>;
    async fn get_total_count(
        &self,
        connectors: &mut Connectors,
        start_timestamp: NaiveDateTime,
    ) -> Result<u32, Error>;
    fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, Error>;
    async fn update_daily_data(
        &self,
        connectors: &mut Connectors,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, usize), Error>;
}

custom_error! { pub Error
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
    DatabaseError{source: diesel::result::Error} = "Unable to run some operations on updatable model ({source}).",
    UpdateError {source: InseeUpdateError} = "{source}",
    MissingInseeConnector = "Missing required Insee connector",
}
