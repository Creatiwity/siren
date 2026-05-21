pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel, copy_remote_zipped_csv};
use super::schema::siren_doublons::dsl;
use crate::connectors::{Connectors, local::Connection};
use crate::update::utils::remote_file::RemoteFile;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use diesel::pg::{CopyFormat, CopyHeader};
use diesel::prelude::*;
use diesel::sql_query;
use error::Error;

pub fn find_canonical_siren(
    connection: &mut Connection,
    siren_doublon: &str,
) -> Result<Option<String>, Error> {
    dsl::siren_doublons
        .filter(dsl::siren_doublon.eq(siren_doublon))
        .order(dsl::date_dernier_traitement.desc().nulls_last())
        .select(dsl::siren)
        .first::<String>(connection)
        .optional()
        .map_err(|error| error.into())
}

pub struct SirenDoublonModel {}

#[async_trait]
impl UpdatableModel for SirenDoublonModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        dsl::siren_doublons
            .select(diesel::dsl::count_star())
            .first::<i64>(&mut connection)
            .map_err(|error| error.into())
    }

    fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::siren_doublons_staging::dsl as staging_dsl;

        let mut connection = connectors.local.pool.get()?;
        staging_dsl::siren_doublons_staging
            .select(diesel::dsl::count_star())
            .first::<i64>(&mut connection)
            .map_err(|error| error.into())
    }

    fn insert_remote_file_in_staging(
        &self,
        connectors: &Connectors,
        remote_file: RemoteFile,
    ) -> Result<bool, UpdatableError> {
        use super::schema::siren_doublons_staging::dsl as staging_dsl;

        let mut connection = connectors.local.pool.get()?;

        sql_query("TRUNCATE siren_doublons_staging").execute(&mut connection)?;

        diesel::copy_from(staging_dsl::siren_doublons_staging)
            .from_raw_data(
                (
                    staging_dsl::siren,
                    staging_dsl::siren_doublon,
                    staging_dsl::date_dernier_traitement,
                ),
                |write| copy_remote_zipped_csv(remote_file.to_reader(), write),
            )
            .with_delimiter(',')
            .with_format(CopyFormat::Csv)
            .with_header(CopyHeader::Set(true))
            .execute(&mut connection)
            .map(|count| count > 0)
            .map_err(|error| error.into())
    }

    fn swap(&self, connectors: &Connectors) -> Result<(), UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        connection.build_transaction().read_write().run(|conn| {
            sql_query("ALTER TABLE siren_doublons RENAME TO siren_doublons_temp").execute(conn)?;
            sql_query("ALTER TABLE siren_doublons_staging RENAME TO siren_doublons")
                .execute(conn)?;
            sql_query("ALTER TABLE siren_doublons_temp RENAME TO siren_doublons_staging")
                .execute(conn)?;
            sql_query("TRUNCATE siren_doublons_staging").execute(conn)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET last_imported_timestamp = staging_imported_timestamp
                WHERE group_type = 'siren_doublons'
                "#,
            )
            .execute(conn)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET staging_imported_timestamp = NULL
                WHERE group_type = 'siren_doublons'
                "#,
            )
            .execute(conn)?;

            Ok(())
        })
    }

    async fn get_total_count(
        &self,
        _connectors: &mut Connectors,
        _start_timestamp: NaiveDateTime,
    ) -> Result<u32, UpdatableError> {
        Ok(0)
    }

    fn get_last_insee_synced_timestamp(
        &self,
        _connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        Ok(None)
    }

    async fn update_daily_data(
        &self,
        _connectors: &mut Connectors,
        _start_timestamp: NaiveDateTime,
        _cursor: String,
    ) -> Result<(Option<String>, usize), UpdatableError> {
        Ok((None, 0))
    }
}
