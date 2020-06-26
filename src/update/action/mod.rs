use super::common::Config;
use super::error::Error;
use super::summary::SummaryStepDelegate;
use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::Step;
use common::Action;

pub mod clean;
pub mod common;
pub mod download_stock;
pub mod insert_stock;
pub mod swap;
pub mod sync_insee;
pub mod unzip_stock;

pub async fn execute_step<'a>(
    step: Step,
    config: &Config,
    groups: &Vec<GroupType>,
    connectors: &mut Connectors,
    summary_delegate: &'a mut SummaryStepDelegate<'a>,
) -> Result<(), Error> {
    let action = build_action(config, step);

    summary_delegate.start(connectors)?;

    for group in groups {
        action
            .execute(
                *group,
                connectors,
                &mut summary_delegate.group_delegate(*group),
            )
            .await?;
    }

    summary_delegate.finish(connectors)?;

    Ok(())
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
        Step::SyncInsee => Box::new(sync_insee::SyncInseeAction {}),
    }
}
