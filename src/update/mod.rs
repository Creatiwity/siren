use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata;
use crate::models::update_metadata::common::{
    Step, SyntheticGroupType, UpdateMetadata, UpdateSummary,
};
use crate::sentry_crons::SentryCrons;
use action::execute_step;
use chrono::Utc;
use common::Config;
use error::Error;
use tokio::task;
use tracing::{debug, error};

pub mod action;
pub mod common;
pub mod error;
pub mod summary;
pub mod utils;

pub async fn update(
    synthetic_group_type: SyntheticGroupType,
    config: Config,
    connectors: &mut Connectors,
) -> Result<UpdateMetadata, Error> {
    // Build and execute workflow
    execute_workflow(build_workflow(), synthetic_group_type, config, connectors).await
}

pub async fn update_step(
    step: Step,
    synthetic_group_type: SyntheticGroupType,
    config: Config,
    connectors: &mut Connectors,
) -> Result<UpdateMetadata, Error> {
    // Execute step
    execute_workflow(vec![step], synthetic_group_type, config, connectors).await
}

async fn execute_workflow(
    workflow: Vec<Step>,
    synthetic_group_type: SyntheticGroupType,
    config: Config,
    connectors: &mut Connectors,
) -> Result<UpdateMetadata, Error> {
    let mut summary = UpdateSummary::default();

    summary
        .start(connectors, synthetic_group_type, config.force)
        .await?;

    let asynchronous = config.asynchronous;
    let sentry_crons = config
        .crontab
        .as_ref()
        .and_then(|tab| SentryCrons::for_update(synthetic_group_type).map(|c| (c, tab.clone())));
    let mut thread_connectors = connectors.clone();

    let handle = task::spawn(async move {
        task::yield_now().await;

        if let Some((crons, tab)) = sentry_crons.clone() {
            crons.notify_in_progress(tab).await;
        }

        let result = execute_workflow_thread(
            workflow,
            synthetic_group_type,
            config,
            &mut thread_connectors,
            summary,
        )
        .await;

        if let Some((crons, _)) = sentry_crons {
            if result.is_ok() {
                crons.notify_ok().await;
            } else {
                crons.notify_error().await;
            }
        }

        result
    });

    if !asynchronous {
        handle.await??;
    }

    Ok(update_metadata::current_update(connectors).await?)
}

async fn execute_workflow_thread(
    workflow: Vec<Step>,
    synthetic_group_type: SyntheticGroupType,
    config: Config,
    connectors: &mut Connectors,
    mut summary: UpdateSummary,
) -> Result<(), Error> {
    debug!("Starting");

    for step in workflow.into_iter() {
        let groups: Vec<GroupType> = synthetic_group_type.into();

        let result = execute_step(
            step,
            &config,
            groups.as_slice(),
            connectors,
            &mut summary.step_delegate(step),
        )
        .await;

        if let Err(err) = result {
            error!("Errored: {}", err.to_string());
            update_metadata::error_update(connectors, err.to_string(), Utc::now()).await?;
            return Err(err);
        }
    }

    summary.finish(connectors).await?;

    debug!("Finished");

    Ok(())
}

fn build_workflow() -> Vec<Step> {
    vec![Step::UpdateData, Step::SwapData, Step::SyncInsee]
}
