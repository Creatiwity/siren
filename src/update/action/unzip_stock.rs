use super::super::error::Error;
use super::super::summary::SummaryGroupDelegate;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use async_trait::async_trait;
use std::fs::{create_dir_all, set_permissions, File, Permissions};
use std::io;
use std::path::PathBuf;
use tracing::debug;

pub struct UnzipAction {
    pub temp_folder: String,
    pub file_folder: String,
    pub force: bool,
}

#[async_trait]
impl Action for UnzipAction {
    async fn execute<'a, 'b>(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
        summary_delegate: &'b mut SummaryGroupDelegate<'a, 'b>,
    ) -> Result<(), Error> {
        debug!("Unzipping {:#?}", group_type);
        summary_delegate.start(connectors, None, 1)?;

        let metadata = group_metadata::get(connectors, group_type)?;

        // Unzip only if zip file is referenced in database
        let staging_file_timestamp = match metadata.staging_file_timestamp {
            Some(staging_file_timestamp) => staging_file_timestamp,
            None => {
                debug!("Nothing to unzip for {:#?}", group_type);

                summary_delegate.finish(connectors, String::from("nothing to unzip"), 0, false)?;

                return Ok(());
            }
        };

        // Test if not already imported or unzipped
        if !self.force {
            if let Some(staging_csv_file_timestamp) = metadata.staging_csv_file_timestamp {
                if let Some(last_imported_timestamp) = metadata.last_imported_timestamp {
                    if staging_csv_file_timestamp.le(&last_imported_timestamp) {
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

                if staging_file_timestamp.le(&staging_csv_file_timestamp) {
                    debug!("{:#?} already unzipped", group_type);

                    summary_delegate.finish(
                        connectors,
                        String::from("already unzipped"),
                        0,
                        false,
                    )?;

                    return Ok(());
                }
            }
        }

        // Get Zip path
        let mut zip_path = PathBuf::from(self.temp_folder.clone());
        zip_path.push(metadata.file_name.clone());
        zip_path.set_extension("zip");

        // Get CSV path
        let mut csv_path = PathBuf::from(self.file_folder.clone());
        csv_path.push(metadata.file_name);
        csv_path.set_extension("csv");

        if let Some(p) = csv_path.parent() {
            if !p.exists() {
                create_dir_all(&p).map_err(|io_error| Error::FileFolderCreation { io_error })?;
            }
        }

        let zip_file = File::open(&zip_path).map_err(|io_error| Error::ZipOpen { io_error })?;
        let mut archive =
            zip::ZipArchive::new(zip_file).map_err(|zip_error| Error::ZipDecode { zip_error })?;

        if archive.len() != 1 {
            return Err(Error::ZipFormat);
        }

        let mut zipped_csv_file = archive
            .by_index(0)
            .map_err(|zip_error| Error::ZipAccessFile { zip_error })?;

        debug!(
            "Unzipping file {:#?} extracted to \"{}\" ({} bytes)",
            group_type,
            csv_path.as_path().display(),
            zipped_csv_file.size()
        );

        let mut csv_file =
            File::create(&csv_path).map_err(|io_error| Error::FileCSVCreation { io_error })?;
        io::copy(&mut zipped_csv_file, &mut csv_file)
            .map_err(|io_error| Error::FileCSVCopy { io_error })?;

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = zipped_csv_file.unix_mode() {
                set_permissions(&csv_path, Permissions::from_mode(mode))
                    .map_err(|io_error| Error::FileCSVPermission { io_error })?;
            }
        }

        group_metadata::set_staging_csv_file_timestamp(
            connectors,
            group_type,
            staging_file_timestamp,
        )?;

        debug!("Unzip of {:#?} finished", group_type);

        summary_delegate.finish(connectors, String::from("unzipped"), 1, true)?;

        Ok(())
    }
}
