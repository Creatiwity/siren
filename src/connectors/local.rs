use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

pub type Connection = r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;

#[derive(Clone)]
pub struct Connector {
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

#[derive(Clone)]
pub struct ConnectorBuilder {
    pool: Pool<ConnectionManager<PgConnection>>,
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
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(15);
        let manager = ConnectionManager::<PgConnection>::new(database_url.clone());

        let builder = ConnectorBuilder {
            pool: Pool::builder()
                .max_size(pool_size)
                .build(manager)
                .unwrap_or_else(|error| panic!("Error connecting to {database_url} ({error})")),
        };

        let mut connection = builder
            .pool
            .get()
            .expect("Unable to connect for migrations");

        connection
            .run_pending_migrations(MIGRATIONS)
            .expect("Unable to run migrations");

        builder
    }

    pub fn create(&self) -> Connector {
        Connector {
            pool: self.pool.clone(),
        }
    }
}
