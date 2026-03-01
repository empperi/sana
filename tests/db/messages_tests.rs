use sana::db;
use sana::messages::ChatMessage;
use chrono::Utc;
use uuid::Uuid;
use crate::db::common::{TestContext, create_test_user, create_test_channel};

#[tokio::test]
async fn test_db_migrations_and_connection() {
    let ctx = TestContext::new("sana_test_db_migration").await;
    
    let is_connected = db::check_connection(&ctx.pool).await.expect("Failed to check connection");
    assert!(is_connected);

    let table_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'messages')"
    )
    .fetch_one(&ctx.pool)
    .await
    .expect("Failed to query information_schema");
    
    assert!(table_exists.0, "The 'messages' table should exist after migrations");
}

#[tokio::test]
async fn test_message_insertion() {
    let ctx = TestContext::new("sana_test_msg_insertion").await;
    let pool = &ctx.pool;

    let user = create_test_user(pool, "msg_user").await;
    let channel = create_test_channel(pool, "msg_channel").await;

    let msg = ChatMessage {
        id: Uuid::new_v4(),
        channel_id: channel.id,
        user_id: user.id,
        user: user.username,
        timestamp: Utc::now(),
        message: "Hello world".to_string(),
        seq: Some(10),
    };

    let mut tx = pool.begin().await.expect("Failed to start transaction");
    db::messages::insert_message(&mut tx, 10, &msg).await.expect("Failed to insert message");
    tx.commit().await.expect("Failed to commit transaction");

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM messages")
        .fetch_one(pool)
        .await
        .expect("Failed to count messages");
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn test_message_insertion_idempotency() {
    let ctx = TestContext::new("sana_test_msg_idempotency").await;
    let pool = &ctx.pool;

    let user = create_test_user(pool, "idemp_user").await;
    let channel = create_test_channel(pool, "idemp_channel").await;

    let msg = ChatMessage {
        id: Uuid::new_v4(),
        channel_id: channel.id,
        user_id: user.id,
        user: user.username,
        timestamp: Utc::now(),
        message: "Hello world".to_string(),
        seq: Some(20),
    };

    // Insert twice
    let mut tx1 = pool.begin().await.expect("Failed to start tx1");
    db::messages::insert_message(&mut tx1, 20, &msg).await.expect("Failed insert 1");
    tx1.commit().await.expect("Failed commit 1");

    let mut tx2 = pool.begin().await.expect("Failed to start tx2");
    db::messages::insert_message(&mut tx2, 20, &msg).await.expect("Failed insert 2 (idempotency)");
    tx2.commit().await.expect("Failed commit 2");

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM messages")
        .fetch_one(pool)
        .await
        .expect("Failed to count messages");
    assert_eq!(count.0, 1, "Duplicate insertion should be ignored due to idempotency");
}
