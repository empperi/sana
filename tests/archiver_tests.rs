mod db;

use sana::logic::archiver;
use sana::state::AppState;
use sana::config::Config;
use sana::messages::{ChannelEntry, ChatMessage, MessageType};
use sana::db::channels::{self, Channel};
use sana::db::users;
use uuid::Uuid;
use chrono::Utc;
use crate::db::common::TestContext;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

fn init_test_logging() {
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("sana=debug".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();
}

#[tokio::test]
async fn test_archiver_message_persistence() {
    init_test_logging();
    let state = setup_app_state("test_archiver_persistence").await;
    let channel_name = format!("archiver-test-{}", Uuid::new_v4());
    let durable_name = format!("test-archiver-{}", Uuid::new_v4());
    
    // 1. Create channel and user in DB
    let channel = Channel {
        id: Uuid::new_v4(),
        name: channel_name.clone(),
        is_private: false,
        created_at: Utc::now(),
    };

    let mut tx = state.db_pool.begin().await.unwrap();
    channels::insert_channel(&mut tx, &channel).await.unwrap();
    let user = users::create_user(&mut tx, &format!("user-{}", Uuid::new_v4()), "pw").await.unwrap();
    tx.commit().await.unwrap();
    
    // 2. Start archiver
    archiver::start_with_durable(state.clone(), durable_name).await;
    
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // 3. Publish message to NATS
    let msg_id = Uuid::new_v4();
    let chat_msg = ChatMessage {
        id: msg_id,
        channel_id: channel.id,
        user_id: user.id,
        user: user.username.clone(),
        timestamp: Utc::now(),
        message: "persistent message".to_string(),
        seq: None,
        msg_type: MessageType::Chat,
    };
    let entry = ChannelEntry::Message(chat_msg);
    let payload = serde_json::to_string(&entry).unwrap();
    let subject = format!("topic.{}", sana::nats_util::encode(&channel_name));
    
    state.jetstream.publish(subject, payload.into()).await.unwrap().await.unwrap();
    
    // 4. Wait and verify in DB
    let mut found = false;
    for _ in 0..20 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let mut tx = state.db_pool.begin().await.unwrap();
        let msgs = sana::db::messages::get_messages(&mut tx, channel.id, 10, None, true).await.unwrap();
        if msgs.iter().any(|m| m.id == msg_id) {
            found = true;
            break;
        }
    }
    
    assert!(found, "Message was not archived to database");
}

#[tokio::test]
async fn test_archiver_system_channel() {
    init_test_logging();
    let state = setup_app_state("test_archiver_system").await;
    let durable_name = format!("test-archiver-sys-{}", Uuid::new_v4());
    
    archiver::start_with_durable(state.clone(), durable_name).await;
    
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let new_channel = Channel {
        id: Uuid::new_v4(),
        name: format!("nats-chan-{}", Uuid::new_v4()),
        is_private: false,
        created_at: Utc::now(),
    };
    let payload = serde_json::to_string(&new_channel).unwrap();
    
    state.jetstream.publish("topic.system.channels", payload.into()).await.unwrap().await.unwrap();
    
    let mut found = false;
    for _ in 0..20 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let mut tx = state.db_pool.begin().await.unwrap();
        if let Ok(Some(c)) = sana::db::channels::get_channel_by_name(&mut tx, &new_channel.name).await {
            if c.id == new_channel.id {
                found = true;
                break;
            }
        }
    }
    
    assert!(found, "System channel was not archived to database");
}
