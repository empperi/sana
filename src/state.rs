use std::collections::HashMap;
use std::sync::{Mutex};
use tokio::sync::broadcast;
use async_nats::Client;
use sqlx::PgPool;
use axum_extra::extract::cookie::Key;
use axum::extract::FromRef;
use std::sync::Arc;
use crate::messages::MessageStore;

#[derive(Clone)]
pub struct AppState {
    pub channels: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    pub nats_client: Client,
    pub jetstream: async_nats::jetstream::Context,
    pub message_store: Arc<MessageStore>,
    pub db_pool: PgPool,
}

impl AppState {
    pub fn new(nats_client: Client, jetstream: async_nats::jetstream::Context, db_pool: PgPool) -> Self {
        let mut channels = HashMap::new();
        let (tx_gen, _rx_gen) = broadcast::channel(100);
        channels.insert("General".to_string(), tx_gen);
        
        let (tx_sys, _rx_sys) = broadcast::channel(100);
        channels.insert("system.channels".to_string(), tx_sys);

        Self {
            channels: Arc::new(Mutex::new(channels)),
            nats_client,
            jetstream,
            message_store: Arc::new(MessageStore::new()),
            db_pool,
        }
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
