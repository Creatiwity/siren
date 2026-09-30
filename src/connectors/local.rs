use crate::diesel_instrumentation::AsyncDieselInstrumentation;
use diesel::Connection as DieselConnection;
use diesel::QueryableByName;
use diesel::RunQueryDsl as _;
use diesel::migration::MigrationSource;
use diesel::pg::{Pg, PgConnection};
use diesel::sql_types::{Bool, Text};
use diesel_async::AsyncConnection;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, ManagerConfig};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::collections::HashSet;
use std::env;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

pub type Connection = diesel_async::pooled_connection::deadpool::Object<AsyncPgConnection>;

#[derive(Clone)]
pub struct Connector {
    pub pool: Pool<AsyncPgConnection>,
    pub database_url: String,
}

#[derive(Clone)]
pub struct ConnectorBuilder {
    pool: Pool<AsyncPgConnection>,
    database_url: String,
}

/// What to do with the embedded migrations when a connector is built.
#[derive(Clone, Copy, Debug)]
pub enum MigrationMode {
    /// Apply pending migrations (needs a writable database).
    Run,
    /// Only verify none is pending, with read-only queries: the schema is
    /// migrated elsewhere (`migrate` command, deployment hook) and
    /// `DATABASE_URL` may point to a read replica.
    Check,
}

pub fn database_url() -> String {
    env::var("DATABASE_URL")
        .ok()
        .or_else(|| {
            if let (Some(host), Some(port), Some(database), Some(user), Some(password)) = (
                env::var("DATABASE_HOST").ok(),
                env::var("DATABASE_PORT").ok(),
                env::var("DATABASE_NAME").ok(),
                env::var("DATABASE_USER").ok(),
                env::var("DATABASE_PASSWORD").ok(),
            ) {
                Some(format!(
                    "postgresql://{user}:{password}@{host}:{port}/{database}"
                ))
            } else {
                None
            }
        })
        .expect("DATABASE_URL must be set")
}

/// Applies the pending migrations and returns their versions.
pub fn run_migrations(database_url: &str) -> Result<Vec<String>, String> {
    let mut conn = PgConnection::establish(database_url)
        .map_err(|error| format!("Error connecting for migrations: {error}"))?;
    conn.run_pending_migrations(MIGRATIONS)
        .map(|versions| versions.iter().map(ToString::to_string).collect())
        .map_err(|error| format!("Unable to run migrations: {error}"))
}

#[derive(QueryableByName)]
struct AppliedMigration {
    #[diesel(sql_type = Text)]
    version: String,
}

#[derive(QueryableByName)]
struct TableExists {
    #[diesel(sql_type = Bool)]
    exists: bool,
}

/// Returns the versions of the embedded migrations not applied yet.
///
/// Reads `__diesel_schema_migrations` directly instead of going through
/// `MigrationHarness`, which issues a `CREATE TABLE IF NOT EXISTS` first and
/// would therefore fail on a read replica.
pub fn pending_migrations(database_url: &str) -> Result<Vec<String>, String> {
    let mut conn = PgConnection::establish(database_url)
        .map_err(|error| format!("Error connecting for migration check: {error}"))?;

    // A database never migrated has no table yet: everything is pending.
    let has_table =
        diesel::sql_query("SELECT to_regclass('__diesel_schema_migrations') IS NOT NULL AS exists")
            .get_result::<TableExists>(&mut conn)
            .map_err(|error| format!("Unable to read applied migrations: {error}"))?
            .exists;

    let applied: HashSet<String> = if has_table {
        diesel::sql_query("SELECT version FROM __diesel_schema_migrations")
            .load::<AppliedMigration>(&mut conn)
            .map_err(|error| format!("Unable to read applied migrations: {error}"))?
            .into_iter()
            .map(|migration| migration.version)
            .collect()
    } else {
        HashSet::new()
    };

    let embedded = MigrationSource::<Pg>::migrations(&MIGRATIONS)
        .map_err(|error| format!("Unable to list embedded migrations: {error}"))?;

    Ok(embedded
        .iter()
        .map(|migration| migration.name().version().to_string())
        .filter(|version| !applied.contains(version))
        .collect())
}

impl ConnectorBuilder {
    pub fn new(migration_mode: MigrationMode) -> ConnectorBuilder {
        let database_url = database_url();
        let pool_size = env::var("DATABASE_POOL_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(15);

        // Synchronous, on a dedicated connection, before the pool exists
        tokio::task::block_in_place(|| match migration_mode {
            MigrationMode::Run => {
                run_migrations(&database_url).unwrap_or_else(|error| panic!("{error}"));
            }
            MigrationMode::Check => {
                let pending =
                    pending_migrations(&database_url).unwrap_or_else(|error| panic!("{error}"));
                // The code expects the latest schema: refuse to start rather
                // than serve errors. The `migrate` step did not run.
                if !pending.is_empty() {
                    panic!(
                        "Automatic migrations are disabled but {} migration(s) are pending ({}). Run the `migrate` command first.",
                        pending.len(),
                        pending.join(", ")
                    );
                }
            }
        });

        let mut manager_config = ManagerConfig::<AsyncPgConnection>::default();
        manager_config.custom_setup = Box::new(|url| {
            let url = url.to_owned();
            Box::pin(async move {
                let mut conn = AsyncPgConnection::establish(&url).await?;
                conn.set_instrumentation(AsyncDieselInstrumentation::default());
                Ok(conn)
            })
        });
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(
            database_url.clone(),
            manager_config,
        );

        let pool = Pool::builder(manager)
            .max_size(pool_size)
            .build()
            .unwrap_or_else(|error| panic!("Error creating pool for {database_url} ({error})"));

        ConnectorBuilder { pool, database_url }
    }

    pub fn create(&self) -> Connector {
        Connector {
            pool: self.pool.clone(),
            database_url: self.database_url.clone(),
        }
    }
}
