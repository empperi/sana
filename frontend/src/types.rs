use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
    #[serde(default)]
    pub pending: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum ChannelEntry {
    #[serde(rename = "chat")]
    Message(ChatMessage),
    #[serde(rename = "join")]
    UserJoined {
        id: Uuid,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub is_private: bool,
    pub created_at: DateTime<Utc>,
}
