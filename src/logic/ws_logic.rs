use crate::state::AppState;
use crate::stomp::StompCommand;
use bytes::Bytes;
use uuid::Uuid;
use crate::messages::{ChatMessage, ChannelEntry};

#[derive(Debug, PartialEq)]
pub enum WsAction {
    SendToClient(String),
    Subscribe(String, Option<u64>), // channel, last_seen_seq
    PublishToNats(String, String, Option<String>, String), // subject, body, message_id, channel_name
    SendReceipt(String), // receipt-id
    None,
}

pub fn decide(command: StompCommand, user_id: Uuid, username: &str) -> Vec<WsAction> {
    let mut actions = Vec::new();
    match command {
        StompCommand::Connect => {
            let response = format!("CONNECTED\nversion:1.2\nuser_id:{}\nusername:{}\n\n\0", user_id, username);
            actions.push(WsAction::SendToClient(response));
        },
        StompCommand::Subscribe { destination, headers, last_seen_seq, .. } => {
            if let Some(channel_name) = destination.strip_prefix("/topic/") {
                actions.push(WsAction::Subscribe(channel_name.to_string(), last_seen_seq));
                if let Some((_, receipt_id)) = headers.iter().find(|(k, _)| k == "receipt") {
                    actions.push(WsAction::SendReceipt(receipt_id.clone()));
                }
            }
        }
        StompCommand::Send { destination, body, headers } => {
            if let Some(channel_name) = destination.strip_prefix("/topic/") {
                let message_id = headers.iter()
                    .find(|(k, _)| k == "message_id")
                    .map(|(_, v)| v.clone());

                actions.push(WsAction::PublishToNats(
                    format!("topic.{}", crate::nats_util::encode(channel_name)), 
                    body, 
                    message_id,
                    channel_name.to_string()
                ));
                
                if let Some((_, receipt_id)) = headers.iter().find(|(k, _)| k == "receipt") {
                    actions.push(WsAction::SendReceipt(receipt_id.clone()));
                }
            }
        }
        StompCommand::Unknown => {}
    }
    actions
}

pub async fn handle_subscribe(channel_name: String, last_seen_seq: Option<u64>, state: &AppState, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    tracing::info!("Subscribing to channel: {}", channel_name);

    if channel_name == "system.channels" {
        send_initial_channels(state, tx_internal).await;
    } else {
        // Send missed messages
        let entries = state.message_store.get_entries(&channel_name);
        for entry in entries {
            let (seq, _entry_id) = match &entry {
                ChannelEntry::Message(m) => (m.seq, m.id),
                ChannelEntry::UserJoined { id, .. } => (None, *id),
            };

            let should_send = match (seq, last_seen_seq) {
                (Some(s), Some(last)) => s > last,
                _ => true, 
            };
            
            if should_send {
                if let Ok(entry_json) = serde_json::to_string(&entry) {
                    let seq_header = seq.map(|s| format!("seq:{}\n", s)).unwrap_or_default();
                    let stomp_msg = format!("MESSAGE\ndestination:/topic/{}\n{}\n{}\0", channel_name, seq_header, entry_json);
                    let _ = tx_internal.send(stomp_msg).await;
                }
            }
        }
    }

    // Setup broadcast channel and listener
    let tx = get_or_create_broadcast_channel(&channel_name, state);
    subscribe_to_broadcast(channel_name.clone(), tx, tx_internal).await;
}

async fn send_initial_channels(state: &AppState, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    let channels_list: Vec<String> = {
        let channels = state.channels.lock().unwrap();
        channels.keys().cloned().collect()
    };
    
    for name in channels_list {
        if name != "system.channels" {
            let stomp_msg = format!("MESSAGE\ndestination:/topic/system.channels\n\n{}\0", name);
            let _ = tx_internal.send(stomp_msg).await;
        }
    }
}

fn get_or_create_broadcast_channel(channel_name: &str, state: &AppState) -> tokio::sync::broadcast::Sender<String> {
    let mut channels = state.channels.lock().unwrap();
    let mut channel_ids = state.channel_ids.lock().unwrap();
    
    let is_new = !channels.contains_key(channel_name);
    
    let tx = channels.entry(channel_name.to_string())
        .or_insert_with(|| {
            let (tx, _rx) = tokio::sync::broadcast::channel(100);
            tx
        })
        .clone();

    if is_new && channel_name != "system.channels" {
        let channel_id = *channel_ids.entry(channel_name.to_string())
            .or_insert_with(Uuid::new_v4);
            
        publish_new_channel(channel_name, channel_id, &state.nats_client);
    }
    tx
}

fn publish_new_channel(channel_name: &str, channel_id: Uuid, nats_client: &async_nats::Client) {
    let nats_client = nats_client.clone();
    let channel = crate::db::channels::Channel {
        id: channel_id,
        name: channel_name.to_string(),
        is_private: false,
        created_at: chrono::Utc::now(),
    };

    tokio::spawn(async move {
        if let Ok(payload) = serde_json::to_string(&channel) {
            let _ = nats_client.publish("topic.system.channels", Bytes::from(payload)).await;
        }
    });
}

async fn subscribe_to_broadcast(channel_name: String, tx: tokio::sync::broadcast::Sender<String>, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    let mut rx = tx.subscribe();
    let tx_internal = tx_internal.clone();

    tokio::spawn(async move {
        while let Ok(msg_json) = rx.recv().await {
            // Extract seq from JSON if possible to add as header
            let seq_header = if let Ok(entry) = serde_json::from_str::<ChannelEntry>(&msg_json) {
                match entry {
                    ChannelEntry::Message(m) => m.seq.map(|s| format!("seq:{}\n", s)).unwrap_or_default(),
                    _ => "".to_string(),
                }
            } else {
                "".to_string()
            };

            let stomp_msg = format!("MESSAGE\ndestination:/topic/{}\n{}\n{}\0", channel_name, seq_header, msg_json);
            if tx_internal.send(stomp_msg).await.is_err() {
                break;
            }
        }
    });
}

pub async fn process_and_publish_message(
    subject: String, 
    body: String, 
    message_id: Option<String>, 
    user_id: Uuid,
    username: &str, 
    channel_name: &str,
    state: &AppState
) {
    let id = message_id.and_then(|sid| Uuid::parse_str(&sid).ok()).unwrap_or_else(Uuid::new_v4);
    
    let channel_id = {
        let mut ids = state.channel_ids.lock().unwrap();
        *ids.entry(channel_name.to_string()).or_insert_with(Uuid::new_v4)
    };

    let chat_msg = ChatMessage {
        id,
        channel_id,
        user_id,
        user: username.to_string(),
        timestamp: chrono::Utc::now(),
        message: body,
        seq: None,
    };

    let entry = ChannelEntry::Message(chat_msg);

    if let Ok(json_body) = serde_json::to_string(&entry) {
        publish_to_nats(subject, json_body, state).await;
    }
}

async fn publish_to_nats(subject: String, body: String, state: &AppState) {
    let payload = Bytes::from(body);
    if let Err(e) = state.nats_client.publish(subject, payload).await {
        tracing::error!("Failed to publish to NATS: {}", e);
    }
}
