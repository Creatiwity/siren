mod serve;
mod update;

use crate::connectors::ConnectorsBuilders;
use serve::ServeFlags;
use update::UpdateFlags;

/// Sirene service used to update data in database
/// and serve it through a HTTP REST API
#[derive(Clap, Debug)]
#[clap(version = "1.0.0", author = "Julien Blatecky")]
struct Opts {
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

pub fn run(builders: ConnectorsBuilders) {
    let opts = Opts::parse();

    match opts.main_command {
        MainCommand::Update(update_flags) => update::run(update_flags, builders),
        MainCommand::Serve(serve_flags) => serve::run(serve_flags, builders),
    }
}
