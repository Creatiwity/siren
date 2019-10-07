mod runner;

use crate::connectors::ConnectorsBuilders;
use crate::models::metadata::common::GroupType;
use std::env;

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

    /// Path to the temp folder, you can set in environment variable as TEMP_FOLDER
    #[clap(long = "temp-folder")]
    temp_folder: Option<String>,

    /// Path to the file storage folder for this app, you can set in environment variable as FILE_FOLDER
    #[clap(long = "file-folder")]
    file_folder: Option<String>,

    /// Path to the file storage folder for the database, you can set in environment variable as DB_FOLDER
    /// Could be the same as FILE_FOLDER if this app and the database are on the same file system
    /// Files copied by this app inside FILE_FOLDER must be visible by the database in DB_FOLDER
    #[clap(long = "db-folder")]
    db_folder: Option<String>,

    #[clap(subcommand)]
    subcmd: Option<UpdateSubCommand>,
}

arg_enum! {
    #[derive(Debug)]
    enum CmdGroupType {
        UnitesLegales,
        Etablissements,
        All
    }
}

impl From<CmdGroupType> for Vec<GroupType> {
    fn from(group: CmdGroupType) -> Self {
        match group {
            CmdGroupType::UnitesLegales => vec![GroupType::UnitesLegales],
            CmdGroupType::Etablissements => vec![GroupType::Etablissements],
            CmdGroupType::All => vec![GroupType::UnitesLegales, GroupType::Etablissements],
        }
    }
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

pub fn run(flags: UpdateFlags, builders: ConnectorsBuilders) {
    let group_type: Vec<GroupType> = flags.group_type.into();

    let temp_folder = flags
        .temp_folder
        .unwrap_or_else(|| env::var("TEMP_FOLDER").unwrap_or(String::from("./data/temp")));

    let file_folder = flags
        .file_folder
        .unwrap_or_else(|| env::var("FILE_FOLDER").unwrap_or(String::from("./data/files")));

    let db_folder = flags
        .db_folder
        .unwrap_or_else(|| env::var("DB_FOLDER").unwrap_or(file_folder.clone()));

    let connectors = builders.create();
    let result: Result<(), runner::error::Error>;

    match flags.subcmd {
        Some(subcmd) => match subcmd {
            UpdateSubCommand::DownloadFile => {
                result =
                    runner::step_download_file(&group_type, &temp_folder, flags.force, &connectors);
            }
            UpdateSubCommand::UnzipFile => {
                result =
                    runner::step_unzip_file(&group_type, &temp_folder, &file_folder, &connectors);
            }
            UpdateSubCommand::InsertData => {
                result = runner::step_insert_data(&group_type, &db_folder, &connectors);
            }
            UpdateSubCommand::SwapData => {
                result = runner::step_swap_data(&group_type, flags.force, &connectors);
            }
            UpdateSubCommand::CleanFile => {
                result =
                    runner::step_clean_file(&group_type, &temp_folder, &file_folder, &connectors);
            }
        },
        None => {
            result = runner::update(
                &group_type,
                runner::Config {
                    force: flags.force,
                    data_only: flags.data_only,
                    temp_folder,
                    file_folder,
                    db_folder,
                },
                &connectors,
            )
        }
    }

    if let Err(error) = result {
        error.exit();
    }
}
