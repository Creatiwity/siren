use super::super::error::Error;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::{Step, UpdateGroupSummary};
use async_trait::async_trait;
use chrono::Utc;
use std::fs::canonicalize;
use std::path::PathBuf;
use super::super::summary::SummaryGroupDelegate;

pub struct InsertAction {
    pub db_folder: String,
    pub force: bool,
}

#[async_trait]
impl Action for InsertAction {
    fn step(&self) -> Step {
        Step::InsertData
    }

    async fn execute<'a, 'b>(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
        summary_delegate: &'b mut SummaryGroupDelegate<'a, 'b>,
    ) -> Result<(), Error> {
        println!("[Insert] Insert {:#?}", group_type);
        let started_timestamp = Utc::now();

        let metadata = group_metadata::get(connectors, group_type)?;

        // Insert only if csv file is referenced in database
        let staging_csv_file_timestamp = match metadata.staging_csv_file_timestamp {
            Some(staging_csv_file_timestamp) => staging_csv_file_timestamp,
            None => {
                println!("[Insert] Nothing to insert for {:#?}", group_type);
                return Ok(UpdateGroupSummary {
                    group_type,
                    updated: false,
                    status_label: String::from("nothing to insert"),
                    started_timestamp,
                    finished_timestamp: Utc::now(),
                    planned_count: 1,
                    done_count: 0,
                    reference_timestamp: Utc::now(),
                });
            }
        };

        // Test if not already inserted
        if !self.force {
            if let Some(staging_imported_timestamp) = metadata.staging_imported_timestamp {
                if let Some(last_imported_timestamp) = metadata.last_imported_timestamp {
                    if staging_imported_timestamp.le(&last_imported_timestamp) {
                        println!("[Insert] {:#?} already imported", group_type);
                        return Ok(UpdateGroupSummary {
                            group_type,
                            updated: false,
                            status_label: String::from("already imported"),
                            started_timestamp,
                            finished_timestamp: Utc::now(),
                            planned_count: 1,
                            done_count: 0,
                            reference_timestamp: Utc::now(),
                        });
                    }
                }

                if staging_csv_file_timestamp.le(&staging_imported_timestamp) {
                    println!("[Insert] {:#?} already inserted", group_type);
                    return Ok(UpdateGroupSummary {
                        group_type,
                        updated: false,
                        status_label: String::from("already inserted"),
                        started_timestamp,
                        finished_timestamp: Utc::now(),
                        planned_count: 1,
                        done_count: 0,
                        reference_timestamp: Utc::now(),
                    });
                }
            }
        }

        // Get CSV path
        let mut csv_path = PathBuf::from(self.db_folder.clone());
        csv_path.push(metadata.file_name);
        csv_path.set_extension("csv");
        let absolute_csv_path = canonicalize(csv_path)
            .map_err(|io_error| Error::InvalidComponentInCSVPath { io_error })?;
        let csv_path_str = absolute_csv_path
            .into_os_string()
            .into_string()
            .map_err(|_| Error::InvalidCSVPath)?;

        group_type
            .get_updatable_model()
            .insert_in_staging(connectors, csv_path_str)?;

        group_metadata::set_staging_imported_timestamp(
            connectors,
            group_type,
            staging_csv_file_timestamp,
        )?;

        println!("[Insert] Finished insert of {:#?}", group_type);

        Ok(UpdateGroupSummary {
            group_type,
            updated: true,
            status_label: String::from("inserted"),
            started_timestamp,
            finished_timestamp: Utc::now(),
            planned_count: 1,
            done_count: 1,
            reference_timestamp: Utc::now(),
        })
    }
}
