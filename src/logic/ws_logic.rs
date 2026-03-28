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
    PublishReadMarker(String, Uuid), // channel_name, message_id
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
                let message_type = headers.iter().find(|(k, _)| k == "message-type").map(|(_, v)| v.as_str());

                if message_type == Some("read_marker") {
                    if let Ok(last_read_id) = Uuid::parse_str(&body) {
                        actions.push(WsAction::PublishReadMarker(channel_name.to_string(), last_read_id));
                    }
                } else {
                    let message_id = headers.iter()
                        .find(|(k, _)| k == "message_id")
                        .map(|(_, v)| v.clone());

                    actions.push(WsAction::PublishToNats(
                        format!("topic.{}", crate::nats_util::encode(channel_name)), 
                        body, 
                        message_id,
                        channel_name.to_string()
                    ));
                }
                
                if let Some((_, receipt_id)) = headers.iter().find(|(k, _)| k == "receipt") {
                    actions.push(WsAction::SendReceipt(receipt_id.clone()));
                }
            }
        }
        StompCommand::Unknown => {}
    }
    actions
}

pub async fn handle_subscribe(channel_name: String, last_seen_seq: Option<u64>, user_id: Uuid, state: &AppState, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    tracing::info!("Subscribing to channel: {}", channel_name);

    if channel_name == "system.channels" {
        handle_system_subscribe(state, tx_internal).await;
        return;
    }

    // 1. Fetch channel ID and last read message ID
    let channel_id = state.channel_ids.get(&channel_name).map(|r| *r.value());
    let mut last_read_message_id = None;

    if let Some(cid) = channel_id {
        if let Ok(mut tx) = state.db_pool.begin().await {
            if let Ok(last_read) = crate::db::messages::get_last_message_read(&mut tx, cid, user_id).await {
                last_read_message_id = last_read;
            }
        }
    }

    // 2. Send Metadata message first
    let metadata_entry = ChannelEntry::Metadata { last_read_message_id };
    send_entry(&channel_name, metadata_entry, tx_internal).await;

    // 3. Subscribe to broadcast FIRST to buffer live messages
    let tx = get_or_create_broadcast_channel(&channel_name, state);
    let live_rx = tx.subscribe();

    // 4. Collect history
    let mut combined_history = Vec::new();
    let mut last_db_seq = 0;

    if let Some(cid) = channel_id {
        if let Ok(mut db_tx) = state.db_pool.begin().await {
            if let Ok(msgs) = crate::db::messages::get_messages(&mut db_tx, cid, 100, None, true).await {
                for msg in msgs {
                    let seq = msg.seq;
                    if let Some(s) = seq { last_db_seq = last_db_seq.max(s); }
                    if should_send(seq, last_seen_seq) {
                        match msg.msg_type {
                            crate::messages::MessageType::Join => {
                                combined_history.push(ChannelEntry::UserJoined {
                                    id: msg.id,
                                    user_id: msg.user_id,
                                    username: msg.user.clone(),
                                    timestamp: msg.timestamp,
                                });
                            }
                            crate::messages::MessageType::Chat => {
                                combined_history.push(ChannelEntry::Message(msg));
                            }
                        }
                    }
                }
            }
        }
    }

    for entry in state.message_store.get_entries(&channel_name) {
        let seq = match &entry {
            ChannelEntry::Message(m) => m.seq,
            _ => None,
        };
        if seq.map_or(true, |s| s > last_db_seq) && should_send(seq, last_seen_seq) {
            combined_history.push(entry);
        }
    }

    // 5. Send history in batches of 20
    for chunk in combined_history.chunks(20) {
        let batch_entry = ChannelEntry::Batch(chunk.to_vec());
        send_entry(&channel_name, batch_entry, tx_internal).await;
    }

    // 6. Start forwarding buffered and future live messages
    spawn_forwarding_task(channel_name, live_rx, tx_internal.clone());
}

async fn handle_system_subscribe(state: &AppState, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    send_initial_channels(state, tx_internal).await;
    let tx = get_or_create_broadcast_channel("system.channels", state);
    spawn_forwarding_task("system.channels".to_string(), tx.subscribe(), tx_internal.clone());
}

