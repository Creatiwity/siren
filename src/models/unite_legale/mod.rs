mod columns;
pub mod common;
pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel};
use super::schema::unite_legale::dsl;
use crate::connectors::Connectors;
use chrono::{DateTime, Utc};
use common::UniteLegale;
use diesel::prelude::*;
use diesel::sql_query;
use error::Error;

pub fn get(connectors: &Connectors, siren: &String) -> Result<UniteLegale, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::unite_legale
        .find(siren)
        .first::<UniteLegale>(&connection)
        .map_err(|error| error.into())
}

pub struct UniteLegaleModel {}

impl UpdatableModel for UniteLegaleModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        dsl::unite_legale
            .select(diesel::dsl::count(dsl::siren))
            .first::<i64>(&connection)
            .map_err(|error| error.into())
    }

    fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::unite_legale_staging::dsl;

        let connection = connectors.local.pool.get()?;
        dsl::unite_legale_staging
            .select(diesel::dsl::count(dsl::siren))
            .first::<i64>(&connection)
            .map_err(|error| error.into())
    }

    fn insert_in_staging(
        &self,
        connectors: &Connectors,
        file_path: String,
    ) -> Result<bool, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        sql_query("TRUNCATE unite_legale_staging").execute(&connection)?;
        let query = format!(
            "COPY unite_legale_staging({}) FROM '{}' DELIMITER ',' CSV HEADER",
            columns::COLUMNS,
            file_path
        );
        sql_query(query)
            .execute(&connection)
            .map(|count| count > 0)
            .map_err(|error| error.into())
    }

    fn swap(&self, connectors: &Connectors) -> Result<(), UpdatableError> {
        let connection = connectors.local.pool.get()?;
        connection.build_transaction().read_write().run(|| {
            sql_query("ALTER TABLE unite_legale RENAME TO unite_legale_temp")
                .execute(&connection)?;
            sql_query("ALTER TABLE unite_legale_staging RENAME TO unite_legale")
                .execute(&connection)?;
            sql_query("ALTER TABLE unite_legale_temp RENAME TO unite_legale_staging")
                .execute(&connection)?;
            sql_query("TRUNCATE unite_legale_staging").execute(&connection)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET last_imported_timestamp = staging_imported_timestamp
                WHERE group_type = 'unites_legales'
                "#,
            )
            .execute(&connection)?;
            sql_query(
                r#"
                UPDATE group_metadata
                SET staging_imported_timestamp = NULL
                WHERE group_type = 'unites_legales'
                "#,
            )
            .execute(&connection)?;

            Ok(())
        })
    }

    // SELECT date_dernier_traitement FROM unite_legale WHERE date_dernier_traitement IS NOT NULL ORDER BY date_dernier_traitement DESC LIMIT 1;
    fn get_last_insee_synced_timestamp(
        &self,
        _connectors: &Connectors,
    ) -> Result<Option<DateTime<Utc>>, UpdatableError> {
        Ok(None)
    }

    fn update_daily_data(
        &self,
        connectors: &Connectors,
        start_timestamp: DateTime<Utc>,
    ) -> Result<(), UpdatableError> {
        let insee = connectors
            .insee
            .as_ref()
            .ok_or(UpdatableError::MissingInseeConnector)?;

        let result = insee.get_daily_unites_legales(start_timestamp)?;

        println!("[SyncInsee] Done updating {} UnitesLegales", result.len());
        Ok(())
    }
}
