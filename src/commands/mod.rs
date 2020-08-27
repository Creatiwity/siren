mod common;
mod serve;
mod update;

use crate::connectors::ConnectorsBuilders;
use clap::Clap;
use common::FolderOptions;
use serve::ServeFlags;
use std::env;
use update::UpdateFlags;

/// Sirene service used to update data in database
/// and serve it through a HTTP REST API
#[derive(Clap, Debug)]
#[clap(version = "2.0.0", author = "Julien Blatecky")]
struct Opts {
    /// Path to the temp folder, you can set in environment variable as TEMP_FOLDER
    #[clap(long = "temp-folder")]
    temp_folder: Option<String>,

    /// Path to the file storage folder for this app, you can set in environment variable as FILE_FOLDER
    #[clap(long = "file-folder")]
    file_folder: Option<String>,

    /// Path to the file storage folder for the database, you can set in environment variable as DB_FOLDER.
    /// Could be the same as FILE_FOLDER if this app and the database are on the same file system.
    /// Files copied by this app inside FILE_FOLDER must be visible by the database in DB_FOLDER
    #[clap(long = "db-folder")]
    db_folder: Option<String>,

    #[clap(subcommand)]
    main_command: MainCommand,
}

#[derive(Clap, Debug)]
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

    let temp_folder = opts
        .temp_folder
        .unwrap_or_else(|| env::var("TEMP_FOLDER").unwrap_or(String::from("./data/temp")));

    let file_folder = opts
        .file_folder
        .unwrap_or_else(|| env::var("FILE_FOLDER").unwrap_or(String::from("./data/files")));

    let db_folder = opts
        .db_folder
        .unwrap_or_else(|| env::var("DB_FOLDER").unwrap_or(file_folder.clone()));

    let folder_options = FolderOptions {
        temp: temp_folder,
        file: file_folder,
        db: db_folder,
    };

    match opts.main_command {
        MainCommand::Update(update_flags) => {
            update::run(update_flags, folder_options, builders).await
        }
        MainCommand::Serve(serve_flags) => serve::run(serve_flags, folder_options, builders).await,
    }
}
