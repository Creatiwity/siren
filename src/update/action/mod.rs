use super::common::Config;
use super::error::Error;
use super::summary::SummaryStepDelegate;
use crate::connectors::Connectors;
use crate::models::group_metadata::common::GroupType;
use crate::models::update_metadata::common::Step;
use common::Action;

pub mod common;
pub mod swap;
pub mod sync_insee;
pub mod update_stock;

pub async fn execute_step<'a>(
    step: Step,
    config: &Config,
    groups: &[GroupType],
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
        Step::UpdateData => Box::new(update_stock::UpdateAction {
            force: config.force,
        }),
        Step::SwapData => Box::new(swap::SwapAction {
            force: config.force,
        }),
        Step::SyncInsee => Box::new(sync_insee::SyncInseeAction {}),
    }
}
