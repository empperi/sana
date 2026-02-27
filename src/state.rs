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
        Self {
            channels: Mutex::new(HashMap::new()),
            nats_client,
            message_store: MessageStore::new(),
        }
    }
}
