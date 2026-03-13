use std::collections::HashMap;
use std::sync::{Mutex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

const MAX_LIVE_HISTORY: usize = 100;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub user: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub seq: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum ChannelEntry {
    #[serde(rename = "chat")]
    Message(ChatMessage),
    #[serde(rename = "join")]
    UserJoined {
        id: Uuid,
        username: String,
        timestamp: DateTime<Utc>,
    }
}

pub struct MessageStore {
    // Map of channel_name -> List of entries
    messages: Mutex<HashMap<String, Vec<ChannelEntry>>>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            messages: Mutex::new(HashMap::new()),
        }
    }

    pub fn add_entry(&self, channel: &str, entry: ChannelEntry) {
        let mut store = self.messages.lock().unwrap();
        let channel_entries = store.entry(channel.to_string()).or_insert_with(Vec::new);

        let entry_id = match &entry {
            ChannelEntry::Message(m) => m.id,
            ChannelEntry::UserJoined { id, .. } => *id,
        };

        // Idempotency check
        if !channel_entries.iter().any(|e| {
            let id = match e {
                ChannelEntry::Message(m) => m.id,
                ChannelEntry::UserJoined { id, .. } => *id,
            };
            id == entry_id
        }) {
            channel_entries.push(entry);
            if channel_entries.len() > MAX_LIVE_HISTORY {
                channel_entries.remove(0);
            }
        }
    }

    pub fn get_entries(&self, channel: &str) -> Vec<ChannelEntry> {
        let store = self.messages.lock().unwrap();
        store.get(channel).cloned().unwrap_or_default()
    }

    pub fn get_entries_after(&self, channel: &str, last_id: Uuid) -> Vec<ChannelEntry> {
        let store = self.messages.lock().unwrap();
        if let Some(entries) = store.get(channel) {
            if let Some(pos) = entries.iter().position(|e| {
                let id = match e {
                    ChannelEntry::Message(m) => m.id,
                    ChannelEntry::UserJoined { id, .. } => *id,
                };
                id == last_id
            }) {
                return entries[pos + 1..].to_vec();
            }
            return entries.clone();
        }
        Vec::new()
    }
}