fn should_send(seq: Option<u64>, last_seen: Option<u64>) -> bool {
    match (seq, last_seen) {
        (Some(s), Some(last)) => s > last,
        _ => true,
    }
}

async fn send_entry(channel_name: &str, entry: ChannelEntry, tx: &tokio::sync::mpsc::Sender<String>) {
    let Ok(json) = serde_json::to_string(&entry) else { return; };
    let stomp_msg = format_stomp_message(channel_name, &entry, &json);
    let _ = tx.send(stomp_msg).await;
}

fn format_stomp_message(channel_name: &str, entry: &ChannelEntry, json: &str) -> String {
    let seq = match entry {
        ChannelEntry::Message(m) => m.seq,
        _ => None,
    };
    let seq_header = seq.map(|s| format!("seq:{}\n", s)).unwrap_or_default();
    format!("MESSAGE\ndestination:/topic/{}\n{}\n{}\0", channel_name, seq_header, json)
}

fn spawn_forwarding_task(channel_name: String, mut rx: tokio::sync::broadcast::Receiver<String>, tx_internal: tokio::sync::mpsc::Sender<String>) {
    tokio::spawn(async move {
        while let Ok(msg_json) = rx.recv().await {
            let seq_header = if let Ok(entry) = serde_json::from_str::<ChannelEntry>(&msg_json) {
                match entry {
                    ChannelEntry::Message(m) => m.seq.map(|s| format!("seq:{}\n", s)).unwrap_or_default(),
                    _ => String::new(),
                }
            } else {
                String::new()
            };

            let stomp_msg = format!("MESSAGE\ndestination:/topic/{}\n{}\n{}\0", channel_name, seq_header, msg_json);
            if tx_internal.send(stomp_msg).await.is_err() {
                break;
            }
        }
    });
}

async fn send_initial_channels(state: &AppState, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    let channels_list: Vec<String> = state.channels.iter().map(|r| r.key().clone()).collect();
    
    for name in channels_list {
        if name != "system.channels" {
            let stomp_msg = format!("MESSAGE\ndestination:/topic/system.channels\n\n{}\0", name);
            let _ = tx_internal.send(stomp_msg).await;
        }
    }
}

fn get_or_create_broadcast_channel(channel_name: &str, state: &AppState) -> tokio::sync::broadcast::Sender<String> {
    state.channels.entry(channel_name.to_string())
        .or_insert_with(|| {
            let (tx, _rx) = tokio::sync::broadcast::channel(100);
            tx
        })
        .clone()
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
    
    let channel_id = state.channel_ids.get(channel_name).map(|r| *r.value());

    let channel_id = match channel_id {
        Some(id) => id,
        None => {
            // Try to look up in DB
            let mut tx = match state.db_pool.begin().await {
                Ok(tx) => tx,
                Err(e) => {
                    tracing::error!("Failed to start transaction for channel lookup: {}", e);
                    return;
                }
            };
            match crate::db::channels::get_channel_by_name(&mut tx, channel_name).await {
                Ok(Some(c)) => {
                    state.channel_ids.insert(channel_name.to_string(), c.id);
                    c.id
                },
                Ok(None) => {
                    tracing::warn!("Attempted to send message to non-existent channel: {}", channel_name);
                    return;
                },
                Err(e) => {
                    tracing::error!("Failed to look up channel by name: {}", e);
                    return;
                }
            }
        }
    };

    let chat_msg = ChatMessage {
        id,
        channel_id,
        user_id,
        user: username.to_string(),
        timestamp: chrono::Utc::now(),
        message: body,
        seq: None,
        msg_type: crate::messages::MessageType::Chat,
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

pub async fn publish_read_marker(channel_name: &str, user_id: Uuid, message_id: Uuid, state: &AppState) {
    let entry = ChannelEntry::ReadMarker { user_id, message_id };
    tracing::debug!("Publishing ReadMarker to NATS for channel {}: user {} read message {}", channel_name, user_id, message_id);
    if let Ok(json_body) = serde_json::to_string(&entry) {
        let subject = format!("topic.{}", crate::nats_util::encode(channel_name));
        publish_to_nats(subject, json_body, state).await;
    }
}
