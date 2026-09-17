//! Index-free staging loads.
//!
//! This path *drops indexes* before loading: a failure at the wrong moment would
//! leave a staging table without indexes, which the swap would promote to
//! production. Hence the explicit rollback check.

use diesel::Connection as _;
use diesel::RunQueryDsl as _;
use diesel::pg::PgConnection;
use diesel::sql_types::{BigInt, Text};

use crate::models::common::{Error as UpdatableError, load_staging_without_indexes};

#[derive(diesel::QueryableByName)]
struct Name {
    #[diesel(sql_type = Text)]
    indexname: String,
}

#[derive(diesel::QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

/// Throwaway table shaped like a staging table: a primary key, whose index must
/// survive, and two plain indexes, which must be dropped then rebuilt.
fn scratch_table(connection: &mut PgConnection, name: &str) {
    for statement in [
        format!("DROP TABLE IF EXISTS {name}"),
        format!("CREATE TABLE {name} (id text PRIMARY KEY, label text, rang int)"),
        format!("CREATE INDEX {name}_label_idx ON {name} (label)"),
        format!("CREATE INDEX {name}_rang_idx ON {name} (rang DESC NULLS LAST)"),
    ] {
        diesel::sql_query(statement).execute(connection).unwrap();
    }
}

fn index_names(connection: &mut PgConnection, table: &str) -> Vec<String> {
    diesel::sql_query(
        "SELECT indexname FROM pg_indexes WHERE schemaname='public' AND tablename=$1 \
         ORDER BY indexname",
    )
    .bind::<Text, _>(table.to_string())
    .load::<Name>(connection)
    .unwrap()
    .into_iter()
    .map(|row| row.indexname)
    .collect()
}

fn row_count(connection: &mut PgConnection, table: &str) -> i64 {
    diesel::sql_query(format!("SELECT count(*) AS count FROM {table}"))
        .get_result::<Count>(connection)
        .unwrap()
        .count
}

fn sync_connection() -> Option<PgConnection> {
    let url = std::env::var("SIRENE_TEST_DATABASE_URL").ok()?;
    Some(PgConnection::establish(&url).expect("SIRENE_TEST_DATABASE_URL inutilisable"))
}

#[test]
fn index_retires_puis_reconstruits() {
    let Some(mut connection) = sync_connection() else {
        eprintln!("ignore : SIRENE_TEST_DATABASE_URL non defini");
        return;
    };

    let table = "_test_staging_ok";
    scratch_table(&mut connection, table);
    let before = index_names(&mut connection, table);
    assert_eq!(before.len(), 3, "une cle primaire et deux index ordinaires");

    let inserted = load_staging_without_indexes(&mut connection, table, |connection| {
        diesel::sql_query(format!(
            "INSERT INTO {table} (id, label, rang) VALUES ('a', 'alpha', 1), ('b', 'beta', 2)"
        ))
        .execute(connection)
        .map(|count| count > 0)
        .map_err(|source| UpdatableError::Database { source })
    })
    .unwrap();

    assert!(inserted);
    assert_eq!(row_count(&mut connection, table), 2);
    assert_eq!(
        index_names(&mut connection, table),
        before,
        "tous les index doivent etre revenus, a l'identique"
    );

    diesel::sql_query(format!("DROP TABLE {table}"))
        .execute(&mut connection)
        .unwrap();
}

#[test]
fn echec_du_chargement_restaure_les_index() {
    let Some(mut connection) = sync_connection() else {
        eprintln!("ignore : SIRENE_TEST_DATABASE_URL non defini");
        return;
    };

    let table = "_test_staging_ko";
    scratch_table(&mut connection, table);
    let before = index_names(&mut connection, table);

    let result = load_staging_without_indexes(&mut connection, table, |connection| {
        diesel::sql_query(format!(
            "INSERT INTO {table} (id, label, rang) VALUES ('a', 'alpha', 1)"
        ))
        .execute(connection)
        .map_err(|source| UpdatableError::Database { source })?;

        // Primary key violation: the load fails midway.
        diesel::sql_query(format!(
            "INSERT INTO {table} (id, label, rang) VALUES ('a', 'doublon', 2)"
        ))
        .execute(connection)
        .map(|count| count > 0)
        .map_err(|source| UpdatableError::Database { source })
    });

    assert!(result.is_err(), "le chargement devait echouer");
    assert_eq!(
        index_names(&mut connection, table),
        before,
        "un echec ne doit pas laisser la table sans index"
    );
    assert_eq!(
        row_count(&mut connection, table),
        0,
        "la transaction doit avoir tout annule"
    );

    diesel::sql_query(format!("DROP TABLE {table}"))
        .execute(&mut connection)
        .unwrap();
}
