pub mod runner;

use super::common::{CmdGroupType, FolderOptions};
use crate::connectors::ConnectorsBuilders;
use crate::models::metadata::common::GroupType;
use runner::common::UpdateStepSummary;

#[derive(Clap, Debug)]
pub struct UpdateFlags {
    /// Configure which part will be updated
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

#[derive(Clap, Debug)]
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

    /// Swap loader-table to production
    #[clap(name = "swap-data")]
    SwapData,

    /// Clean files from FILE_FOLDER
    #[clap(name = "clean-file")]
    CleanFile,
}

pub fn run(flags: UpdateFlags, folder_options: FolderOptions, builders: ConnectorsBuilders) {
    let group_type: Vec<GroupType> = flags.group_type.into();

    let connectors = builders.create();

    match flags.subcmd {
        Some(subcmd) => {
            let result: Result<UpdateStepSummary, runner::error::Error>;

            match subcmd {
                UpdateSubCommand::DownloadFile => {
                    result = runner::step_download_file(
                        &group_type,
                        &folder_options.temp,
                        flags.force,
                        &connectors,
                    );
                }
                UpdateSubCommand::UnzipFile => {
                    result = runner::step_unzip_file(
                        &group_type,
                        &folder_options.temp,
                        &folder_options.file,
                        flags.force,
                        &connectors,
                    );
                }
                UpdateSubCommand::InsertData => {
                    result = runner::step_insert_data(
                        &group_type,
                        &folder_options.db,
                        flags.force,
                        &connectors,
                    );
                }
                UpdateSubCommand::SwapData => {
                    result = runner::step_swap_data(&group_type, flags.force, &connectors);
                }
                UpdateSubCommand::CleanFile => {
                    result = runner::step_clean_file(
                        &group_type,
                        &folder_options.temp,
                        &folder_options.file,
                        &connectors,
                    );
                }
            }

            match result {
                Ok(summary) => println!("{}", serde_json::to_string(&summary).unwrap()),
                Err(error) => error.exit(),
            }
        }
        None => {
            match runner::update(
                &group_type,
                runner::Config {
                    force: flags.force,
                    data_only: flags.data_only,
                    temp_folder: folder_options.temp,
                    file_folder: folder_options.file,
                    db_folder: folder_options.db,
                },
                &connectors,
            ) {
                Ok(summary) => println!("{}", serde_json::to_string(&summary).unwrap()),
                Err(error) => error.exit(),
            }
        }
    }
}
