use super::super::error::Error;
use super::common::Action;
use crate::connectors::Connectors;
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
        println!("[SyncInsee] Starting {:#?}", group_type);
        let started_timestamp = Utc::now();
        let mut result = String::new();

        if let Some(insee) = &connectors.insee {
            // Use Insee connector only if present
            println!("Insee access token: {}", insee.token);

            result = match group_type {
                GroupType::UnitesLegales => insee.get_daily_unites_legales()?,
                GroupType::Etablissements => insee.get_daily_etablissements()?,
            };
        }

        println!("Result: {}", result);

        println!("[SyncInsee] Finished for {:#?}", group_type);
        Ok(UpdateGroupSummary {
            group_type,
            updated: true,
            status_label: String::from("synced"),
            started_timestamp,
            finished_timestamp: Utc::now(),
        })
    }
}
