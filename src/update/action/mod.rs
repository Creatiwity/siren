use super::common::Config;
use super::error::Error;
use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::{Step, UpdateGroupSummary, UpdateStepSummary};
use chrono::Utc;
use common::Action;

pub mod clean;
pub mod common;
pub mod download_stock;
pub mod insert_stock;
pub mod swap;
pub mod unzip_stock;

pub fn execute_step(
    step: Step,
    config: &Config,
    groups: &Vec<GroupType>,
    connectors: &Connectors,
) -> Result<UpdateStepSummary, Error> {
    let started_timestamp = Utc::now();
    let mut groups_summary: Vec<UpdateGroupSummary> = vec![];
    let action = build_action(config, step);

    for group in groups {
        groups_summary.push(action.execute(*group, connectors)?);
    }

    Ok(UpdateStepSummary {
        step,
        updated: groups_summary.iter().find(|&g| g.updated).is_some(),
        started_timestamp,
        finished_timestamp: Utc::now(),
        groups: groups_summary,
    })
}

fn build_action(config: &Config, step: Step) -> Box<dyn Action> {
    match step {
        Step::DownloadFile => Box::new(download_stock::DownloadAction {
            temp_folder: config.temp_folder.clone(),
            force: config.force,
        }),
        Step::UnzipFile => Box::new(unzip_stock::UnzipAction {
            temp_folder: config.temp_folder.clone(),
            file_folder: config.file_folder.clone(),
            force: config.force,
        }),
        Step::InsertData => Box::new(insert_stock::InsertAction {
            db_folder: config.file_folder.clone(),
            force: config.force,
        }),
        Step::SwapData => Box::new(swap::SwapAction {
            force: config.force,
        }),
        Step::CleanFile => Box::new(clean::CleanAction {
            temp_folder: config.temp_folder.clone(),
            file_folder: config.file_folder.clone(),
        }),
    }
}
