use std::collections::HashMap;
use std::sync::{Mutex};
use tokio::sync::broadcast;
use async_nats::Client;
use crate::messages::MessageStore;

pub struct AppState {
    pub channels: Mutex<HashMap<String, broadcast::Sender<String>>>,
    pub nats_client: Client,
    pub message_store: MessageStore,
}

impl AppState {
    pub fn new(nats_client: Client) -> Self {
        let mut channels = HashMap::new();
        let (tx, _rx) = broadcast::channel(100);
        channels.insert("General".to_string(), tx);

        Self {
            channels: Mutex::new(channels),
            nats_client,
            message_store: MessageStore::new(),
        }
    }
}
