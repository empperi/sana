use std::collections::{HashMap, HashSet};
use crate::types::{ChatMessage, Channel};
use crate::services::websocket::ConnectionStatus;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct ChatState {
    pub channels: Vec<String>,
    pub channel_id_map: HashMap<String, Uuid>,
    pub current_channel: String,
    pub username: String,
    pub user_id: Uuid,
    pub messages: HashMap<String, Vec<ChatMessage>>,
    pub unread_channels: HashSet<String>,
    pub connection_status: ConnectionStatus,
}

impl ChatState {
    pub fn new() -> Self {
        let mut channel_id_map = HashMap::new();
        // Hardcoded "General" ID matching backend for now
        channel_id_map.insert("General".to_string(), Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap());

        Self {
            channels: vec!["General".to_string()],
            channel_id_map,
            current_channel: "General".to_string(),
            username: String::new(),
            user_id: Uuid::nil(),
            messages: HashMap::new(),
            unread_channels: HashSet::new(),
            connection_status: ConnectionStatus::Disconnected,
        }
    }

    pub fn handle_message(&mut self, channel: String, msg: ChatMessage) {
        let messages = self.messages.entry(channel.clone()).or_insert_with(Vec::new);
        
        // Update pending message or add new one.
        if let Some(pos) = messages.iter().position(|m| m.id == msg.id && m.pending) {
            messages[pos] = msg;
        } else if !messages.iter().any(|m| m.id == msg.id) {
            messages.push(msg);
            
            if channel != self.current_channel {
                self.unread_channels.insert(channel);
            }
        }
    }

    pub fn handle_system_message(&mut self, body: String) {
        if let Ok(channel) = serde_json::from_str::<Channel>(&body) {
            if !self.channels.contains(&channel.name) {
                self.channels.push(channel.name.clone());
            }
            self.channel_id_map.insert(channel.name, channel.id);
        }
    }

    pub fn switch_channel(&mut self, channel: String) {
        self.current_channel = channel.clone();
        self.unread_channels.remove(&channel);
    }

    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }

    pub fn set_user_info(&mut self, username: String, user_id: Uuid) {
        self.username = username;
        self.user_id = user_id;
    }
    
    pub fn add_pending_message(&mut self, channel: String, msg: ChatMessage) {
        let messages = self.messages.entry(channel).or_insert_with(Vec::new);
        messages.push(msg);
    }
}
