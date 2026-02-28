use std::sync::Arc;
use crate::state::AppState;
use crate::stomp::StompCommand;
use bytes::Bytes;
use uuid::Uuid;
use crate::messages::ChatMessage;

#[derive(Debug, PartialEq)]
pub enum WsAction {
    SendToClient(String),
    Subscribe(String), // channel
    PublishToNats(String, String, Option<String>), // subject, body, message_id
    SendReceipt(String), // receipt-id
    None,
}

pub fn decide(command: StompCommand, user_id: &str, username: &str) -> Vec<WsAction> {
    let mut actions = Vec::new();
    match command {
        StompCommand::Connect => {
            let response = format!("CONNECTED
version:1.2
user_id:{}
username:{}

\0", user_id, username);
            actions.push(WsAction::SendToClient(response));
        },
        StompCommand::Subscribe { destination, headers, .. } => {
            if let Some(channel_name) = destination.strip_prefix("/topic/") {
                actions.push(WsAction::Subscribe(channel_name.to_string()));
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

                actions.push(WsAction::PublishToNats(format!("topic.{}", crate::nats_util::encode(channel_name)), body, message_id));
                
                if let Some((_, receipt_id)) = headers.iter().find(|(k, _)| k == "receipt") {
                    actions.push(WsAction::SendReceipt(receipt_id.clone()));
                }
            }
        }
        StompCommand::Unknown => {}
    }
    actions
}

pub async fn handle_subscribe(channel_name: String, state: &Arc<AppState>, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    tracing::info!("Subscribing to channel: {}", channel_name);

    if channel_name == "system.channels" {
        send_initial_channels(state, tx_internal).await;
    }

    // Setup broadcast channel and listener.
    // Since the global nats.rs subscriber now uses DeliverPolicy::All,
    // it will replay all history into this broadcast channel upon backend start.
    // Reconnecting clients will pick up everything they missed.
    let tx = get_or_create_broadcast_channel(&channel_name, state);
    subscribe_to_broadcast(channel_name.clone(), tx, tx_internal).await;
}

async fn send_initial_channels(state: &Arc<AppState>, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    let channels_list: Vec<String> = {
        let channels = state.channels.lock().unwrap();
        channels.keys().cloned().collect()
    };
    
    for name in channels_list {
        if name != "system.channels" {
            let stomp_msg = format!("MESSAGE
destination:/topic/system.channels

{}\0", name);
            let _ = tx_internal.send(stomp_msg).await;
        }
    }
}

fn get_or_create_broadcast_channel(channel_name: &str, state: &Arc<AppState>) -> tokio::sync::broadcast::Sender<String> {
    let mut channels = state.channels.lock().unwrap();
    let is_new = !channels.contains_key(channel_name);
    
    let tx = channels.entry(channel_name.to_string())
        .or_insert_with(|| {
            let (tx, _rx) = tokio::sync::broadcast::channel(100);
            tx
        })
        .clone();

    if is_new && channel_name != "system.channels" {
        publish_new_channel(channel_name, &state.nats_client);
    }
    tx
}

fn publish_new_channel(channel_name: &str, nats_client: &async_nats::Client) {
    let nats_client = nats_client.clone();
    let name = crate::nats_util::encode(channel_name);
    tokio::spawn(async move {
        let _ = nats_client.publish("topic.system.channels", Bytes::from(name)).await;
    });
}

async fn subscribe_to_broadcast(channel_name: String, tx: tokio::sync::broadcast::Sender<String>, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    let mut rx = tx.subscribe();
    let tx_internal = tx_internal.clone();

    tokio::spawn(async move {
        while let Ok(msg_json) = rx.recv().await {
            // Extract seq from JSON if possible to add as header
            let seq_header = if let Ok(msg) = serde_json::from_str::<ChatMessage>(&msg_json) {
                msg.seq.map(|s| format!("seq:{}\n", s)).unwrap_or_default()
            } else {
                "".to_string()
            };

            let stomp_msg = format!("MESSAGE
destination:/topic/{}
{}
{}\0", channel_name, seq_header, msg_json);
            if tx_internal.send(stomp_msg).await.is_err() {
                break;
            }
        }
    });
}

pub async fn process_and_publish_message(subject: String, body: String, message_id: Option<String>, username: &str, state: &Arc<AppState>) {
    let id = message_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let chat_msg = ChatMessage {
        id,
        user: username.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        message: body,
        seq: None,
    };

    if let Ok(json_body) = serde_json::to_string(&chat_msg) {
        publish_to_nats(subject, json_body, state).await;
    }
}

async fn publish_to_nats(subject: String, body: String, state: &Arc<AppState>) {
    let payload = Bytes::from(body);
    if let Err(e) = state.nats_client.publish(subject, payload).await {
        tracing::error!("Failed to publish to NATS: {}", e);
    }
}
