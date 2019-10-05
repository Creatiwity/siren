mod columns;
pub mod common;
mod error;

use super::schema::etablissement::dsl;
use crate::connectors::Connectors;
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

pub fn insert_in_staging(connectors: &Connectors, file_path: String) -> Result<bool, Error> {
    let connection = connectors.local.pool.get()?;
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

pub fn swap(connectors: &Connectors) -> Result<(), Error> {
    let connection = connectors.local.pool.get()?;
    connection.build_transaction().read_write().run(|| {
        sql_query("ALTER TABLE etablissement RENAME TO etablissement_temp").execute(&connection)?;
        sql_query("ALTER TABLE etablissement_staging RENAME TO etablissement").execute(&connection)?;
        sql_query("ALTER TABLE etablissement_temp RENAME TO etablissement_staging").execute(&connection)?;
        sql_query("TRUNCATE etablissement_staging").execute(&connection)?;
        sql_query(r#"
            UPDATE group_metadata
            SET last_imported_timestamp = staging_imported_timestamp, last_file_timestamp = staging_file_timestamp
            WHERE group_type = 'etablissements'
        "#).execute(&connection)?;
        sql_query(r#"
            UPDATE group_metadata
            SET staging_imported_timestamp = NULL
            WHERE group_type = 'etablissements'
        "#).execute(&connection)?;

        Ok(())
    })
}
