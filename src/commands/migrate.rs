use crate::connectors::local::{database_url, pending_migrations, run_migrations};

#[derive(clap::Parser, Debug)]
pub struct MigrateFlags {
    /// Only list the pending migrations, exit with 1 if there is any
    #[clap(long = "check")]
    check: bool,
}

pub async fn run(flags: MigrateFlags) {
    let database_url = database_url();

    let result = tokio::task::block_in_place(|| {
        if flags.check {
            pending_migrations(&database_url)
        } else {
            run_migrations(&database_url)
        }
    });

    match result {
        Ok(versions) if flags.check && !versions.is_empty() => {
            println!("Pending migrations: {}", versions.join(", "));
            crate::telemetry::exit(1);
        }
        Ok(_) if flags.check => println!("No pending migration"),
        Ok(versions) if versions.is_empty() => println!("No pending migration"),
        Ok(versions) => println!("Applied migrations: {}", versions.join(", ")),
        Err(error) => {
            sentry::capture_message(&error, sentry::Level::Error);
            eprintln!("{error}");
            crate::telemetry::exit(1);
        }
    }
}
