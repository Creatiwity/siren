use super::common::{CmdGroupType, FolderOptions};
use crate::connectors::ConnectorsBuilders;
use crate::update::{common::Config, update};

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
}

pub fn run(flags: UpdateFlags, folder_options: FolderOptions, builders: ConnectorsBuilders) {
    let connectors = builders.create();

    match update(
        flags.group_type.into(),
        Config {
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
