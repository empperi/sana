use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub id: String,
    pub user: String,
    pub timestamp: i64,
    pub message: String,
    #[serde(default)]
    pub pending: bool,
    pub seq: Option<u64>,
}
