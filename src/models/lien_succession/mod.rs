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
use error::Error;

pub fn get(connection: &mut Connection, siret: &str) -> Result<Vec<LienSuccession>, Error> {
    dsl::lien_succession
        .select(LienSuccession::as_select())
        .filter(
            dsl::siret_etablissement_predecesseur
                .eq(siret)
                .or(dsl::siret_etablissement_successeur.eq(siret)),
        )
        .load::<LienSuccession>(connection)
        .map_err(|error| error.into())
}

pub struct LienSuccessionModel {}

#[async_trait]
impl UpdatableModel for LienSuccessionModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        dsl::lien_succession
            .select(diesel::dsl::count(dsl::id))
            .first::<i64>(&mut connection)
            .map_err(|error| error.into())
    }

    fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::lien_succession_staging::dsl;

        let mut connection = connectors.local.pool.get()?;
        dsl::lien_succession_staging
            .select(diesel::dsl::count(dsl::id))
            .first::<i64>(&mut connection)
            .map_err(|error| error.into())
    }

    fn insert_remote_file_in_staging(
        &self,
        connectors: &Connectors,
        remote_file: RemoteFile,
    ) -> Result<bool, UpdatableError> {
        use super::schema::lien_succession_staging::dsl;

        let mut connection = connectors.local.pool.get()?;

        sql_query("TRUNCATE lien_succession_staging").execute(&mut connection)?;

        diesel::copy_from(dsl::lien_succession_staging)
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
            .with_header(CopyHeader::Set(true))
            .execute(&mut connection)
            .map(|count| count > 0)
            .map_err(|error| error.into())
    }

    fn swap(&self, connectors: &Connectors) -> Result<(), UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        connection.build_transaction().read_write().run(|conn| {
            sql_query("ALTER TABLE lien_succession RENAME TO lien_succession_temp")
                .execute(conn)?;
            sql_query("ALTER TABLE lien_succession_staging RENAME TO lien_succession")
                .execute(conn)?;
            sql_query("ALTER TABLE lien_succession_temp RENAME TO lien_succession_staging")
                .execute(conn)?;
            sql_query("TRUNCATE lien_succession_staging").execute(conn)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET last_imported_timestamp = staging_imported_timestamp
                WHERE group_type = 'liens_succession'
                "#,
            )
            .execute(conn)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET staging_imported_timestamp = NULL
                WHERE group_type = 'liens_succession'
                "#,
            )
            .execute(conn)?;

            Ok(())
        })
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

    // SELECT date_dernier_traitement_lien_succession FROM lien_succession WHERE date_dernier_traitement_lien_succession IS NOT NULL ORDER BY date_dernier_traitement_lien_succession DESC LIMIT 1;
    fn get_last_insee_synced_timestamp(
        &self,
        connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        let mut connection = connectors.local.pool.get()?;
        dsl::lien_succession
            .select(dsl::date_dernier_traitement_lien_succession)
            .order(dsl::date_dernier_traitement_lien_succession.desc())
            .filter(dsl::date_dernier_traitement_lien_succession.is_not_null())
            .first::<Option<NaiveDateTime>>(&mut connection)
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

        let mut connection = connectors.local.pool.get()?;

        let updated_count = diesel::insert_into(dsl::lien_succession)
            .values(&liens_succession)
            .execute(&mut connection)?;

        Ok((next_cursor, updated_count))
    }
}
