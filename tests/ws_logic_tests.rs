mod db;

use sana::logic::ws_logic::*;
use sana::messages::{ChannelEntry, ChatMessage, MessageType};
use sana::state::AppState;
use sana::config::Config;
use sana::stomp::StompCommand;
use uuid::Uuid;
use chrono::Utc;
use crate::db::common::TestContext;

async fn setup_app_state(db_name: &str) -> AppState {
    let ctx = TestContext::new(db_name).await;
    let config = Config::new();
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    AppState::new(nats_client, jetstream, ctx.pool)
}

#[test]
fn test_merge_and_deduplicate() {
    let channel_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    
    let msg1 = ChatMessage {
        id: Uuid::new_v4(),
        channel_id,
        user_id,
        user: "alice".to_string(),
        timestamp: Utc::now(),
        message: "hello".to_string(),
        seq: Some(1),
        msg_type: MessageType::Chat,
    };
    
    let msg2 = ChatMessage {
        id: Uuid::new_v4(),
        channel_id,
        user_id,
        user: "bob".to_string(),
        timestamp: Utc::now(),
        message: "world".to_string(),
        seq: Some(2),
        msg_type: MessageType::Chat,
    };
    
    let db_history = vec![
        ChannelEntry::Message(msg1.clone()),
    ];
    
    let mem_history = vec![
        ChannelEntry::Message(msg1.clone()), // duplicate
        ChannelEntry::Message(msg2.clone()),
    ];
    
    let merged = merge_and_deduplicate(db_history, mem_history);
    
    assert_eq!(merged.len(), 2);
    if let ChannelEntry::Message(m) = &merged[0] {
        assert_eq!(m.id, msg1.id);
    }
    if let ChannelEntry::Message(m) = &merged[1] {
        assert_eq!(m.id, msg2.id);
    }
}

#[tokio::test]
async fn test_send_in_batches() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let channel_id = Uuid::new_v4();
    let history: Vec<ChannelEntry> = (0..25).map(|i| {
        ChannelEntry::Message(ChatMessage {
            id: Uuid::new_v4(),
            channel_id,
            user_id: Uuid::new_v4(),
            user: "user".to_string(),
            timestamp: Utc::now(),
            message: format!("msg {}", i),
            seq: Some(i as u64),
            msg_type: MessageType::Chat,
        })
    }).collect();
    
    send_in_batches("test-chan", history, &tx).await;
    
    // Should receive 2 batches (20 + 5)
    let batch1_str = rx.recv().await.unwrap();
    let batch2_str = rx.recv().await.unwrap();
    
    assert!(batch1_str.contains("MESSAGE"));
    assert!(batch2_str.contains("MESSAGE"));
    
    // Extract JSON part (after double newline)
    let json1 = batch1_str.split("\n\n").nth(1).unwrap().trim_matches('\0');
    let json2 = batch2_str.split("\n\n").nth(1).unwrap().trim_matches('\0');
    
    let entry1: ChannelEntry = serde_json::from_str(json1).unwrap();
    let entry2: ChannelEntry = serde_json::from_str(json2).unwrap();
    
    if let ChannelEntry::Batch(b1) = entry1 {
        assert_eq!(b1.len(), 20);
    } else { panic!("Expected batch"); }
    
    if let ChannelEntry::Batch(b2) = entry2 {
        assert_eq!(b2.len(), 5);
    } else { panic!("Expected batch"); }
}

#[test]
fn test_build_chat_message() {
    let msg_id = Uuid::new_v4();
    let channel_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let username = "tester";
    let body = "hello world";
    
    let msg = build_chat_message(
        msg_id,
        channel_id,
        user_id,
        username,
        body.to_string()
    );
    
    assert_eq!(msg.id, msg_id);
    assert_eq!(msg.channel_id, channel_id);
    assert_eq!(msg.user_id, user_id);
    assert_eq!(msg.user, username);
    assert_eq!(msg.message, body);
    assert_eq!(msg.seq, None);
}

#[tokio::test]
async fn test_resolve_channel_id() {
    let state = setup_app_state("test_resolve_chan_id").await;
    let channel_name = "test-resolve";
    
    // 1. Resolve non-existent
    let id = resolve_channel_id(channel_name, &state).await;
    assert!(id.is_err());
    
    // 2. Create and resolve
    let channel = sana::db::channels::Channel {
        id: Uuid::new_v4(),
        name: channel_name.to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    let mut tx = state.db_pool.begin().await.unwrap();
    sana::db::channels::insert_channel(&mut tx, &channel).await.unwrap();
    tx.commit().await.unwrap();
    
    let id = resolve_channel_id(channel_name, &state).await;
    assert_eq!(id.unwrap(), channel.id);
}

#[tokio::test]
async fn test_handle_subscribe_basic() {
    let state = setup_app_state("test_handle_sub_basic").await;
    let (tx_internal, mut rx_internal) = tokio::sync::mpsc::channel(100);
    let user_id = Uuid::new_v4();
    let channel_name = "general".to_string();
    
    // Create channel
    let channel = sana::db::channels::Channel {
        id: Uuid::new_v4(),
        name: channel_name.clone(),
        is_private: false,
        created_at: Utc::now(),
    };
    let mut tx = state.db_pool.begin().await.unwrap();
    sana::db::channels::insert_channel(&mut tx, &channel).await.unwrap();
    tx.commit().await.unwrap();
    
    handle_subscribe(channel_name.clone(), None, user_id, &state, &tx_internal).await;
    
    // Should receive Metadata message
    let meta_msg = rx_internal.recv().await.unwrap();
    assert!(meta_msg.contains("MESSAGE"));
    assert!(meta_msg.contains("metadata"));
}

#[tokio::test]
async fn test_decide_read_marker() {
    let user_id = Uuid::new_v4();
    let ctx = WsContext {
        user_id,
        username: "tester".to_string(),
    };
    let message_id = Uuid::new_v4();
    
    let command = StompCommand::Send {
        destination: "/topic/general".to_string(),
        body: message_id.to_string(),
        headers: vec![("message-type".to_string(), "read_marker".to_string())],
    };
    
    let actions = decide(command, &ctx);
    assert_eq!(actions.len(), 1);
    if let WsAction::PublishReadMarker(chan, mid) = &actions[0] {
        assert_eq!(chan, "general");
        assert_eq!(mid, &message_id);
    } else {
        panic!("Expected PublishReadMarker action");
    }
}

#[tokio::test]
async fn test_process_and_publish_message_basic() {
    let state = setup_app_state("test_process_publish").await;
    let user_id = Uuid::new_v4();
    let channel_name = "general";
    
    // Create channel
    let channel = sana::db::channels::Channel {
        id: Uuid::new_v4(),
        name: channel_name.to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    let mut tx = state.db_pool.begin().await.unwrap();
    sana::db::channels::insert_channel(&mut tx, &channel).await.unwrap();
    tx.commit().await.unwrap();
    
    // This will publish to NATS. We don't verify NATS reception here (that's infrastructure test), 
    // but we verify it doesn't crash and resolves correctly.
    process_and_publish_message(
        "topic.general".to_string(),
        "hello".to_string(),
        None,
        user_id,
        "tester",
        channel_name,
        &state
    ).await.unwrap();
}
