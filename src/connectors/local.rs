use crate::diesel_instrumentation::AsyncDieselInstrumentation;
use diesel::Connection as DieselConnection;
use diesel::pg::PgConnection;
use diesel_async::AsyncConnection;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, ManagerConfig};
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
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

impl ConnectorBuilder {
    pub fn new() -> ConnectorBuilder {
        let database_url = env::var("DATABASE_URL")
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
            .expect("DATABASE_URL must be set");
        let pool_size = env::var("DATABASE_POOL_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(15);

        // Run migrations synchronously using a separate sync connection
        tokio::task::block_in_place(|| {
            let mut conn: PgConnection = DieselConnection::establish(&database_url)
                .unwrap_or_else(|error| panic!("Error connecting for migrations: {error}"));
            conn.run_pending_migrations(MIGRATIONS)
                .expect("Unable to run migrations");
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
