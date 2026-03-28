mod db;

use sana::state::AppState;
use sana::config::Config;
use sana::db::channels::{self, Channel};
use uuid::Uuid;
use chrono::Utc;
use crate::db::common::TestContext;
use std::sync::Arc;

async fn setup_app_state(db_name: &str) -> AppState {
    let ctx = TestContext::new(db_name).await;
    let config = Config::new();
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    AppState::new(nats_client, jetstream, ctx.pool)
}

#[tokio::test]
async fn test_concurrent_channel_registration() {
    let state = setup_app_state("test_concurrent_reg").await;
    let state_arc = Arc::new(state);
    
    let mut handles = vec![];
    
    for i in 0..10 {
        let state = Arc::clone(&state_arc);
        let handle = tokio::spawn(async move {
            let channel_name = format!("channel-{}", i);
            let channel_id = Uuid::new_v4();
            
            // Simulating what happens during channel creation
            state.channel_ids.insert(channel_name.clone(), channel_id);
            state.channels.entry(channel_name).or_insert_with(|| {
                let (tx, _rx) = tokio::sync::broadcast::channel(100);
                tx
            });
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
    
    assert_eq!(state_arc.channel_ids.len(), 10);
    assert_eq!(state_arc.channels.len(), 10);
}

#[tokio::test]
async fn test_load_channels_from_db() {
    let state = setup_app_state("test_load_channels").await;
    
    // 1. Insert some channels to DB
    let mut tx = state.db_pool.begin().await.unwrap();
    let c1 = Channel {
        id: Uuid::new_v4(),
        name: "chan1".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    let c2 = Channel {
        id: Uuid::new_v4(),
        name: "chan2".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    channels::insert_channel(&mut tx, &c1).await.unwrap();
    channels::insert_channel(&mut tx, &c2).await.unwrap();
    tx.commit().await.unwrap();
    
    // 2. Load into state
    state.load_channels_from_db().await.unwrap();
    
    // 3. Verify
    assert!(state.channel_ids.contains_key("chan1"));
    assert!(state.channel_ids.contains_key("chan2"));
    assert!(state.channels.contains_key("chan1"));
    assert!(state.channels.contains_key("chan2"));
    assert!(state.channels.contains_key("system.channels"));
    
    assert_eq!(*state.channel_ids.get("chan1").unwrap(), c1.id);
}
