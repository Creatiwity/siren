use super::super::error::Error;
use super::super::summary::SummaryGroupDelegate;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use reqwest::header::LAST_MODIFIED;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::File;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tracing::debug;

pub struct DownloadAction {
    pub temp_folder: String,
    pub force: bool,
}

#[async_trait]
impl Action for DownloadAction {
    async fn execute<'a, 'b>(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
        summary_delegate: &'b mut SummaryGroupDelegate<'a, 'b>,
    ) -> Result<(), Error> {
        debug!("Downloading {:#?}", group_type);
        summary_delegate.start(connectors, None, 1)?;

        let metadata = group_metadata::get(connectors, group_type)?;

        // Prepare file download
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(3600))
            .build()
            .map_err(|req_error| Error::Download { req_error })?;

        let resp = client
            .get(&metadata.url)
            .send()
            .await
            .map_err(|req_error| Error::Download { req_error })?;

        // Decode Last-Modified header
        let last_modified_str = resp
            .headers()
            .get(LAST_MODIFIED)
            .ok_or(Error::MissingLastModifiedHeader)?
            .to_str()
            .map_err(|head_error| Error::InvalidLastModifiedHeader { head_error })?;

        let last_modified = DateTime::parse_from_rfc2822(last_modified_str)
            .map_err(|date_error| Error::InvalidLastModifiedDate { date_error })?;

        let last_modified = last_modified.with_timezone(&Utc);

        // Test if not already imported or downloaded
        if !self.force {
            if let Some(last_imported_timestamp) = metadata.last_imported_timestamp {
                if last_modified.le(&last_imported_timestamp) {
                    debug!("{:#?} already imported", group_type);

                    summary_delegate.finish(
                        connectors,
                        String::from("already imported"),
                        0,
                        false,
                    )?;

                    return Ok(());
                }
            }

            if let Some(staging_file_timestamp) = metadata.staging_file_timestamp {
                if last_modified.le(&staging_file_timestamp) {
                    debug!("{:#?} already downloaded", group_type);

                    summary_delegate.finish(
                        connectors,
                        String::from("already downloaded"),
                        0,
                        false,
                    )?;

                    return Ok(());
                }
            }
        }

        // Create temp path
        create_dir_all(self.temp_folder.clone())
            .map_err(|io_error| Error::TempFolderCreation { io_error })?;

        // Get Zip path
        let mut zip_path = PathBuf::from(self.temp_folder.clone());
        zip_path.push(metadata.file_name);
        zip_path.set_extension("zip");

        // Create an output file into which we will save current stock.
        let mut outfile = File::create(zip_path)
            .await
            .map_err(|io_error| Error::FileCreation { io_error })?;

        let mut stream = resp
            .bytes_stream() // Convert the body of the response into a futures::io::Stream.
            .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e)) // We must first convert the reqwest::Error into an futures::io::Error.
            .into_async_read() // Convert the stream into an futures::io::AsyncRead.
            .compat(); // Convert the futures::io::AsyncRead into a tokio::io::AsyncRead.

        // Invoke tokio::io::copy to actually perform the download.
        tokio::io::copy(&mut stream, &mut outfile)
            .await
            .map_err(|io_error| Error::FileCopy { io_error })?;

        debug!("Download of {:#?} finished", group_type);

        // Update staging file timestamp
        group_metadata::set_staging_file_timestamp(connectors, group_type, last_modified)?;

        summary_delegate.finish(connectors, String::from("downloaded"), 1, true)?;

        Ok(())
    }
}
