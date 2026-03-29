use dashmap::DashMap;
use tokio::sync::broadcast;
use async_nats::Client;
use sqlx::PgPool;
use axum_extra::extract::cookie::Key;
use axum::extract::FromRef;
use std::sync::Arc;
use crate::messages::MessageStore;
use chrono::{DateTime, Utc};

use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub channels: Arc<DashMap<String, broadcast::Sender<String>>>,
    pub channel_ids: Arc<DashMap<String, Uuid>>,
    pub nats_client: Client,
    pub jetstream: async_nats::jetstream::Context,
    pub message_store: Arc<MessageStore>,
    pub db_pool: PgPool,
    pub session_cache: Arc<DashMap<Uuid, DateTime<Utc>>>,
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
            session_cache: Arc::new(DashMap::new()),
        }
    }

    pub async fn validate_session(&self, user_id: Uuid) -> bool {
        // 1. Check cache
        if let Some(timestamp) = self.session_cache.get(&user_id) {
            if Utc::now() - *timestamp < chrono::Duration::seconds(60) {
                return true;
            }
        }

        // 2. Check DB
        let mut tx = match self.db_pool.begin().await {
            Ok(tx) => tx,
            Err(_) => return false,
        };

        match crate::db::users::get_user_by_id(&mut tx, user_id).await {
            Ok(Some(_)) => {
                self.session_cache.insert(user_id, Utc::now());
                true
            }
            _ => false,
        }
    }

    pub fn invalidate_session(&self, user_id: Uuid) {
        self.session_cache.remove(&user_id);
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
