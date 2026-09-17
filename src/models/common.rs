use std::io::{Read, Seek, Write};

use crate::connectors::Connectors;
use crate::connectors::insee::error::InseeUpdate;
use crate::update::utils::remote_file::RemoteFile;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use custom_error::custom_error;
use diesel_async::pooled_connection::deadpool::PoolError;
use tracing::debug;

#[async_trait]
pub trait UpdatableModel: Sync + Send {
    async fn count(&self, connectors: &Connectors) -> Result<i64, Error>;
    async fn count_staging(&self, connectors: &Connectors) -> Result<i64, Error>;
    fn insert_remote_file_in_staging(
        &self,
        connectors: &Connectors,
        remote_file: RemoteFile,
    ) -> Result<bool, Error>;
    async fn swap(&self, connectors: &Connectors) -> Result<(), Error>;
    async fn get_total_count(
        &self,
        connectors: &mut Connectors,
        start_timestamp: NaiveDateTime,
    ) -> Result<u32, Error>;
    async fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, Error>;
    async fn update_daily_data(
        &self,
        connectors: &mut Connectors,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, usize), Error>;

    /// Source key in `search_lexicon`, or `None` for models without text
    /// search.
    fn search_source(&self) -> Option<&'static str> {
        None
    }

    /// Refreshes the correction lexicon, the commune dimension and the planner
    /// statistics.
    ///
    /// `None` rebuilds everything, after the monthly stock swap. `Some(_)` only
    /// merges rows touched since that timestamp, after the daily Insee sync.
    ///
    /// Derived from [`Self::search_source`]: a model without a search source has
    /// nothing to refresh.
    async fn refresh_search_metadata(
        &self,
        connectors: &Connectors,
        since: Option<NaiveDateTime>,
    ) -> Result<(), Error> {
        match self.search_source() {
            Some(source) => refresh_search_metadata(connectors, source, since).await,
            None => Ok(()),
        }
    }
}

/// The source name doubles as the table name, so `ANALYZE` needs no extra
/// argument.
async fn refresh_search_metadata(
    connectors: &Connectors,
    source: &'static str,
    since: Option<NaiveDateTime>,
) -> Result<(), Error> {
    use diesel::sql_query;
    use diesel::sql_types::{Text, Timestamp};
    use diesel_async::RunQueryDsl as _;

    let mut connection = connectors.local.pool.get().await?;

    match since {
        Some(since) => {
            sql_query("SELECT public.search_refresh_incremental($1, $2)")
                .bind::<Text, _>(source)
                .bind::<Timestamp, _>(since)
                .execute(&mut connection)
                .await?;
        }
        None => {
            sql_query("SELECT public.search_refresh_full($1)")
                .bind::<Text, _>(source)
                .execute(&mut connection)
                .await?;

            // The swap is a RENAME: the table now in production carries no
            // representative statistics, and the planner falls back to defaults
            // that pick sequential scans over the GIN predicates.
            sql_query(format!("ANALYZE {source}"))
                .execute(&mut connection)
                .await?;

            // A full rebuild replaces every row of the source and leaves as many
            // dead tuples behind. The function already analyzed the result, so
            // plain VACUUM suffices — and unlike VACUUM FULL it blocks neither
            // reads nor writes.
            sql_query("VACUUM search_lexicon")
                .execute(&mut connection)
                .await?;
        }
    }

    Ok(())
}

