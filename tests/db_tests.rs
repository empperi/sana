use sana::config::Config;
use sana::db::{self, users, messages};
use std::env;
use sqlx::{Connection, PgConnection, PgPool};
use sana::messages::ChatMessage;
use chrono::Utc;
use uuid::Uuid;

struct TestContext {
    pub pool: PgPool,
}

impl TestContext {
    async fn new(db_name: &str) -> Self {
        let db_name = db_name.to_string();
        
        // 1. Create the test database if it doesn't exist
        {
            let base_config = Config::load(None);
            let base_url = base_config.database_url;
            let admin_url = base_url.rsplit_once('/').map(|(base, _)| format!("{}/postgres", base)).unwrap_or(base_url);
            
            let mut conn = PgConnection::connect(&admin_url).await.expect("Failed to connect to postgres admin db");
            
            // Drop and recreate to have a clean state
            sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name)).execute(&mut conn).await.ok();
            sqlx::query(&format!("CREATE DATABASE {}", db_name)).execute(&mut conn).await.expect("Failed to create test db");
        }

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
        env::remove_var("POSTGRES_DB");
    }
}

#[tokio::test]
async fn test_db_migrations_and_connection() {
    let ctx = TestContext::new("sana_test_db_migration").await;
    
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

#[tokio::test]
async fn test_user_crud() {
    let ctx = TestContext::new("sana_test_db_user_crud").await;
    let pool = &ctx.pool;

    let username = "testuser";
    let password_hash = "hashed_password";

    let mut tx = pool.begin().await.expect("Failed to start transaction");

    // 1. Create
    let user = users::create_user(&mut tx, username, password_hash).await.expect("Failed to create user");
    assert_eq!(user.username, username);
    assert_eq!(user.password, password_hash);
    assert!(user.last_login.is_none());

    // 2. Read by ID
    let fetched_user = users::get_user_by_id(&mut tx, user.user_id).await.expect("Failed to get user by id");
    assert!(fetched_user.is_some());
    let fetched_user = fetched_user.unwrap();
    assert_eq!(fetched_user.username, username);

    // 3. Read by Username
    let fetched_user_by_name = users::get_user_by_username(&mut tx, username).await.expect("Failed to get user by username");
    assert!(fetched_user_by_name.is_some());
    assert_eq!(fetched_user_by_name.unwrap().user_id, user.user_id);

    // 4. Update last login
    users::update_last_login(&mut tx, user.user_id).await.expect("Failed to update last login");
    let updated_user = users::get_user_by_id(&mut tx, user.user_id).await.expect("Failed to get user").unwrap();
    assert!(updated_user.last_login.is_some());

    // 5. Delete
    users::delete_user(&mut tx, user.user_id).await.expect("Failed to delete user");
    let deleted_user = users::get_user_by_id(&mut tx, user.user_id).await.expect("Failed to get user");
    assert!(deleted_user.is_none());
    
    tx.commit().await.expect("Failed to commit transaction");
}

#[tokio::test]
async fn test_message_insertion() {
    let ctx = TestContext::new("sana_test_db_messages").await;
    let pool = &ctx.pool;

    let mut tx = pool.begin().await.expect("Failed to start transaction");

    let msg = ChatMessage {
        id: Uuid::new_v4().to_string(),
        user: "testuser".to_string(),
        timestamp: Utc::now().timestamp_millis(),
        message: "Hello world".to_string(),
        seq: Some(10),
    };

    // Insert
    messages::insert_message(&mut tx, "General", 10, &msg).await.expect("Failed to insert message");

    // Insert same ID should do nothing (idempotency)
    messages::insert_message(&mut tx, "General", 10, &msg).await.expect("Failed to insert same message again");

    // Check if inserted
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM messages")
        .fetch_one(&mut *tx)
        .await
        .expect("Failed to count messages");
    
    assert_eq!(count.0, 1);

    tx.commit().await.expect("Failed to commit transaction");
}
