use dashmap::DashMap;
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
    pub channels: Arc<DashMap<String, broadcast::Sender<String>>>,
    pub channel_ids: Arc<DashMap<String, Uuid>>,
    pub nats_client: Client,
    pub jetstream: async_nats::jetstream::Context,
    pub message_store: Arc<MessageStore>,
    pub db_pool: PgPool,
}

impl AppState {
    pub fn new(nats_client: Client, jetstream: async_nats::jetstream::Context, db_pool: PgPool) -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
            channel_ids: Arc::new(DashMap::new()),
            nats_client,
            jetstream,
            message_store: Arc::new(MessageStore::new()),
            db_pool,
        }
    }

    pub async fn load_channels_from_db(&self) -> Result<(), sqlx::Error> {
        let channels = crate::db::channels::get_all_channels(&self.db_pool).await?;
        
        for c in channels {
            self.channel_ids.insert(c.name.clone(), c.id);
            self.channels.entry(c.name.clone()).or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(100);
                tx
            });
        }

        // Also ensure system.channels exists in chans map
        self.channels.entry("system.channels".to_string()).or_insert_with(|| {
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