/// Loads a staging table with its indexes dropped, then rebuilds them.
///
/// Maintaining indexes row by row during a bulk load costs far more than
/// rebuilding them in one pass: 36.9 s down to 14.2 s on 1M establishment rows.
/// The primary key is kept — cheap to maintain while loading, 4.9 s to rebuild.
///
/// Definitions are read back from the catalogue rather than hardcoded, so what
/// the migration created is exactly what is restored.
///
/// Wrapped in a transaction: a failed load brings the indexes back. Otherwise a
/// crash at the wrong moment would leave a staging table without indexes, which
/// the swap would promote to production.
pub fn load_staging_without_indexes<F>(
    connection: &mut diesel::pg::PgConnection,
    staging_table: &str,
    load: F,
) -> Result<bool, Error>
where
    F: FnOnce(&mut diesel::pg::PgConnection) -> Result<bool, Error>,
{
    use diesel::Connection as _;
    use diesel::RunQueryDsl as _;
    use diesel::sql_types::Text;

    #[derive(diesel::QueryableByName)]
    struct StagingIndex {
        #[diesel(sql_type = Text)]
        indexname: String,
        #[diesel(sql_type = Text)]
        indexdef: String,
    }

    connection.transaction(|connection| {
        // Constraint-backed indexes stay: dropping them would mean dropping the
        // constraint itself.
        let indexes: Vec<StagingIndex> = diesel::sql_query(
            "SELECT i.indexname, i.indexdef \
               FROM pg_indexes i \
              WHERE i.schemaname = 'public' \
                AND i.tablename = $1 \
                AND NOT EXISTS ( \
                      SELECT 1 FROM pg_constraint c \
                       WHERE c.conindid = format('%I.%I', i.schemaname, i.indexname)::regclass \
                    ) \
              ORDER BY i.indexname",
        )
        .bind::<Text, _>(staging_table)
        .load(connection)?;

        for index in &indexes {
            diesel::sql_query(format!("DROP INDEX {}", quote_identifier(&index.indexname)))
                .execute(connection)?;
        }

        let inserted = load(connection)?;

        diesel::sql_query("SET LOCAL maintenance_work_mem = '1GB'").execute(connection)?;
        for index in &indexes {
            diesel::sql_query(&index.indexdef).execute(connection)?;
        }

        debug!("{} index reconstruits sur {}", indexes.len(), staging_table);

        Ok(inserted)
    })
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

pub fn copy_remote_zipped_csv(
    zip_file: impl Read + Seek,
    write: &mut dyn Write,
) -> Result<(), diesel::result::Error> {
    let mut archive = zip::ZipArchive::new(zip_file).map_err(|zip_error| {
        diesel::result::Error::DeserializationError(Box::new(Error::ZipDecode { zip_error }))
    })?;

    if archive.len() != 1 {
        return Err(diesel::result::Error::DeserializationError(Box::new(
            Error::ZipFormat,
        )));
    }

    let mut zipped_csv_file = archive.by_index(0).map_err(|zip_error| {
        diesel::result::Error::DeserializationError(Box::new(Error::ZipAccessFile { zip_error }))
    })?;

    debug!(
        "Unzipping and inserting extracted to database ({} bytes)",
        zipped_csv_file.size()
    );

    std::io::copy(&mut zipped_csv_file, write).map_err(|io_error| {
        diesel::result::Error::DeserializationError(Box::new(Error::FileCSVRead { io_error }))
    })?;

    diesel::QueryResult::Ok(())
}

custom_error! { pub Error
    LocalConnectionFailed{source: PoolError} = "Unable to connect to local database ({source}).",
    SyncConnectionFailed = "Unable to establish sync database connection.",
    Database{source: diesel::result::Error} = "Unable to run some operations on updatable model ({source}).",
    Update {source: InseeUpdate} = "{source}",
    MissingInseeConnector = "Missing required Insee connector",
    ZipOpen {io_error: std::io::Error} = "Unable to open data zip file ({io_error})",
    ZipDecode {zip_error: zip::result::ZipError} = "Unable to decode zip file ({zip_error})",
    ZipFormat = "Archive has more than one file inside it, you should review it before running it again",
    ZipAccessFile {zip_error: zip::result::ZipError} = "Unable to open file in archive ({zip_error})",
    FileCSVRead {io_error: std::io::Error} = "Unable to read CSV file from archive ({io_error})",
}
