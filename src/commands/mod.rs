mod common;
mod serve;
mod update;

use crate::connectors::ConnectorsBuilders;
use clap::Parser;
use serve::ServeFlags;
use update::UpdateFlags;

/// Sirene service used to update data in database
/// and serve it through a HTTP REST API
#[derive(Parser, Debug)]
#[clap(version = "2.6.1", author = "Julien Blatecky")]
struct Opts {
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

    match opts.main_command {
        MainCommand::Update(update_flags) => update::run(update_flags, builders).await,
        MainCommand::Serve(serve_flags) => serve::run(serve_flags, builders).await,
    }
}
