use super::super::error::Error;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::{Step, UpdateGroupSummary};
use chrono::Utc;

pub struct SwapAction {
    pub force: bool,
}

impl Action for SwapAction {
    fn step(&self) -> Step {
        Step::SwapData
    }

    fn execute(
        &self,
        group_type: GroupType,
        connectors: &Connectors,
    ) -> Result<UpdateGroupSummary, Error> {
        println!("[Insert] Swapping {:#?}", group_type);
        let started_timestamp = Utc::now();

        let metadata = group_metadata::get(connectors, group_type)?;

        // Swap only if inserted data are referenced in database
        let staging_imported_timestamp = match metadata.staging_imported_timestamp {
            Some(staging_imported_timestamp) => staging_imported_timestamp,
            None => {
                println!("[Swap] Nothing to swap for {:#?}", group_type);
                return Ok(UpdateGroupSummary {
                    group_type,
                    updated: false,
                    status_label: String::from("nothing to swap"),
                    started_timestamp,
                    finished_timestamp: Utc::now(),
                });
            }
        };

        // Test if not already swapped
        if !self.force {
            if let Some(last_imported_timestamp) = metadata.last_imported_timestamp {
                if staging_imported_timestamp.le(&last_imported_timestamp) {
                    println!("[Swap] {:#?} already imported", group_type);
                    return Ok(UpdateGroupSummary {
                        group_type,
                        updated: false,
                        status_label: String::from("already imported"),
                        started_timestamp,
                        finished_timestamp: Utc::now(),
                    });
                }
            }
        }

        let model = group_type.get_updatable_model();

        if !self.force {
            let count = model.count(connectors)? as f64;
            let count_staging = model.count_staging(connectors)? as f64;

            let max_count_staging = count * 1.01;
            let min_count_staging = count * 0.99;

            if count != 0.0
                && (count_staging < min_count_staging || max_count_staging < count_staging)
            {
                return Err(Error::SwapStoppedTooMuchDifference { group_type });
            }
        }

        model.swap(connectors)?;

        group_metadata::set_last_imported_timestamp(
            connectors,
            group_type,
            staging_imported_timestamp,
        )?;

        println!("[Insert] Swap of {:#?} finished", group_type);

        Ok(UpdateGroupSummary {
            group_type,
            updated: true,
            status_label: String::from("swapped"),
            started_timestamp,
            finished_timestamp: Utc::now(),
        })
    }
}
