//! Integration tests backed by a real database.
//!
//! They cover the `search-etablissements` and `search-unites-legales`
//! specifications. Each one goes dormant, without failing, when
//! `SIRENE_TEST_DATABASE_URL` is unset:
//!
//!   SIRENE_TEST_DATABASE_URL="postgresql://…" cargo test

mod search;
mod staging;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;

use crate::connectors::local::Connection;

/// Deliberately distinct from `DATABASE_URL`: running the suite must not be
/// able to touch the development database by accident.
pub async fn test_connection() -> Option<Connection> {
    let url = std::env::var("SIRENE_TEST_DATABASE_URL").ok()?;

    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
    let pool = Pool::builder(manager)
        .max_size(1)
        .build()
        .expect("pool de test inconstructible");

    Some(
        pool.get()
            .await
            .expect("SIRENE_TEST_DATABASE_URL est defini mais inutilisable"),
    )
}

/// Skips the test out loud, rather than passing it silently.
macro_rules! require_database {
    () => {
        match crate::tests::test_connection().await {
            Some(connection) => connection,
            None => {
                eprintln!("ignore : SIRENE_TEST_DATABASE_URL non defini");
                return;
            }
        }
    };
}

pub(crate) use require_database;
