use std::collections::{HashMap, HashSet};
use crate::types::{ChatMessage, Channel, ChannelEntry};
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
    pub messages: HashMap<String, Vec<ChannelEntry>>,
    pub last_read_message_map: HashMap<String, Option<Uuid>>,
    pub last_read_message_index: HashMap<String, Option<usize>>,
    pub unread_channels: HashSet<String>,
    pub subscribed_channels: HashSet<String>,
    pub connection_status: ConnectionStatus,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            channel_id_map: HashMap::new(),
            pending_channels: HashSet::new(),
            current_channel: "General".to_string(), // Keep as default selection
            username: String::new(),
            user_id: Uuid::nil(),
            messages: HashMap::new(),
            last_read_message_map: HashMap::new(),
            last_read_message_index: HashMap::new(),
            unread_channels: HashSet::new(),
            subscribed_channels: HashSet::new(),
            connection_status: ConnectionStatus::Disconnected,
        }
    }

    pub fn handle_message(&mut self, channel: String, entry: ChannelEntry) {
        match entry {
            ChannelEntry::Metadata { last_read_message_id } => {
                self.last_read_message_map.insert(channel.clone(), last_read_message_id);
                self.update_read_index(&channel);
                return;
            }
            ChannelEntry::Batch(entries) => {
                for e in entries {
                    self.handle_single_entry(&channel, e);
                }
                return;
            }
            ChannelEntry::ReadMarker { user_id, message_id } => {
                if user_id == self.user_id {
                    self.last_read_message_map.insert(channel.clone(), Some(message_id));
                    self.update_read_index(&channel);
                }
                return;
            }
            _ => {
                self.handle_single_entry(&channel, entry);
            }
        }
    }

    fn update_read_index(&mut self, channel: &str) {
        let last_id = self.last_read_message_map.get(channel).cloned().flatten();
        let messages = self.messages.get(channel);
        
        let index = if let (Some(id), Some(msgs)) = (last_id, messages) {
            msgs.iter().position(|e| {
                match e {
                    ChannelEntry::Message(m) => m.id == id,
                    ChannelEntry::UserJoined { id: eid, .. } => *eid == id,
                    _ => false,
                }
            })
        } else {
            None
        };

        self.last_read_message_index.insert(channel.to_string(), index);
        self.recalculate_unread(channel);
    }

    fn recalculate_unread(&mut self, channel: &str) {
        if channel == self.current_channel {
            self.unread_channels.remove(channel);
            return;
        }

        let msgs = self.messages.get(channel);
        let read_idx = self.last_read_message_index.get(channel).cloned().flatten();

        let has_unread = match (msgs, read_idx) {
            (Some(msgs), Some(idx)) => {
                // If the last read message is not the last message in the list
                idx < msgs.len() - 1
            }
            (Some(msgs), None) => !msgs.is_empty(),
            _ => false,
        };

        if has_unread {
            self.unread_channels.insert(channel.to_string());
        } else {
            self.unread_channels.remove(channel);
        }
    }

    fn handle_single_entry(&mut self, channel: &str, entry: ChannelEntry) {
        let messages = self.messages.entry(channel.to_string()).or_insert_with(Vec::new);
        
        let entry_id = match &entry {
            ChannelEntry::Message(m) => m.id,
            ChannelEntry::UserJoined { id, .. } => *id,
            _ => return, // Should not happen here
        };

        // Find existing message by ID
        if let Some(pos) = messages.iter().position(|e| {
            let id = match e {
                ChannelEntry::Message(m) => m.id,
                ChannelEntry::UserJoined { id, .. } => *id,
                _ => Uuid::nil(),
            };
            id == entry_id
        }) {
            // Update if the existing message is pending or if the new one has a sequence number
            let should_update = match (&messages[pos], &entry) {
                (ChannelEntry::Message(old), ChannelEntry::Message(new)) => {
                    old.pending || (old.seq.is_none() && new.seq.is_some())
                },
                _ => false, // Non-message entries are idempotent based on ID
            };

            if should_update {
                messages[pos] = entry;
            }
        } else {
            // New message, add it
            messages.push(entry);
        }
        
        self.update_read_index(channel);
    }

    pub fn prepend_historical_messages(&mut self, channel: String, mut history: Vec<ChannelEntry>) {
        let messages = self.messages.entry(channel.clone()).or_insert_with(Vec::new);
        
        // The REST API returns messages in DESC order (newest of history first).
        // We want them in ASC order in our state.
        history.reverse();

        let mut new_entries = Vec::new();
        for entry in history {
            let entry_id = match &entry {
                ChannelEntry::Message(m) => m.id,
                ChannelEntry::UserJoined { id, .. } => *id,
                _ => continue, // REST API shouldn't return Metadata or Batch
            };
            
            // Deduplicate: check if it already exists
            let exists = messages.iter().any(|e| {
                let id = match e {
                    ChannelEntry::Message(m) => m.id,
                    ChannelEntry::UserJoined { id, .. } => *id,
                    _ => Uuid::nil(),
                };
                id == entry_id
            });
            
            if !exists {
                new_entries.push(entry);
            }
        }
        
        // Prepend non-duplicate historical messages
        // Since history is older, they go to the beginning.
        // We assume 'history' is already sorted chronologically (oldest to newest among the batch).
        if !new_entries.is_empty() {
            new_entries.extend(messages.drain(..));
            *messages = new_entries;
            self.update_read_index(&channel);
        }
    }

    pub fn handle_system_message(&mut self, body: String) -> Option<String> {
        if let Ok(channel) = serde_json::from_str::<Channel>(&body) {
            let name = channel.name.clone();
            let is_already_joined = self.channels.contains(&name);
            let is_pending = self.pending_channels.contains(&name);

            // Update metadata map regardless
            self.channel_id_map.insert(name.clone(), channel.id);

            if is_pending {
                // If it was a channel we just created, keep it in the list and clear pending status
                self.pending_channels.remove(&name);
                return None; 
            }

            if !is_already_joined {
                return None;
            }
        }
        None
    }

    pub fn switch_channel(&mut self, channel: String) {
        let old_channel = self.current_channel.clone();
        self.current_channel = channel.clone();
        self.unread_channels.remove(&channel);
        self.recalculate_unread(&old_channel);
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

    pub fn join_channel(&mut self, channel: Channel) {
        if !self.channels.contains(&channel.name) {
            self.channels.push(channel.name.clone());
            self.channel_id_map.insert(channel.name, channel.id);
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
        messages.push(ChannelEntry::Message(msg));
    }
}
