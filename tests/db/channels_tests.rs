use sana::db::channels;
use uuid::Uuid;
use chrono::Utc;
use crate::db::common::TestContext;

#[tokio::test]
async fn test_channel_insertion() {
    let ctx = TestContext::new("sana_test_chan_insertion").await;
    let pool = &ctx.pool;

    let channel = channels::Channel {
        id: Uuid::new_v4(),
        name: "test-channel".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };

    let mut tx = pool.begin().await.expect("Failed to start transaction");
    channels::insert_channel(&mut tx, &channel).await.expect("Failed to insert channel");
    tx.commit().await.expect("Failed to commit transaction");

    let mut tx_fetch = pool.begin().await.unwrap();
    let fetched = channels::get_channel_by_id(&mut tx_fetch, channel.id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "test-channel");
}

#[tokio::test]
async fn test_channel_idempotency() {
    let ctx = TestContext::new("sana_test_chan_idempotency").await;
    let pool = &ctx.pool;

    let channel = channels::Channel {
        id: Uuid::new_v4(),
        name: "idemp-channel".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };

    let mut tx1 = pool.begin().await.expect("Failed start 1");
    channels::insert_channel(&mut tx1, &channel).await.expect("Insert 1");
    tx1.commit().await.expect("Commit 1");

    let mut tx2 = pool.begin().await.expect("Failed start 2");
    channels::insert_channel(&mut tx2, &channel).await.expect("Insert 2 (idemp)");
    tx2.commit().await.expect("Commit 2");

    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM channels")
        .fetch_one(pool)
        .await
        .expect("Failed count");
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn test_get_channel_by_name() {
    let ctx = TestContext::new("sana_test_chan_by_name").await;
    let pool = &ctx.pool;

    let name = "find-me";
    let channel = channels::Channel {
        id: Uuid::new_v4(),
        name: name.to_string(),
        is_private: true,
        created_at: Utc::now(),
    };

    let mut tx = pool.begin().await.unwrap();
    channels::insert_channel(&mut tx, &channel).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx_fetch = pool.begin().await.unwrap();
    let fetched = channels::get_channel_by_name(&mut tx_fetch, name).await.unwrap();
    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, channel.id);
    assert!(fetched.is_private);
}
