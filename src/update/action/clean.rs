use super::super::error::Error;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::{Step, UpdateGroupSummary};
use async_trait::async_trait;
use chrono::Utc;
use std::fs::remove_file;
use std::path::PathBuf;

pub struct CleanAction {
    pub temp_folder: String,
    pub file_folder: String,
}

#[async_trait]
impl Action for CleanAction {
    fn step(&self) -> Step {
        Step::CleanFile
    }

    async fn execute(
        &self,
        group_type: GroupType,
        connectors: &Connectors,
    ) -> Result<UpdateGroupSummary, Error> {
        println!("[Clean] Cleaning {:#?}", group_type);
        let started_timestamp = Utc::now();
        let mut updated = true;
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
            println!("[Clean] Zip not deleted ({})", error);
            updated = false;
            status_label = String::from("zip not deleted");
        }

        if let Err(error) = remove_file(csv_path) {
            println!("[Clean] CSV not deleted ({})", error);
            updated = false;
            status_label = String::from("csv not deleted");
        }

        group_metadata::reset_staging_timestamps(connectors, group_type)?;

        println!("[Clean] Finished cleaning of {:#?}", group_type);

        Ok(UpdateGroupSummary {
            group_type,
            updated,
            status_label,
            started_timestamp,
            finished_timestamp: Utc::now(),
        })
    }
}
