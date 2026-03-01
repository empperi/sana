use sana::config::Config;
use sana::db::{self, users, channels};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;
use chrono::Utc;

pub struct TestContext {
    pub pool: PgPool,
}

impl TestContext {
    pub async fn new(db_name: &str) -> Self {
        let db_name = db_name.to_string();
        
        let base_config = Config::load(None);
        let base_url = base_config.database_url;
        let admin_url = base_url.rsplit_once('/').map(|(base, _)| format!("{}/postgres", base)).unwrap_or(base_url.clone());
        let test_db_url = base_url.rsplit_once('/').map(|(base, _)| format!("{}/{}", base, db_name)).unwrap_or(format!("{}_{}", base_url, db_name));

        // 1. Create the test database
        {
            let mut conn = PgConnection::connect(&admin_url).await.expect("Failed to connect to postgres admin db");
            sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name)).execute(&mut conn).await.ok();
            sqlx::query(&format!("CREATE DATABASE {}", db_name)).execute(&mut conn).await.expect("Failed to create test db");
        }

        let pool = PgPool::connect(&test_db_url).await.expect("Failed to connect to the test database");
        
        // 3. Run migrations
        db::run_migrations(&pool).await.expect("Failed to run migrations");
        
        TestContext { pool }
    }
}

#[allow(dead_code)]
pub async fn create_test_user(pool: &PgPool, username: &str) -> users::User {
    let mut tx = pool.begin().await.expect("Failed to start transaction");
    let user = users::create_user(&mut tx, username, "testpass").await.expect("Failed to create test user");
    tx.commit().await.expect("Failed to commit test user transaction");
    user
}

#[allow(dead_code)]
pub async fn create_test_channel(pool: &PgPool, name: &str) -> channels::Channel {
    let mut tx = pool.begin().await.expect("Failed to start transaction");
    let channel = channels::Channel {
        id: Uuid::new_v4(),
        name: name.to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    channels::insert_channel(&mut tx, &channel).await.expect("Failed to insert test channel");
    tx.commit().await.expect("Failed to commit test channel transaction");
    channel
}
