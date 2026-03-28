use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

const MAX_LIVE_HISTORY: usize = 100;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, sqlx::Type)]
#[sqlx(type_name = "message_type", rename_all = "PascalCase")]
#[serde(rename_all = "PascalCase")]
pub enum MessageType {
    Chat,
    Join,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub user: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub seq: Option<u64>,
    pub msg_type: MessageType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReadMarker {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub last_message_read: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
pub enum ChannelEntry {
    #[serde(rename = "chat")]
    Message(ChatMessage),
    #[serde(rename = "join")]
    UserJoined {
        id: Uuid, // unique event id
        user_id: Uuid,
        username: String,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "metadata")]
    Metadata {
        last_read_message_id: Option<Uuid>,
    },
    #[serde(rename = "batch")]
    Batch(Vec<ChannelEntry>),
    #[serde(rename = "read_marker")]
    ReadMarker {
        user_id: Uuid,
        message_id: Uuid,
    },
}

impl ChannelEntry {
    pub fn get_id(&self) -> Option<Uuid> {
        match self {
            ChannelEntry::Message(m) => Some(m.id),
            ChannelEntry::UserJoined { id, .. } => Some(*id),
            _ => None,
        }
    }
}

pub struct MessageStore {
    // Map of channel_name -> List of entries
    messages: DashMap<String, Vec<ChannelEntry>>,
}

impl MessageStore {
    pub fn new() -> Self {
        Self {
            messages: DashMap::new(),
        }
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageStore {
    pub fn add_entry(&self, channel: &str, entry: ChannelEntry) {
        let Some(id) = entry.get_id() else {
            return;
        };

        let mut channel_entries = self.messages.entry(channel.to_string()).or_default();

        // Idempotency check for entries with IDs
        if channel_entries.iter().any(|e| e.get_id() == Some(id)) {
            return;
        }

        channel_entries.push(entry);

        if channel_entries.len() > MAX_LIVE_HISTORY {
            channel_entries.remove(0);
        }
    }

    pub fn get_entries(&self, channel: &str) -> Vec<ChannelEntry> {
        self.messages.get(channel).map(|r| r.value().clone()).unwrap_or_default()
    }

    pub fn get_entries_after(&self, channel: &str, last_id: Uuid) -> Vec<ChannelEntry> {
        if let Some(entries) = self.messages.get(channel) {
            if let Some(pos) = entries.iter().position(|e| e.get_id() == Some(last_id)) {
                return entries[pos + 1..].to_vec();
            }
            return entries.value().clone();
        }
        Vec::new()
    }
}
