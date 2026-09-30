use super::common::CmdGroupType;
use crate::connectors::ConnectorsBuilders;
use crate::models::update_metadata::common::{Step, SyntheticGroupType};
use crate::models::update_metadata::error_update;
use crate::update::{common::Config, error::Error, update, update_step};
use chrono::Utc;

#[derive(clap::Parser, Debug)]
pub struct UpdateFlags {
    /// Configure which part will be updated
    #[clap(value_enum)]
    group_type: CmdGroupType,

    /// Force update even if the source data where not updated
    #[clap(long = "force")]
    force: bool,

    /// Crontab expression that triggered this execution (for Sentry Crons monitoring)
    #[clap(long = "crontab")]
    crontab: Option<String>,

    #[cfg(feature = "geocoding")]
    #[clap(flatten)]
    geocoding: GeocodingFlags,

    #[clap(subcommand)]
    subcmd: Option<UpdateSubCommand>,
}

/// The geocoding index sync: `update geocoding`, and after `update all`.
#[cfg(feature = "geocoding")]
#[derive(clap::Args, Debug)]
struct GeocodingFlags {
    /// Geocoding index file to build or refresh (the one `serve` reads)
    #[clap(long = "geocoding-index-path", env = "GEOCODING_INDEX_PATH")]
    index_path: Option<std::path::PathBuf>,

    /// Source of the index: URL or local path, gzipped if it ends with .gz
    #[clap(
        long = "geocoding-source-url",
        env = "GEOCODING_SOURCE_URL",
        default_value = crate::geocoding::DEFAULT_SOURCE_URL
    )]
    source_url: String,

    /// Source format: addok (BAN exports) or bano
    #[clap(
        long = "geocoding-source-format",
        env = "GEOCODING_SOURCE_FORMAT",
        default_value = "addok"
    )]
    source_format: String,

    /// Text processing profile of the index
    #[clap(
        long = "geocoding-profile",
        env = "GEOCODING_PROFILE",
        default_value = "fr"
    )]
    profile: String,

    /// Also sync the geocoding index after `update all` (when an index path is set)
    #[clap(
        long = "geocoding-with-all",
        env = "GEOCODING_WITH_UPDATE_ALL",
        default_value_t = true,
        action = clap::ArgAction::Set
    )]
    with_all: bool,
}

#[derive(clap::Subcommand, Debug)]
enum UpdateSubCommand {
    /// Download, unzip and load CSV file in database in loader-table
    #[clap(name = "update-data")]
    UpdateData,

    /// Swap loader-table to production
    #[clap(name = "swap-data")]
    SwapData,

    /// Synchronise daily data from INSEE since the last modification
    #[clap(name = "sync-insee")]
    SyncInsee,

    /// Set a staled update process to error, use only if the process is really stopped
    #[clap(name = "finish-error")]
    FinishError,
}

pub async fn run(flags: UpdateFlags, builders: ConnectorsBuilders) {
    #[cfg(feature = "geocoding")]
    if matches!(flags.group_type, CmdGroupType::Geocoding) {
        if flags.subcmd.is_some() {
            eprintln!("The geocoding index has no steps: run `update geocoding` alone");
            crate::telemetry::exit(1);
        }
        sync_geocoding(&flags.geocoding, flags.force, true).await;
        return;
    }
    #[cfg(feature = "geocoding")]
    let geocoding_after = matches!(flags.group_type, CmdGroupType::All)
        && flags.subcmd.is_none()
        && flags.geocoding.with_all;

    let mut connectors = builders
        .create_with_insee()
        .expect("Unable to create INSEE connector");
    let synthetic_group_type: SyntheticGroupType = flags.group_type.into();

    // Prepare config
    let config = Config {
        force: flags.force,
        asynchronous: false,
        crontab: flags.crontab,
    };

    let summary_result = match flags.subcmd {
        Some(subcmd) => {
            let step = match subcmd {
                UpdateSubCommand::UpdateData => Step::UpdateData,
                UpdateSubCommand::SwapData => Step::SwapData,
                UpdateSubCommand::SyncInsee => Step::SyncInsee,
                UpdateSubCommand::FinishError => {
                    if let Err(err) = error_update(
                        &connectors,
                        "Process stopped manually.".to_string(),
                        Utc::now(),
                    )
                    .await
                    {
                        let error: Error = err.into();
                        error.exit()
                    }

                    crate::telemetry::exit(0);
                }
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
        Err(error) => {
            sentry::capture_error(&error);
            error.exit()
        }
    }

    #[cfg(feature = "geocoding")]
    if geocoding_after {
        sync_geocoding(&flags.geocoding, flags.force, false).await;
    }
}

/// Build or refresh the geocoding index. `required`: fail when no index path
/// is configured (explicit `update geocoding`), rather than skip.
#[cfg(feature = "geocoding")]
async fn sync_geocoding(flags: &GeocodingFlags, force: bool, required: bool) {
    use crate::geocoding::sync::{SyncConfig, sync};

    let Some(index_path) = flags.index_path.clone() else {
        if required {
            eprintln!(
                "No geocoding index path: set GEOCODING_INDEX_PATH or --geocoding-index-path"
            );
            crate::telemetry::exit(1);
        }
        tracing::debug!("No geocoding index path configured, skipping the geocoding sync");
        return;
    };
    let config = SyncConfig {
        index_path,
        source_url: flags.source_url.clone(),
        source_format: flags.source_format.clone(),
        profile: flags.profile.clone(),
        force,
    };
    match sync(&config).await {
        Ok(outcome) => println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "geocoding": outcome }))
                .expect("Unable to stringify geocoding outcome")
        ),
        Err(error) => {
            sentry::capture_error(&error);
            tracing::error!("{error}");
            crate::telemetry::exit(1);
        }
    }
}
