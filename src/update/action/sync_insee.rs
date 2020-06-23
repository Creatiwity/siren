use super::super::error::Error;
use super::common::Action;
use crate::connectors::Connectors;
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::{Step, UpdateGroupSummary};
use async_trait::async_trait;
use chrono::Utc;

pub struct SyncInseeAction {}

#[async_trait]
impl Action for SyncInseeAction {
    fn step(&self) -> Step {
        Step::SyncInsee
    }

    async fn execute(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
    ) -> Result<UpdateGroupSummary, Error> {
        println!("[SyncInsee] Syncing {:#?}", group_type);
        let started_timestamp = Utc::now();
        let status_label: String;
        let mut updated = false;

        // Use Insee connector only if present
        if connectors.insee.is_some() {
            let model = group_type.get_updatable_model();

            if let Some(timestamp) = model.get_last_insee_synced_timestamp(connectors)? {
                let updated_count = model.update_daily_data(connectors, timestamp).await?;
                println!("[SyncInsee] {} {:#?} synced", updated_count, group_type);

                group_metadata::set_last_insee_synced_timestamp(
                    connectors,
                    group_type,
                    Utc::now(),
                )?;

                updated = updated_count > 0;
                status_label = String::from("synced");
            } else {
                status_label = String::from("missing last treatment date");
            }
        } else {
            status_label = String::from("no insee connector configured");
        }

        println!("[SyncInsee] Syncing of {:#?} done", group_type);
        Ok(UpdateGroupSummary {
            group_type,
            updated,
            status_label,
            started_timestamp,
            finished_timestamp: Utc::now(),
        })
    }
}
