use sana::config::Config;
use sana::db;
use std::env;
use sqlx::{Connection, PgConnection, PgPool};

#[tokio::test]
async fn test_db_migrations_and_connection() {
    let ctx = TestContext::new("sana_test_db").await;

    let is_connected = db::check_connection(&ctx.pool).await.expect("Failed to check connection");
    assert!(is_connected);

    let table_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'users')"
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Failed to query information_schema");

    assert!(table_exists.0, "The 'users' table should exist after migrations");
}

struct TestContext {
    pub pool: PgPool,
}

impl TestContext {
    async fn new(db_name: &str) -> Self {
        let db_name = db_name.to_string();
        
        // 1. Create the test database if it doesn't exist
        let base_config = Config::load(None);
        let base_url = base_config.database_url;
        let admin_url = base_url.rsplit_once('/').map(|(base, _)| format!("{}/postgres", base)).unwrap_or(base_url);

        let mut conn = PgConnection::connect(&admin_url).await.expect("Failed to connect to postgres admin db");

        // Drop and recreate to have a clean state
        sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name)).execute(&mut conn).await.ok();
        sqlx::query(&format!("CREATE DATABASE {}", db_name)).execute(&mut conn).await.expect("Failed to create test db");

        // 2. Set environment variable for Config::load
        env::set_var("POSTGRES_DB", &db_name);
        let config = Config::load(None);
        
        let pool = db::connect(&config).await.expect("Failed to connect to the test database");
        
        // 3. Run migrations
        db::run_migrations(&pool).await.expect("Failed to run migrations");
        
        TestContext { pool }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Note: Dropping the database in 'drop' is hard because it's async 
        // and we are in a synchronous drop. For tests, we usually rely on 
        // 'DROP DATABASE IF EXISTS' at the start of the next run.
        env::remove_var("POSTGRES_DB");
    }
}

