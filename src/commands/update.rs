use super::common::{CmdGroupType, FolderOptions};
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

    /// Use an existing CSV file already present in FILE_FOLDER and does not delete it
    #[clap(long = "data-only")]
    data_only: bool,

    #[clap(subcommand)]
    subcmd: Option<UpdateSubCommand>,
}

#[derive(clap::Subcommand, Debug)]
enum UpdateSubCommand {
    /// Download file in TEMP_FOLDER
    #[clap(name = "download-file")]
    DownloadFile,

    /// Unzip file from TEMP_FOLDER, and move it to the FILE_FOLDER
    #[clap(name = "unzip-file")]
    UnzipFile,

    /// Load CSV file in database in loader-table from DB_FOLDER
    #[clap(name = "insert-data")]
    InsertData,

    /// Unzip and load CSV file in database in loader-table from TEMP_FOLDER
    #[clap(name = "unzip-insert-data")]
    UnzipInsertData,

    /// Download, unzip and load CSV file in database in loader-table
    #[clap(name = "update-data")]
    UpdateData,

    /// Swap loader-table to production
    #[clap(name = "swap-data")]
    SwapData,

    /// Clean files from FILE_FOLDER
    #[clap(name = "clean-file")]
    CleanFile,

    /// Synchronise daily data from INSEE since the last modification
    #[clap(name = "sync-insee")]
    SyncInsee,

    /// Set a staled update process to error, use only if the process is really stopped
    #[clap(name = "finish-error")]
    FinishError,
}

pub async fn run(flags: UpdateFlags, folder_options: FolderOptions, builders: ConnectorsBuilders) {
    let mut connectors = builders
        .create_with_insee()
        .await
        .expect("Unable to create INSEE connector");
    let synthetic_group_type: SyntheticGroupType = flags.group_type.into();

    // Prepare config
    let config = Config {
        force: flags.force,
        data_only: flags.data_only,
        temp_folder: folder_options.temp,
        file_folder: folder_options.file,
        db_folder: folder_options.db,
        asynchronous: false,
    };

    let summary_result = match flags.subcmd {
        Some(subcmd) => {
            let step = match subcmd {
                UpdateSubCommand::DownloadFile => Step::DownloadFile,
                UpdateSubCommand::UnzipFile => Step::UnzipFile,
                UpdateSubCommand::InsertData => Step::InsertData,
                UpdateSubCommand::UnzipInsertData => Step::UnzipInsertData,
                UpdateSubCommand::UpdateData => Step::UpdateData,
                UpdateSubCommand::SwapData => Step::SwapData,
                UpdateSubCommand::CleanFile => Step::CleanFile,
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
