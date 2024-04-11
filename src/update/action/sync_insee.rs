use super::super::error::Error;
use super::super::summary::SummaryGroupDelegate;
use super::common::Action;
use crate::connectors::{insee::INITIAL_CURSOR, Connectors};
use crate::models::group_metadata;
use crate::models::group_metadata::common::GroupType;
use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use tracing::debug;

pub struct SyncInseeAction {}

#[async_trait]
impl Action for SyncInseeAction {
    async fn execute<'a, 'b>(
        &self,
        group_type: GroupType,
        connectors: &mut Connectors,
        summary_delegate: &'b mut SummaryGroupDelegate<'a, 'b>,
    ) -> Result<(), Error> {
        debug!("Syncing {:#?}", group_type);

        // Use Insee connector only if present
        if connectors.insee.is_some() {
            let model = group_type.get_updatable_model();

            if let Some(last_timestamp) = model.get_last_insee_synced_timestamp(connectors)? {
                let mut current_cursor: Option<String> = Some(INITIAL_CURSOR.to_string());
                let mut updated_count = 0;
                let timestamp = get_minimum_timestamp_for_request(last_timestamp);

                let planned_count = model.get_total_count(connectors, timestamp).await?;

                summary_delegate.start(
                    connectors,
                    Some(DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc)),
                    planned_count,
                )?;

                debug!("Syncing {} {:#?}...", planned_count, group_type);

                while let Some(cursor) = current_cursor {
                    let (next_cursor, inserted_count) = model
                        .update_daily_data(connectors, timestamp, cursor)
                        .await?;

                    current_cursor = next_cursor;
                    updated_count += inserted_count;

                    summary_delegate.progress(connectors, updated_count as u32)?;
                }

                debug!("{} {:#?} synced", updated_count, group_type);

                group_metadata::set_last_insee_synced_timestamp(
                    connectors,
                    group_type,
                    Utc::now(),
                )?;

                summary_delegate.finish(
                    connectors,
                    String::from("synced"),
                    updated_count as u32,
                    updated_count > 0,
                )?;
            } else {
                summary_delegate.finish(
                    connectors,
                    String::from("missing last treatment date"),
                    0,
                    false,
                )?;
            }
        } else {
            summary_delegate.finish(
                connectors,
                String::from("no insee connector configured"),
                0,
                false,
            )?;
        }

        debug!("Syncing of {:#?} done", group_type);

        Ok(())
    }
}

fn get_minimum_timestamp_for_request(timestamp: NaiveDateTime) -> NaiveDateTime {
    timestamp.max(Utc::now().naive_local() - Duration::days(31))
}
