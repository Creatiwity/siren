use super::super::error::Error;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::{Step, UpdateGroupSummary};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::TryStreamExt;
use reqwest::header::LAST_MODIFIED;
use std::fs::create_dir_all;
use std::path::PathBuf;
use tokio::fs::File;
use tokio_util::compat::FuturesAsyncReadCompatExt;

pub struct DownloadAction {
    pub temp_folder: String,
    pub force: bool,
}

#[async_trait]
impl Action for DownloadAction {
    fn step(&self) -> Step {
        Step::DownloadFile
    }

    async fn execute(
        &self,
        group_type: GroupType,
        connectors: &Connectors,
    ) -> Result<UpdateGroupSummary, Error> {
        println!("[Download] Downloading {:#?}", group_type);
        let started_timestamp = Utc::now();

        let metadata = group_metadata::get(connectors, group_type)?;

        // Create temp path
        create_dir_all(self.temp_folder.clone())
            .map_err(|io_error| Error::TempFolderCreationError { io_error })?;

        // Get Zip path
        let mut zip_path = PathBuf::from(self.temp_folder.clone());
        zip_path.push(metadata.file_name);
        zip_path.set_extension("zip");

        // Prepare file download
        let resp = reqwest::get(metadata.url.as_str())
            .await
            .map_err(|req_error| Error::DownloadError { req_error })?;

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
                    println!("[Download] {:#?} already imported", group_type);
                    return Ok(UpdateGroupSummary {
                        group_type,
                        updated: false,
                        status_label: String::from("already imported"),
                        started_timestamp,
                        finished_timestamp: Utc::now(),
                    });
                }
            }

            if let Some(staging_file_timestamp) = metadata.staging_file_timestamp {
                if last_modified.le(&staging_file_timestamp) {
                    println!("[Download] {:#?} already downloaded", group_type);
                    return Ok(UpdateGroupSummary {
                        group_type,
                        updated: false,
                        status_label: String::from("already downloaded"),
                        started_timestamp,
                        finished_timestamp: Utc::now(),
                    });
                }
            }
        }

        // Create an output file into which we will save current stock.
        let mut outfile = File::create(zip_path)
            .await
            .map_err(|io_error| Error::FileCreationError { io_error })?;

        let mut stream = resp
            .bytes_stream() // Convert the body of the response into a futures::io::Stream.
            .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e)) // We must first convert the reqwest::Error into an futures::io::Error.
            .into_async_read() // Convert the stream into an futures::io::AsyncRead.
            .compat(); // Convert the futures::io::AsyncRead into a tokio::io::AsyncRead.

        // Invoke tokio::io::copy to actually perform the download.
        tokio::io::copy(&mut stream, &mut outfile)
            .await
            .map_err(|io_error| Error::FileCopyError { io_error })?;

        println!("[Download] Download of {:#?} finished", group_type);

        // Update staging file timestamp
        group_metadata::set_staging_file_timestamp(connectors, group_type, last_modified)?;

        return Ok(UpdateGroupSummary {
            group_type,
            updated: true,
            status_label: String::from("downloaded"),
            started_timestamp,
            finished_timestamp: Utc::now(),
        });
    }
}
