use super::super::error::Error;
use super::super::summary::SummaryGroupDelegate;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::debug;

pub struct UnzipInsertAction {
    pub temp_folder: String,
    pub force: bool,
}

#[async_trait]
impl Action for UnzipInsertAction {
    async fn execute<'a, 'b>(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
        summary_delegate: &'b mut SummaryGroupDelegate<'a, 'b>,
    ) -> Result<(), Error> {
        debug!("Unzipping and inserting {:#?}", group_type);
        summary_delegate.start(connectors, None, 1)?;

        let metadata = group_metadata::get(connectors, group_type)?;

        // Unzip only if zip file is referenced in database
        let staging_file_timestamp = match metadata.staging_file_timestamp {
            Some(staging_file_timestamp) => staging_file_timestamp,
            None => {
                debug!("Nothing to unzip and insert for {:#?}", group_type);

                summary_delegate.finish(
                    connectors,
                    String::from("nothing to unzip and insert"),
                    0,
                    false,
                )?;

                return Ok(());
            }
        };

        // Test if not already imported or inserted
        if !self.force {
            if let Some(staging_imported_timestamp) = metadata.staging_imported_timestamp {
                if let Some(last_imported_timestamp) = metadata.last_imported_timestamp {
                    if staging_imported_timestamp.le(&last_imported_timestamp) {
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

                if staging_file_timestamp.le(&staging_imported_timestamp) {
                    debug!("{:#?} already unzippped and inserted", group_type);

                    summary_delegate.finish(
                        connectors,
                        String::from("already unzippped and inserted"),
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

        group_type
            .get_updatable_model()
            .insert_zip_in_staging(connectors, &zip_path)?;

        group_metadata::set_staging_imported_timestamp(
            connectors,
            group_type,
            staging_file_timestamp,
        )?;

        debug!("Unzip of {:#?} finished", group_type);

        summary_delegate.finish(connectors, String::from("inserted"), 1, true)?;

        Ok(())
    }
}
