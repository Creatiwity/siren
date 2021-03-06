use super::super::error::Error;
use super::super::summary::SummaryGroupDelegate;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use async_trait::async_trait;
use std::fs::remove_file;
use std::path::PathBuf;
use tracing::debug;

pub struct CleanAction {
    pub temp_folder: String,
    pub file_folder: String,
}

#[async_trait]
impl Action for CleanAction {
    async fn execute<'a, 'b>(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
        summary_delegate: &'b mut SummaryGroupDelegate<'a, 'b>,
    ) -> Result<(), Error> {
        debug!("Cleaning {:#?}", group_type);

        summary_delegate.start(connectors, None, 2)?;

        let mut updated = true;
        let mut done_count = 2;
        let mut status_label = String::from("cleaned");

        let metadata = group_metadata::get(connectors, group_type)?;

        // Get Zip path
        let mut zip_path = PathBuf::from(self.temp_folder.clone());
        zip_path.push(metadata.file_name.clone());
        zip_path.set_extension("zip");

        // Get CSV path
        let mut csv_path = PathBuf::from(self.file_folder.clone());
        csv_path.push(metadata.file_name);
        csv_path.set_extension("csv");

        if let Err(error) = remove_file(zip_path) {
            debug!("Zip not deleted ({})", error);
            updated = false;
            done_count -= 1;
            status_label = String::from("zip not deleted");
        }

        if let Err(error) = remove_file(csv_path) {
            debug!("CSV not deleted ({})", error);
            updated = false;
            done_count -= 1;
            status_label = String::from("csv not deleted");
        }

        group_metadata::reset_staging_timestamps(connectors, group_type)?;

        summary_delegate.finish(connectors, status_label, done_count, updated)?;

        debug!("Finished cleaning of {:#?}", group_type);

        Ok(())
    }
}
