use std::collections::HashMap;
use std::sync::{Mutex};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub id: String,
    pub user: String,
    pub timestamp: i64,
    pub message: String,
}

pub struct MessageStore {
    // Map of channel_name -> List of messages
    messages: Mutex<HashMap<String, Vec<ChatMessage>>>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_message(&self, channel: &str, message: ChatMessage) {
        let mut store = self.messages.lock().unwrap();
        let channel_messages = store.entry(channel.to_string()).or_insert_with(Vec::new);

        // Check if message with same ID already exists (idempotency)
        if !channel_messages.iter().any(|m| m.id == message.id) {
            channel_messages.push(message);
        }
    }

    pub fn get_messages(&self, channel: &str) -> Vec<ChatMessage> {
        let store = self.messages.lock().unwrap();
        store.get(channel).cloned().unwrap_or_default()
    }
}
