mod common;
mod migrate;
mod serve;
mod update;

use crate::connectors::ConnectorsBuilders;
use crate::connectors::local::MigrationMode;
use clap::Parser;
use migrate::MigrateFlags;
use serve::ServeFlags;
use update::UpdateFlags;

/// Sirene service used to update data in database
/// and serve it through a HTTP REST API
#[derive(Parser, Debug)]
#[clap(author = "Julien Blatecky")]
struct Opts {
    /// Do not apply pending migrations on startup, only check that none is
    /// left (read-only queries, so DATABASE_URL may target a read replica).
    /// Run the `migrate` command beforehand.
    #[clap(long = "skip-migrations", env = "SKIP_MIGRATIONS", global = true)]
    skip_migrations: bool,

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

    /// Apply the pending database migrations, then exit
    #[clap(name = "migrate")]
    Migrate(MigrateFlags),
}

pub async fn run() {
    let opts = Opts::parse();

    let migration_mode = if opts.skip_migrations {
        MigrationMode::Check
    } else {
        MigrationMode::Run
    };

    match opts.main_command {
        MainCommand::Update(update_flags) => {
            update::run(update_flags, ConnectorsBuilders::new(migration_mode)).await
        }
        MainCommand::Serve(serve_flags) => {
            serve::run(serve_flags, ConnectorsBuilders::new(migration_mode)).await
        }
        MainCommand::Migrate(migrate_flags) => migrate::run(migrate_flags).await,
    }
}
