use crate::connectors::Connectors;
use crate::models::update_metadata;
use crate::models::update_metadata::common::{Step, SyntheticGroupType, UpdateSummary};
use action::execute_step;
use chrono::Utc;
use common::Config;
use error::Error;
use summary::Summary;

pub mod action;
pub mod common;
pub mod error;
pub mod summary;

pub async fn update(
    synthetic_group_type: SyntheticGroupType,
    config: Config,
    connectors: &mut Connectors,
) -> Result<UpdateSummary, Error> {
    // Build and execute workflow
    execute_workflow(
        build_workflow(&config),
        synthetic_group_type,
        config,
        connectors,
    )
    .await
}

pub async fn update_step(
    step: Step,
    synthetic_group_type: SyntheticGroupType,
    config: Config,
    connectors: &mut Connectors,
) -> Result<UpdateSummary, Error> {
    // Execute step
    execute_workflow(vec![step], synthetic_group_type, config, connectors).await
}

async fn execute_workflow(
    workflow: Vec<Step>,
    synthetic_group_type: SyntheticGroupType,
    config: Config,
    connectors: &mut Connectors,
) -> Result<UpdateSummary, Error> {
    // Start
    println!("[Update] Starting");

    // Execute workflow
    let mut summary = Summary::new();

    summary.start(
        connectors,
        synthetic_group_type,
        config.force,
        config.data_only,
    )?;

    for step in workflow.into_iter() {
        if let Err(error) = execute_step(
            step,
            &config,
            &synthetic_group_type.into(),
            connectors,
            &mut summary.step_delegate(step),
        )
        .await
        {
            update_metadata::error_update(connectors, error.to_string(), Utc::now())?;
            return Err(error);
        }
    }

    summary.finish(connectors);

    // End
    println!("[Update] Finished");

    Ok(summary)
}

fn build_workflow(config: &Config) -> Vec<Step> {
    let mut workflow: Vec<Step> = vec![];

    if !config.data_only {
        workflow.push(Step::DownloadFile);
        // If INSEE && newly downloaded file, get update date from INSEE and update
        workflow.push(Step::UnzipFile);
    }

    workflow.push(Step::InsertData);
    workflow.push(Step::SwapData);

    if !config.data_only {
        workflow.push(Step::CleanFile);
    }

    // If INSEE, download and insert daily modifications
    workflow.push(Step::SyncInsee);

    workflow
}
