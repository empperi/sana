use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub user: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    #[serde(default)]
    pub pending: bool,
    pub seq: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub is_private: bool,
    pub created_at: DateTime<Utc>,
}
