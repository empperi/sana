use std::collections::{HashMap, HashSet};
use crate::types::ChatMessage;
use crate::services::websocket::ConnectionStatus;

#[derive(Clone, Debug, PartialEq)]
pub struct ChatState {
    pub channels: Vec<String>,
    pub current_channel: String,
    pub username: String,
    pub messages: HashMap<String, Vec<ChatMessage>>,
    pub unread_channels: HashSet<String>,
    pub connection_status: ConnectionStatus,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            channels: vec!["General".to_string()],
            current_channel: "General".to_string(),
            username: String::new(),
            messages: HashMap::new(),
            unread_channels: HashSet::new(),
            connection_status: ConnectionStatus::Disconnected,
        }
    }

    pub fn handle_message(&mut self, channel: String, msg: ChatMessage) {
        let messages = self.messages.entry(channel.clone()).or_insert_with(Vec::new);
        
        // Update pending message or add new one
        if let Some(pos) = messages.iter().position(|m| m.id == msg.id && m.pending) {
            messages[pos] = msg;
        } else {
            messages.push(msg);
            
            // Mark as unread if not current channel
            if channel != self.current_channel {
                self.unread_channels.insert(channel);
            }
        }
    }

    pub fn handle_system_message(&mut self, body: String) {
        if !self.channels.contains(&body) {
            self.channels.push(body);
        }
    }

    pub fn switch_channel(&mut self, channel: String) {
        self.current_channel = channel.clone();
        self.unread_channels.remove(&channel);
    }

    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }

    pub fn set_username(&mut self, username: String) {
        self.username = username;
    }
    
    pub fn add_pending_message(&mut self, channel: String, msg: ChatMessage) {
        let messages = self.messages.entry(channel).or_insert_with(Vec::new);
        messages.push(msg);
    }
}
