use std::collections::HashMap;
use std::sync::{Mutex};
use tokio::sync::broadcast;
use async_nats::Client;
use crate::messages::MessageStore;

pub struct AppState {
    pub channels: Mutex<HashMap<String, broadcast::Sender<String>>>,
    pub nats_client: Client,
    pub jetstream: async_nats::jetstream::Context,
    pub message_store: MessageStore,
}

impl AppState {
    pub fn new(nats_client: Client, jetstream: async_nats::jetstream::Context) -> Self {
        let mut channels = HashMap::new();
        let (tx_gen, _rx_gen) = broadcast::channel(100);
        channels.insert("General".to_string(), tx_gen);
        
        let (tx_sys, _rx_sys) = broadcast::channel(100);
        channels.insert("system.channels".to_string(), tx_sys);

        Self {
            channels: Mutex::new(channels),
            nats_client,
            jetstream,
            message_store: MessageStore::new(),
        }
    }
}
