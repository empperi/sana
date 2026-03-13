use sana::db;
use sana::messages::ChatMessage;
use chrono::{DateTime, Utc, Duration, TimeZone};
use uuid::Uuid;
use sqlx::PgPool;
use crate::db::common::{TestContext, create_test_user, create_test_channel};

#[tokio::test]
async fn test_get_recent_messages() {
    let ctx = TestContext::new("sana_test_msg_recent").await;
    let user = create_test_user(&ctx.pool, "user1").await;
    let channel = create_test_channel(&ctx.pool, "chan1").await;
    setup_channel_history(&ctx.pool, channel.id, user.id, &user.username, 10).await;

    let msgs = db::messages::get_messages(&ctx.pool, channel.id, 5, None, false).await.unwrap();

    assert_eq!(msgs.len(), 5);
    assert_eq!(msgs[0].message, "Message 9");
    assert_eq!(msgs[4].message, "Message 5");
}

#[tokio::test]
async fn test_get_messages_before_timestamp() {
    let ctx = TestContext::new("sana_test_msg_before").await;
    let user = create_test_user(&ctx.pool, "user1").await;
    let channel = create_test_channel(&ctx.pool, "chan1").await;
    let base_time = setup_channel_history(&ctx.pool, channel.id, user.id, &user.username, 10).await;
    
    // Get messages older than "Message 5" (base_time + 5s)
    let before_ts = base_time + Duration::seconds(5);
    let msgs = db::messages::get_messages(&ctx.pool, channel.id, 5, Some(before_ts), false).await.unwrap();

    assert_eq!(msgs.len(), 5);
    assert_eq!(msgs[0].message, "Message 4");
    assert_eq!(msgs[4].message, "Message 0");
}

#[tokio::test]
async fn test_get_messages_channel_isolation() {
    let ctx = TestContext::new("sana_test_msg_isolation").await;
    let user = create_test_user(&ctx.pool, "user1").await;
    let channel1 = create_test_channel(&ctx.pool, "chan1").await;
    let channel2 = create_test_channel(&ctx.pool, "chan2").await;
    
    insert_test_message(&ctx.pool, channel1.id, user.id, &user.username, Utc::now(), 1, "Chan 1 Msg").await;
    insert_test_message(&ctx.pool, channel2.id, user.id, &user.username, Utc::now(), 2, "Chan 2 Msg").await;

    let msgs = db::messages::get_messages(&ctx.pool, channel1.id, 10, None, false).await.unwrap();

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].message, "Chan 1 Msg");
}

#[tokio::test]
async fn test_get_messages_limit_handling() {
    let ctx = TestContext::new("sana_test_msg_limit").await;
    let user = create_test_user(&ctx.pool, "user1").await;
    let channel = create_test_channel(&ctx.pool, "chan1").await;
    setup_channel_history(&ctx.pool, channel.id, user.id, &user.username, 3).await;

    let msgs = db::messages::get_messages(&ctx.pool, channel.id, 10, None, false).await.unwrap();

    assert_eq!(msgs.len(), 3);
}

#[tokio::test]
async fn test_get_messages_order_asc() {
    let ctx = TestContext::new("sana_test_msg_order_asc").await;
    let user = create_test_user(&ctx.pool, "user1").await;
    let channel = create_test_channel(&ctx.pool, "chan1").await;
    setup_channel_history(&ctx.pool, channel.id, user.id, &user.username, 10).await;

    let msgs = db::messages::get_messages(&ctx.pool, channel.id, 5, None, true).await.unwrap();

    assert_eq!(msgs.len(), 5);
    // When order_asc is true, it gets the 5 most recent, but returns them in ASC order.
    // So the latest 5 are 5, 6, 7, 8, 9. Ordered ASC they should be 5, 6, 7, 8, 9.
    assert_eq!(msgs[0].message, "Message 5");
    assert_eq!(msgs[4].message, "Message 9");
}


async fn insert_test_message(pool: &PgPool, channel_id: Uuid, user_id: Uuid, username: &str, timestamp: DateTime<Utc>, seq: u64, content: &str) {
    let msg = ChatMessage {
        id: Uuid::new_v4(),
        channel_id,
        user_id,
        user: username.to_string(),
        timestamp,
        message: content.to_string(),
        seq: Some(seq),
    };
    let mut tx = pool.begin().await.unwrap();
    db::messages::insert_message(&mut tx, seq, &msg).await.unwrap();
    tx.commit().await.unwrap();
}

async fn setup_channel_history(pool: &PgPool, channel_id: Uuid, user_id: Uuid, username: &str, count: i64) -> DateTime<Utc> {
    let base_time = Utc.with_ymd_and_hms(2026, 3, 13, 12, 0, 0).unwrap();
    for i in 0..count {
        insert_test_message(
            pool,
            channel_id,
            user_id,
            username,
            base_time + Duration::seconds(i),
            i as u64,
            &format!("Message {}", i)
        ).await;
    }
    base_time
}