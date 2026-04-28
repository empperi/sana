use crate::db::common::{TestContext, create_test_user, create_test_channel};
use sana::db::{attachments, messages};
use sana::messages::{ChatMessage, MessageType};
use uuid::Uuid;
use chrono::Utc;

#[tokio::test]
async fn test_insert_and_get_attachment() {
    let ctx = TestContext::new("test_insert_attachment").await;
    let user = create_test_user(&ctx.pool, "testuser").await;
    
    let mut tx = ctx.pool.begin().await.unwrap();
    let meta = attachments::insert_attachment(
        &mut tx,
        "test.png",
        "uuid-test.png",
        1024,
        "image/png",
        user.id
    ).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(meta.original_filename, "test.png");
    assert_eq!(meta.file_size, 1024);
    assert_eq!(meta.mime_type, "image/png");

    let mut tx = ctx.pool.begin().await.unwrap();
    let retrieved = attachments::get_attachment_by_id(&mut tx, meta.id).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(retrieved, meta);
}

#[tokio::test]
async fn test_link_attachments_to_message() {
    let ctx = TestContext::new("test_link_attachments").await;
    let user = create_test_user(&ctx.pool, "sender").await;
    let channel = create_test_channel(&ctx.pool, "test-channel").await;
    
    // 1. Insert two orphan attachments
    let mut tx = ctx.pool.begin().await.unwrap();
    let a1 = attachments::insert_attachment(&mut tx, "f1.txt", "s1.txt", 10, "text/plain", user.id).await.unwrap();
    let a2 = attachments::insert_attachment(&mut tx, "f2.txt", "s2.txt", 20, "text/plain", user.id).await.unwrap();
    tx.commit().await.unwrap();

    // 2. Create a message
    let msg = ChatMessage {
        id: Uuid::new_v4(),
        channel_id: channel.id,
        user_id: user.id,
        user: user.username.clone(),
        timestamp: Utc::now(),
        message: "hello".to_string(),
        seq: None,
        msg_type: MessageType::Chat,
        attachments: Vec::new(),
    };
    let mut tx = ctx.pool.begin().await.unwrap();
    messages::insert_message(&mut tx, 1, &msg).await.unwrap();
    tx.commit().await.unwrap();

    // 3. Link them
    let mut tx = ctx.pool.begin().await.unwrap();
    attachments::link_attachments_to_message(&mut tx, &[a1.id, a2.id], msg.id, user.id).await.unwrap();
    tx.commit().await.unwrap();

    // 4. Verify linking
    let mut tx = ctx.pool.begin().await.unwrap();
    let linked = attachments::get_attachments_by_message_id(&mut tx, msg.id).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(linked.len(), 2);
    assert!(linked.contains(&a1));
    assert!(linked.contains(&a2));
}

#[tokio::test]
async fn test_link_attachments_security() {
    let ctx = TestContext::new("test_link_security").await;
    let user1 = create_test_user(&ctx.pool, "user1").await;
    let user2 = create_test_user(&ctx.pool, "user2").await;
    let channel = create_test_channel(&ctx.pool, "test-channel").await;
    
    // User 1 uploads a file
    let mut tx = ctx.pool.begin().await.unwrap();
    let a1 = attachments::insert_attachment(&mut tx, "f1.txt", "s1.txt", 10, "text/plain", user1.id).await.unwrap();
    tx.commit().await.unwrap();

    // User 2 tries to link User 1's file to User 2's message
    let msg2 = ChatMessage {
        id: Uuid::new_v4(),
        channel_id: channel.id,
        user_id: user2.id,
        user: user2.username.clone(),
        timestamp: Utc::now(),
        message: "hacker".to_string(),
        seq: None,
        msg_type: MessageType::Chat,
        attachments: Vec::new(),
    };
    let mut tx = ctx.pool.begin().await.unwrap();
    messages::insert_message(&mut tx, 1, &msg2).await.unwrap();
    tx.commit().await.unwrap();

    // Link attempt by user2 for user1's attachment
    let mut tx = ctx.pool.begin().await.unwrap();
    attachments::link_attachments_to_message(&mut tx, &[a1.id], msg2.id, user2.id).await.unwrap();
    tx.commit().await.unwrap();

    // Verify it was NOT linked
    let mut tx = ctx.pool.begin().await.unwrap();
    let linked = attachments::get_attachments_by_message_id(&mut tx, msg2.id).await.unwrap();
    tx.commit().await.unwrap();

    assert!(linked.is_empty(), "User 2 should not be able to link User 1's attachment");
}

#[tokio::test]
async fn test_get_messages_populates_attachments() {
    let ctx = TestContext::new("test_get_msgs_attachments").await;
    let user = create_test_user(&ctx.pool, "sender").await;
    let channel = create_test_channel(&ctx.pool, "test-channel").await;
    
    // 1. Insert attachment
    let mut tx = ctx.pool.begin().await.unwrap();
    let a1 = attachments::insert_attachment(&mut tx, "f1.txt", "s1.txt", 10, "text/plain", user.id).await.unwrap();
    tx.commit().await.unwrap();

    // 2. Create message and link
    let msg = ChatMessage {
        id: Uuid::new_v4(),
        channel_id: channel.id,
        user_id: user.id,
        user: user.username.clone(),
        timestamp: Utc::now(),
        message: "hello".to_string(),
        seq: None,
        msg_type: MessageType::Chat,
        attachments: Vec::new(),
    };
    let mut tx = ctx.pool.begin().await.unwrap();
    messages::insert_message(&mut tx, 1, &msg).await.unwrap();
    attachments::link_attachments_to_message(&mut tx, &[a1.id], msg.id, user.id).await.unwrap();
    tx.commit().await.unwrap();

    // 3. Fetch messages and verify attachments
    let mut tx = ctx.pool.begin().await.unwrap();
    let msgs = messages::get_messages(&mut tx, channel.id, 10, None, true).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].attachments.len(), 1);
    assert_eq!(msgs[0].attachments[0].id, a1.id);
    assert_eq!(msgs[0].attachments[0].original_filename, "f1.txt");
}
