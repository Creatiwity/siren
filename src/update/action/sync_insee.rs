use super::super::error::Error;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::{Step, UpdateGroupSummary};
use chrono::Utc;

pub struct SyncInseeAction {}

impl Action for SyncInseeAction {
    fn step(&self) -> Step {
        Step::SyncInsee
    }

    fn execute(
        &self,
        group_type: GroupType,
        connectors: &Connectors,
    ) -> Result<UpdateGroupSummary, Error> {
        println!("[SyncInsee] Syncing {:#?}", group_type);
        let started_timestamp = Utc::now();
        let mut updated = false;
        let mut status_label = String::from("missing insee connector");

        if let Some(insee) = &connectors.insee {
            // Use Insee connector only if present
            println!("Insee access token: {}", insee.token);

            let model = group_type.get_updatable_model();

            status_label = String::from("missing last treatment date");

            if let Some(timestamp) = model.get_last_insee_synced_timestamp(connectors)? {
                model.update_daily_data(connectors, timestamp)?;

                group_metadata::set_last_insee_synced_timestamp(
                    connectors,
                    group_type,
                    Utc::now(),
                )?;

                updated = true;
                status_label = String::from("synced");
            }
        }

        println!("[SyncInsee] {:#?} synced", group_type);
        Ok(UpdateGroupSummary {
            group_type,
            updated,
            status_label,
            started_timestamp,
            finished_timestamp: Utc::now(),
        })
    }
}
