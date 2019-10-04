mod columns;
pub mod common;
mod error;

use super::schema::group_metadata::dsl;
use crate::connectors::Connectors;
use common::UniteLegale;
use diesel::prelude::*;
use diesel::sql_query;
use error::Error;

/*pub fn get(connectors: &Connectors, siret: String) -> Result<UniteLegale, Error> {
    let connection = connectors.local.pool.get()?;
    dsl::unite_legale
        .find(siret)
        .first::<UniteLegale>(&connection)
        .map_err(|error| error.into())
}*/

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

pub fn swap(connectors: &Connectors) -> Result<bool, Error> {
    let connection = connectors.local.pool.get()?;
    sql_query(
        r#"
        BEGIN;
        ALTER TABLE unite_legale RENAME TO unite_legale_temp;
        ALTER TABLE unite_legale_staging RENAME TO unite_legale;
        ALTER TABLE unite_legale_temp RENAME TO unite_legale_staging;
        TRUNCATE unite_legale_staging;
        UPDATE group_metadata
        SET last_imported_timestamp = staging_imported_timestamp, last_file_timestamp = staging_file_timestamp
        WHERE group_type = 'unites_legales';
        UPDATE group_metadata
        SET staging_imported_timestamp = NULL
        WHERE group_type = 'unites_legales';
        COMMIT;
        "#,
    )
    .execute(&connection)
    .map(|count| count > 0)
    .map_err(|error| error.into())
}
