mod columns;
pub mod common;
pub mod error;

use super::schema::unite_legale::dsl;
use crate::connectors::Connectors;
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

pub fn count(connectors: &Connectors) -> Result<i64, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::unite_legale
        .select(diesel::dsl::count(dsl::siren))
        .first::<i64>(&connection)
        .map_err(|error| error.into())
}

pub fn count_staging(connectors: &Connectors) -> Result<i64, Error> {
    use super::schema::unite_legale_staging::dsl;

    let connection = connectors.local.pool.get()?;
    dsl::unite_legale_staging
        .select(diesel::dsl::count(dsl::siren))
        .first::<i64>(&connection)
        .map_err(|error| error.into())
}

pub fn insert_in_staging(connectors: &Connectors, file_path: String) -> Result<bool, Error> {
    let connection = connectors.local.pool.get()?;
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

pub fn swap(connectors: &Connectors) -> Result<(), Error> {
    let connection = connectors.local.pool.get()?;
    connection.build_transaction().read_write().run(|| {
        sql_query("ALTER TABLE unite_legale RENAME TO unite_legale_temp").execute(&connection)?;
        sql_query("ALTER TABLE unite_legale_staging RENAME TO unite_legale").execute(&connection)?;
        sql_query("ALTER TABLE unite_legale_temp RENAME TO unite_legale_staging").execute(&connection)?;
        sql_query("TRUNCATE unite_legale_staging").execute(&connection)?;
        sql_query(r#"
            UPDATE group_metadata
            SET last_imported_timestamp = staging_imported_timestamp, last_file_timestamp = staging_file_timestamp
            WHERE group_type = 'unites_legales'
        "#).execute(&connection)?;
        sql_query(r#"
            UPDATE group_metadata
            SET staging_imported_timestamp = NULL
            WHERE group_type = 'unites_legales'
        "#).execute(&connection)?;

        Ok(())
    })
}
