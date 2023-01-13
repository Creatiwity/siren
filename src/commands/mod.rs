mod common;
mod serve;
mod update;

use crate::connectors::ConnectorsBuilders;
use clap::Parser;
use common::FolderOptions;
use serve::ServeFlags;
use update::UpdateFlags;

/// Sirene service used to update data in database
/// and serve it through a HTTP REST API
#[derive(Parser, Debug)]
#[clap(version = "2.6.1", author = "Julien Blatecky")]
struct Opts {
    /// Path to the temp folder
    #[clap(long = "temp-folder", env, default_value = "./data/temp")]
    temp_folder: String,

    /// Path to the file storage folder for this app
    #[clap(long = "file-folder", env, default_value = "./data/files")]
    file_folder: String,

    /// Path to the file storage folder for the database.
    /// Could be the same as FILE_FOLDER if this app and the database are on the same file system.
    /// Files copied by this app inside FILE_FOLDER must be visible by the database in DB_FOLDER
    #[clap(long = "db-folder", env, default_value = "./data/files")]
    db_folder: String,

    #[clap(subcommand)]
    main_command: MainCommand,
}

#[derive(clap::Parser, Debug)]
enum MainCommand {
    /// Update data from CSV source files
    #[clap(name = "update")]
    Update(UpdateFlags),

    /// Serve data from database to /unites_legales/<siren> and /etablissements/<siret>
    #[clap(name = "serve")]
    Serve(ServeFlags),
}

pub async fn run(builders: ConnectorsBuilders) {
    let opts = Opts::parse();

    let folder_options = FolderOptions {
        temp: opts.temp_folder,
        file: opts.file_folder,
        db: opts.db_folder,
    };

    match opts.main_command {
        MainCommand::Update(update_flags) => {
            update::run(update_flags, folder_options, builders).await
        }
        MainCommand::Serve(serve_flags) => serve::run(serve_flags, folder_options, builders).await,
    }
}
