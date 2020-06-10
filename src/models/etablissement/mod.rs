mod columns;
pub mod common;
pub mod error;

use super::common::{Error as UpdatableError, UpdatableModel};
use super::schema::etablissement::dsl;
use crate::connectors::Connectors;
use chrono::NaiveDateTime;
use common::Etablissement;
use diesel::prelude::*;
use diesel::sql_query;
use error::Error;

pub fn get(connectors: &Connectors, siret: &String) -> Result<Etablissement, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::etablissement
        .find(siret)
        .first::<Etablissement>(&connection)
        .map_err(|error| error.into())
}

pub fn get_with_siren(
    connectors: &Connectors,
    siren: &String,
) -> Result<Vec<Etablissement>, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::etablissement
        .filter(dsl::siren.eq(siren))
        .load::<Etablissement>(&connection)
        .map_err(|error| error.into())
}

pub fn get_siege_with_siren(
    connectors: &Connectors,
    siren: &String,
) -> Result<Etablissement, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::etablissement
        .filter(dsl::siren.eq(siren).and(dsl::etablissement_siege.eq(true)))
        .first::<Etablissement>(&connection)
        .map_err(|error| error.into())
}

pub struct EtablissementModel {}

impl UpdatableModel for EtablissementModel {
    fn count(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        dsl::etablissement
            .select(diesel::dsl::count(dsl::siret))
            .first::<i64>(&connection)
            .map_err(|error| error.into())
    }

    fn count_staging(&self, connectors: &Connectors) -> Result<i64, UpdatableError> {
        use super::schema::etablissement_staging::dsl;

        let connection = connectors.local.pool.get()?;
        dsl::etablissement_staging
            .select(diesel::dsl::count(dsl::siret))
            .first::<i64>(&connection)
            .map_err(|error| error.into())
    }

    fn insert_in_staging(
        &self,
        connectors: &Connectors,
        file_path: String,
    ) -> Result<bool, UpdatableError> {
        let connection = connectors.local.pool.get()?;
        sql_query("TRUNCATE etablissement_staging").execute(&connection)?;
        let query = format!(
            "COPY etablissement_staging({}) FROM '{}' DELIMITER ',' CSV HEADER",
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
            sql_query("ALTER TABLE etablissement RENAME TO etablissement_temp")
                .execute(&connection)?;
            sql_query("ALTER TABLE etablissement_staging RENAME TO etablissement")
                .execute(&connection)?;
            sql_query("ALTER TABLE etablissement_temp RENAME TO etablissement_staging")
                .execute(&connection)?;
            sql_query("TRUNCATE etablissement_staging").execute(&connection)?;
            sql_query(
                r#"
            UPDATE group_metadata
            SET last_imported_timestamp = staging_imported_timestamp
            WHERE group_type = 'etablissements'
            "#,
            )
            .execute(&connection)?;
            sql_query(
                r#"
            UPDATE group_metadata
            SET staging_imported_timestamp = NULL
            WHERE group_type = 'etablissements'
            "#,
            )
            .execute(&connection)?;

            Ok(())
        })
    }

    // SELECT date_dernier_traitement FROM etablissement WHERE date_dernier_traitement IS NOT NULL ORDER BY date_dernier_traitement DESC LIMIT 1;
    fn get_last_insee_synced_timestamp(
        &self,
        _connectors: &Connectors,
    ) -> Result<Option<NaiveDateTime>, UpdatableError> {
        Ok(None)
    }

    fn update_daily_data(
        &self,
        connectors: &Connectors,
        _start_timestamp: NaiveDateTime,
    ) -> Result<(), UpdatableError> {
        let insee = connectors
            .insee
            .as_ref()
            .ok_or(UpdatableError::MissingInseeConnector)?;

        let result = insee.get_daily_etablissements()?;

        println!("{:#?}", result);
        Ok(())
    }
}
