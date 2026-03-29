mod db;

use sana::logic::nats;
use sana::state::AppState;
use sana::config::Config;
use sana::messages::{ChannelEntry, ChatMessage, MessageType};
use sana::db::channels::Channel;
use uuid::Uuid;
use chrono::Utc;
use crate::db::common::TestContext;

async fn setup_app_state(db_name: &str) -> AppState {
    let ctx = TestContext::new(db_name).await;
    let config = Config::new();
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    
    // Ensure stream exists
    jetstream.get_or_create_stream(async_nats::jetstream::stream::Config {
        name: "SANA".to_string(),
        subjects: vec!["topic.>".to_string()],
        ..Default::default()
    }).await.unwrap();

    AppState::new(nats_client, jetstream, ctx.pool)
}

#[tokio::test]
async fn test_nats_broadcast_relay() {
    let state = setup_app_state("test_nats_relay").await;
    let channel_name = format!("relay-test-{}", Uuid::new_v4());
    
    // 1. Create broadcast channel in state
    let (tx, mut rx) = tokio::sync::broadcast::channel(100);
    state.channels.insert(channel_name.clone(), tx);
    
    // 2. Start subscriber
    nats::start_nats_subscriber(state.clone()).await;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // 3. Publish to NATS
    let msg_id = Uuid::new_v4();
    let chat_msg = ChatMessage {
        id: msg_id,
        channel_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        user: "nats_tester".to_string(),
        timestamp: Utc::now(),
        message: "relay me".to_string(),
        seq: None,
        msg_type: MessageType::Chat,
    };
    let entry = ChannelEntry::Message(chat_msg);
    let payload = serde_json::to_string(&entry).unwrap();
    let subject = format!("topic.{}", sana::nats_util::encode(&channel_name));
    
    state.jetstream.publish(subject, payload.into()).await.unwrap().await.unwrap();
    
    // 4. Verify reception from internal broadcast
    let received_raw = tokio::time::timeout(tokio::time::Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
    let received_entry: ChannelEntry = serde_json::from_str(&received_raw).unwrap();
    
    if let ChannelEntry::Message(m) = received_entry {
        assert_eq!(m.id, msg_id);
        assert!(m.seq.is_some(), "Sequence number should be populated by subscriber");
    } else {
        panic!("Expected Message entry");
    }
}

#[tokio::test]
async fn test_nats_system_channel_relay() {
    let state = setup_app_state("test_nats_sys_relay").await;
    
    // 1. Ensure system.channels exists
    let (tx, mut rx) = tokio::sync::broadcast::channel(100);
    state.channels.insert("system.channels".to_string(), tx);
    
    // 2. Start subscriber
    nats::start_nats_subscriber(state.clone()).await;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // 3. Publish new channel to NATS
    let new_channel = Channel {
        id: Uuid::new_v4(),
        name: format!("new-chan-{}", Uuid::new_v4()),
        is_private: false,
        created_at: Utc::now(),
    };
    let payload = serde_json::to_string(&new_channel).unwrap();
    
    state.jetstream.publish("topic.system.channels", payload.into()).await.unwrap().await.unwrap();
    
    // 4. Verify reception and state update
    let received_raw = tokio::time::timeout(tokio::time::Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
    let received_chan: Channel = serde_json::from_str(&received_raw).unwrap();
    
    assert_eq!(received_chan.id, new_channel.id);
    assert_eq!(received_chan.name, new_channel.name);
    
    // State cache check
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    assert!(state.channel_ids.contains_key(&new_channel.name));
    assert_eq!(*state.channel_ids.get(&new_channel.name).unwrap(), new_channel.id);
}
