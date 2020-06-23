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
    // Register start
    update_metadata::launch_update(
        connectors,
        synthetic_group_type,
        config.force,
        config.data_only,
    )?;

    // Start
    println!("[Update] Starting");
    let started_timestamp = Utc::now();

    // Execute workflow
    let mut summary = Summary::new();

    for step in workflow.into_iter() {
        execute_step(
            step,
            &config,
            &synthetic_group_type.into(),
            connectors,
            &mut summary,
        )
        .await;

        if summary.error.is_some() {
            break;
        }
    }

    if let Some(error) = summary.error {
        update_metadata::error_update(connectors, error.to_string(), Utc::now())?;
        return Err(error);
    }

    // End
    println!("[Update] Finished");
    let summary = UpdateSummary {
        updated: summary.steps.iter().find(|&s| s.updated).is_some(),
        started_timestamp,
        finished_timestamp: Utc::now(),
        steps: summary.steps,
    };
    update_metadata::finished_update(connectors, summary.clone())?;

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
