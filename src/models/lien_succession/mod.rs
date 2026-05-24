pub mod common;
pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel, copy_remote_zipped_csv};
use super::schema::lien_succession::dsl;
use crate::connectors::{Connectors, local::Connection};
use crate::update::utils::remote_file::RemoteFile;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use common::LienSuccession;
use diesel::pg::{CopyFormat, CopyHeader};
use diesel::prelude::*;
use diesel::sql_query;
use diesel_async::{AsyncConnection, RunQueryDsl};
use error::Error;

pub async fn get(connection: &mut Connection, siret: &str) -> Result<Vec<LienSuccession>, Error> {
    dsl::lien_succession
        .select(LienSuccession::as_select())
        .filter(
            dsl::siret_etablissement_predecesseur
                .eq(siret)
                .or(dsl::siret_etablissement_successeur.eq(siret)),
        )
        .load::<LienSuccession>(connection)
        .await
        .map_err(|error| error.into())
}

pub struct LienSuccessionModel {}

#[async_trait]
impl UpdatableModel for LienSuccessionModel {
    async fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let mut connection = connectors.local.pool.get().await?;
        dsl::lien_succession
            .select(diesel::dsl::count(dsl::id))
            .first::<i64>(&mut connection)
            .await
            .map_err(|error| error.into())
    }

    async fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::lien_succession_staging::dsl;

        let mut connection = connectors.local.pool.get().await?;
        dsl::lien_succession_staging
            .select(diesel::dsl::count(dsl::id))
            .first::<i64>(&mut connection)
            .await
            .map_err(|error| error.into())
    }

    fn insert_remote_file_in_staging(
        &self,
        connectors: &Connectors,
        remote_file: RemoteFile,
    ) -> Result<bool, UpdatableError> {
        use super::schema::lien_succession_staging::dsl;
        use diesel::Connection as _;
        use diesel::ExecuteCopyFromDsl as SyncExecuteCopy;
        use diesel::RunQueryDsl as SyncRunQueryDsl;

        tokio::task::block_in_place(|| {
            let mut connection =
                diesel::pg::PgConnection::establish(&connectors.local.database_url)
                    .map_err(|_| UpdatableError::SyncConnectionFailed)?;

            SyncRunQueryDsl::execute(
                sql_query("TRUNCATE lien_succession_staging"),
                &mut connection,
            )
            .map_err(|e| UpdatableError::Database { source: e })?;

            let copy_query = diesel::copy_from(dsl::lien_succession_staging)
                .from_raw_data(
                    (
                        dsl::siret_etablissement_predecesseur,
                        dsl::siret_etablissement_successeur,
                        dsl::date_lien_succession,
                        dsl::transfert_siege,
                        dsl::continuite_economique,
                        dsl::date_dernier_traitement_lien_succession,
                    ),
                    |write| copy_remote_zipped_csv(remote_file.to_reader(), write),
                )
                .with_delimiter(',')
                .with_format(CopyFormat::Csv)
                .with_header(CopyHeader::Set(true));
            SyncExecuteCopy::execute(copy_query, &mut connection)
                .map(|count| count > 0)
                .map_err(|e| UpdatableError::Database { source: e })
        })
    }

    async fn swap(&self, connectors: &Connectors) -> Result<(), UpdatableError> {
        let mut connection = connectors.local.pool.get().await?;
        connection
            .transaction(async |conn| {
                sql_query("ALTER TABLE lien_succession RENAME TO lien_succession_temp")
                    .execute(conn)
                    .await?;
                sql_query("ALTER TABLE lien_succession_staging RENAME TO lien_succession")
                    .execute(conn)
                    .await?;
                sql_query("ALTER TABLE lien_succession_temp RENAME TO lien_succession_staging")
                    .execute(conn)
                    .await?;
                sql_query("TRUNCATE lien_succession_staging")
                    .execute(conn)
                    .await?;
                sql_query(
                    r#"
                    UPDATE group_metadata
                    SET last_imported_timestamp = staging_imported_timestamp
                    WHERE group_type = 'liens_succession'
                    "#,
                )
                .execute(conn)
                .await?;
                sql_query(
                    r#"
                    UPDATE group_metadata
                    SET staging_imported_timestamp = NULL
                    WHERE group_type = 'liens_succession'
                    "#,
                )
                .execute(conn)
                .await?;

                diesel::QueryResult::Ok(())
            })
            .await
            .map_err(|e| UpdatableError::Database { source: e })
    }

    async fn get_total_count(
        &self,
        connectors: &mut Connectors,
        start_timestamp: NaiveDateTime,
    ) -> Result<u32, UpdatableError> {
        let insee = connectors
            .insee
            .as_mut()
            .ok_or(UpdatableError::MissingInseeConnector)?;

        Ok(insee.get_total_liens_succession(start_timestamp).await?)
    }

    async fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        let mut connection = connectors.local.pool.get().await?;
        dsl::lien_succession
            .select(dsl::date_dernier_traitement_lien_succession)
            .order(dsl::date_dernier_traitement_lien_succession.desc())
            .filter(dsl::date_dernier_traitement_lien_succession.is_not_null())
            .first::<Option<NaiveDateTime>>(&mut connection)
            .await
            .map_err(|error| error.into())
    }

    async fn update_daily_data(
        &self,
        connectors: &mut Connectors,
        start_timestamp: NaiveDateTime,
        cursor: String,
    ) -> Result<(Option<String>, usize), UpdatableError> {
        let insee = connectors
            .insee
            .as_mut()
            .ok_or(UpdatableError::MissingInseeConnector)?;

        let (next_cursor, liens_succession) = insee
            .get_daily_liens_succession(start_timestamp, cursor)
            .await?;

        let mut connection = connectors.local.pool.get().await?;

        let updated_count = diesel::insert_into(dsl::lien_succession)
            .values(&liens_succession)
            .execute(&mut connection)
            .await?;

        Ok((next_cursor, updated_count))
    }
}
