use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::connectors::insee::error::InseeUpdate;
use crate::connectors::Connectors;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use custom_error::custom_error;
use tracing::debug;

#[async_trait]
pub trait UpdatableModel: Sync + Send {
    fn count(&self, connectors: &Connectors) -> Result<i64, Error>;
    fn count_staging(&self, connectors: &Connectors) -> Result<i64, Error>;
    fn insert_in_staging(&self, connectors: &Connectors, file_path: String) -> Result<bool, Error>;
    fn insert_zip_in_staging(
        &self,
        connectors: &Connectors,
        file_path: &Path,
    ) -> Result<bool, Error>;
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

pub fn copy_zipped_csv(
    file_path: &Path,
    write: &mut dyn Write,
) -> Result<(), diesel::result::Error> {
    let zip_file = File::open(file_path).map_err(|io_error| {
        diesel::result::Error::DeserializationError(Box::new(Error::ZipOpen { io_error }))
    })?;
    let mut archive = zip::ZipArchive::new(zip_file).map_err(|zip_error| {
        diesel::result::Error::DeserializationError(Box::new(Error::ZipDecode { zip_error }))
    })?;

    if archive.len() != 1 {
        return Err(diesel::result::Error::DeserializationError(Box::new(
            Error::ZipFormat,
        )));
    }

    let mut zipped_csv_file = archive.by_index(0).map_err(|zip_error| {
        diesel::result::Error::DeserializationError(Box::new(Error::ZipAccessFile { zip_error }))
    })?;

    debug!(
        "Unzipping and inserting file etablissement extracted to database ({} bytes)",
        zipped_csv_file.size()
    );

    std::io::copy(&mut zipped_csv_file, write).map_err(|io_error| {
        diesel::result::Error::DeserializationError(Box::new(Error::FileCSVRead { io_error }))
    })?;

    diesel::QueryResult::Ok(())
}

custom_error! { pub Error
    LocalConnectionFailed{source: r2d2::Error} = "Unable to connect to local database ({source}).",
    Database{source: diesel::result::Error} = "Unable to run some operations on updatable model ({source}).",
    Update {source: InseeUpdate} = "{source}",
    MissingInseeConnector = "Missing required Insee connector",
    ZipOpen {io_error: std::io::Error} = "Unable to open data zip file ({io_error})",
    ZipDecode {zip_error: zip::result::ZipError} = "Unable to decode zip file ({zip_error})",
    ZipFormat = "Archive has more than one file inside it, you should review it before running it again",
    ZipAccessFile {zip_error: zip::result::ZipError} = "Unable to open file in archive ({zip_error})",
    FileCSVRead {io_error: std::io::Error} = "Unable to read CSV file from archive ({io_error})",
}
