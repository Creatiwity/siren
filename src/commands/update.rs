use super::common::CmdGroupType;
use crate::connectors::ConnectorsBuilders;
use crate::models::update_metadata::common::{Step, SyntheticGroupType};
use crate::models::update_metadata::error_update;
use crate::update::{common::Config, error::Error, update, update_step};
use chrono::Utc;

#[derive(clap::Parser, Debug)]
pub struct UpdateFlags {
    /// Configure which part will be updated
    #[clap(value_enum)]
    group_type: CmdGroupType,

    /// Force update even if the source data where not updated
    #[clap(long = "force")]
    force: bool,

    #[clap(subcommand)]
    subcmd: Option<UpdateSubCommand>,
}

#[derive(clap::Subcommand, Debug)]
enum UpdateSubCommand {
    /// Download, unzip and load CSV file in database in loader-table
    #[clap(name = "update-data")]
    UpdateData,

    /// Swap loader-table to production
    #[clap(name = "swap-data")]
    SwapData,

    /// Synchronise daily data from INSEE since the last modification
    #[clap(name = "sync-insee")]
    SyncInsee,

    /// Set a staled update process to error, use only if the process is really stopped
    #[clap(name = "finish-error")]
    FinishError,
}

pub async fn run(flags: UpdateFlags, builders: ConnectorsBuilders) {
    let mut connectors = builders
        .create_with_insee()
        .await
        .expect("Unable to create INSEE connector");
    let synthetic_group_type: SyntheticGroupType = flags.group_type.into();

    // Prepare config
    let config = Config {
        force: flags.force,
        asynchronous: false,
    };

    let summary_result = match flags.subcmd {
        Some(subcmd) => {
            let step = match subcmd {
                UpdateSubCommand::UpdateData => Step::UpdateData,
                UpdateSubCommand::SwapData => Step::SwapData,
                UpdateSubCommand::SyncInsee => Step::SyncInsee,
                UpdateSubCommand::FinishError => {
                    if let Err(err) = error_update(
                        &connectors,
                        "Process stopped manually.".to_string(),
                        Utc::now(),
                    ) {
                        let error: Error = err.into();
                        error.exit()
                    }

                    std::process::exit(0);
                }
            };

            update_step(step, synthetic_group_type, config, &mut connectors).await
        }
        None => update(synthetic_group_type, config, &mut connectors).await,
    };

    match summary_result {
        Ok(summary) => println!(
            "{}",
            serde_json::to_string_pretty(&summary).expect("Unable to stringify summary")
        ),
        Err(error) => error.exit(),
    }
}
