use std::collections::HashMap;
use std::sync::{Mutex};
use tokio::sync::broadcast;
use async_nats::Client;
use sqlx::PgPool;
use axum_extra::extract::cookie::Key;
use axum::extract::FromRef;
use std::sync::Arc;
use crate::messages::MessageStore;

use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub channels: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    pub channel_ids: Arc<Mutex<HashMap<String, Uuid>>>,
    pub nats_client: Client,
    pub jetstream: async_nats::jetstream::Context,
    pub message_store: Arc<MessageStore>,
    pub db_pool: PgPool,
}

impl AppState {
    pub fn new(nats_client: Client, jetstream: async_nats::jetstream::Context, db_pool: PgPool) -> Self {
        let channels = HashMap::new();
        let channel_ids = HashMap::new();

        Self {
            channels: Arc::new(Mutex::new(channels)),
            channel_ids: Arc::new(Mutex::new(channel_ids)),
            nats_client,
            jetstream,
            message_store: Arc::new(MessageStore::new()),
            db_pool,
        }
    }

    pub async fn load_channels_from_db(&self) -> Result<(), sqlx::Error> {
        let channels = crate::db::channels::get_all_channels(&self.db_pool).await?;
        
        let mut ids = self.channel_ids.lock().unwrap();
        let mut chans = self.channels.lock().unwrap();
        
        for c in channels {
            ids.insert(c.name.clone(), c.id);
            chans.entry(c.name.clone()).or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(100);
                tx
            });
        }

        // Also ensure system.channels exists in chans map
        chans.entry("system.channels".to_string()).or_insert_with(|| {
            let (tx, _rx) = broadcast::channel(100);
            tx
        });

        Ok(())
    }
}

#[derive(Clone)]
pub struct CombinedState {
    pub app: AppState,
    pub cookie_key: Key,
}

impl FromRef<CombinedState> for AppState {
    fn from_ref(state: &CombinedState) -> Self {
        state.app.clone()
    }
}

impl FromRef<CombinedState> for Key {
    fn from_ref(state: &CombinedState) -> Self {
        state.cookie_key.clone()
    }
}
