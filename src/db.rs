use sqlx::postgres::{PgPool, PgPoolOptions};
use crate::config::Config;

pub async fn connect(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
}

pub async fn check_connection(pool: &PgPool) -> Result<bool, sqlx::Error> {
    let result: (bool,) = sqlx::query_as("SELECT 1=1")
        .fetch_one(pool)
        .await?;
    Ok(result.0)
}
