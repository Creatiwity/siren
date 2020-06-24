use super::common::{CmdGroupType, FolderOptions};
use crate::connectors::ConnectorsBuilders;
use crate::models::update_metadata::common::{Step, SyntheticGroupType};
use crate::update::{common::Config, update, update_step};

#[derive(Clap, Debug)]
pub struct UpdateFlags {
    /// Configure which part will be updated, unites-legales, etablissements or all
    #[clap(arg_enum)]
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

    /// Synchronise daily data from INSEE since the last modification
    #[clap(name = "sync-insee")]
    SyncInsee,
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
    };

    let summary_result = match flags.subcmd {
        Some(subcmd) => {
            let step = match subcmd {
                UpdateSubCommand::DownloadFile => Step::DownloadFile,
                UpdateSubCommand::UnzipFile => Step::UnzipFile,
                UpdateSubCommand::InsertData => Step::InsertData,
                UpdateSubCommand::SwapData => Step::SwapData,
                UpdateSubCommand::CleanFile => Step::CleanFile,
                UpdateSubCommand::SyncInsee => Step::SyncInsee,
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
