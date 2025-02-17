use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata;
use crate::models::update_metadata::common::{
    Step, SyntheticGroupType, UpdateMetadata, UpdateSummary,
};
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
    // Execute workflow
    let mut summary = UpdateSummary::default();

    summary.start(connectors, synthetic_group_type, config.force)?;

    let asynchronous = config.asynchronous;
    let mut thread_connectors = connectors.clone();

    let handle = task::spawn(async move {
        task::yield_now().await;

        execute_workflow_thread(
            workflow,
            synthetic_group_type,
            config,
            &mut thread_connectors,
            summary,
        )
        .await
    });

    if !asynchronous {
        handle.await??;
    }

    Ok(update_metadata::current_update(connectors)?)
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

        execute_step(
            step,
            &config,
            groups.as_slice(),
            connectors,
            &mut summary.step_delegate(step),
        )
        .await
        .or_else(|error| {
            error!("Errored: {}", error.to_string());

            update_metadata::error_update(connectors, error.to_string(), Utc::now())?;
            Err(error)
        })?;
    }

    summary.finish(connectors)?;

    debug!("Finished");

    Ok(())
}

fn build_workflow() -> Vec<Step> {
    vec![Step::UpdateData, Step::SwapData, Step::SyncInsee]
}
