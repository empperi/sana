use std::collections::{HashMap, HashSet};
use crate::types::{ChatMessage, Channel};
use crate::services::websocket::ConnectionStatus;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub struct ChatState {
    pub channels: Vec<String>,
    pub channel_id_map: HashMap<String, Uuid>,
    pub pending_channels: HashSet<String>,
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
            pending_channels: HashSet::new(),
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

    pub fn handle_system_message(&mut self, body: String) -> Option<String> {
        if let Ok(channel) = serde_json::from_str::<Channel>(&body) {
            let name = channel.name.clone();
            let is_new = !self.channels.contains(&name);
            let is_pending = self.pending_channels.contains(&name);

            if is_new || is_pending {
                if is_new {
                    self.channels.push(name.clone());
                }
                self.channel_id_map.insert(name.clone(), channel.id);
                self.pending_channels.remove(&name);
                
                if is_new {
                    return Some(name);
                }
            }
        }
        None
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
    
    pub fn set_channels(&mut self, channels: Vec<Channel>) {
        self.channels = channels.iter().map(|c| c.name.clone()).collect();
        self.channel_id_map = channels.into_iter().map(|c| (c.name, c.id)).collect();
        
        // Ensure "General" is always present if for some reason it's missing from API
        if !self.channels.contains(&"General".to_string()) {
            self.channels.insert(0, "General".to_string());
            self.channel_id_map.insert("General".to_string(), Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap());
        }
    }

    pub fn add_pending_channel(&mut self, name: String) {
        if !self.channels.contains(&name) {
            self.channels.push(name.clone());
            self.pending_channels.insert(name);
        }
    }

    pub fn add_pending_message(&mut self, channel: String, msg: ChatMessage) {
        let messages = self.messages.entry(channel).or_insert_with(Vec::new);
        messages.push(msg);
    }
}
