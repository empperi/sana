use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use crate::state::AppState;
use bytes::Bytes;
use crate::stomp::{self, StompCommand};
use serde::{Deserialize, Serialize};
use rand::Rng;
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum WsAction {
    SendToClient(String),
    Subscribe(String),
    PublishToNats(String, String, Option<String>), // subject, body, message_id
    None,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    pub id: String,
    pub user: String,
    pub timestamp: i64,
    pub message: String,
}

pub fn decide(command: StompCommand, user_id: &str, username: &str) -> WsAction {
    match command {
        StompCommand::Connect => {
            let response = format!("CONNECTED\nversion:1.2\nuser_id:{}\nusername:{}\n\n\0", user_id, username);
            WsAction::SendToClient(response)
        },
        StompCommand::Subscribe { destination } => {
            if let Some(channel_name) = destination.strip_prefix("/topic/") {
                WsAction::Subscribe(channel_name.to_string())
            } else {
                WsAction::None
            }
        }
        StompCommand::Send { destination, body, headers } => {
            if let Some(channel_name) = destination.strip_prefix("/topic/") {
                let message_id = headers.iter()
                    .find(|(k, _)| k == "message_id")
                    .map(|(_, v)| v.clone());

                WsAction::PublishToNats(format!("topic.{}", channel_name), body, message_id)
            } else {
                WsAction::None
            }
        }
        StompCommand::Unknown => WsAction::None,
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx_internal, mut rx_internal) = tokio::sync::mpsc::channel::<String>(100);

    // Generate a random user_id and username for this session
    let (user_id, username) = {
        let mut rng = rand::thread_rng();
        let id = rng.gen_range(1000..9999);
        (id.to_string(), format!("User{}", id))
    };

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx_internal.recv().await {
             if sender.send(Message::Text(msg)).await.is_err() {
                 break;
             }
        }
    });

    let mut active_subscriptions = std::collections::HashSet::new();

    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    let command = stomp::parse(&text);
                    let action = decide(command, &user_id, &username);

                    match action {
                        WsAction::SendToClient(response) => {
                            let _ = tx_internal.send(response).await;
                        }
                        WsAction::Subscribe(channel_name) => {
                            if active_subscriptions.insert(channel_name.clone()) {
                                handle_subscribe(channel_name, &state, &tx_internal).await;
                            } else {
                                tracing::debug!("Already subscribed to channel: {}", channel_name);
                            }
                        }
                        WsAction::PublishToNats(subject, body, message_id) => {
                             process_and_publish_message(subject, body, message_id, &username, &state).await;
                        }
                        WsAction::None => {
                            tracing::debug!("Unknown or unsupported STOMP command");
                        }
                    }
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        } else {
            break;
        }
    }

    send_task.abort();
}

async fn handle_subscribe(channel_name: String, state: &Arc<AppState>, tx_internal: &tokio::sync::mpsc::Sender<String>) {
    tracing::info!("Subscribing to channel: {}", channel_name);

    if channel_name == "system.channels" {
        let channels_list: Vec<String> = {
            let channels = state.channels.lock().unwrap();
            channels.keys().cloned().collect()
        };
        tracing::info!("Sending initial channels list: {:?}", channels_list);
        for name in channels_list {
            if name != "system.channels" {
                let stomp_msg = format!("MESSAGE\ndestination:/topic/system.channels\n\n{}\0", name);
                let _ = tx_internal.send(stomp_msg).await;
            }
        }
    }

    let tx = {
        let mut channels = state.channels.lock().unwrap();
        let is_new = !channels.contains_key(&channel_name);
        
        let tx = channels.entry(channel_name.clone())
            .or_insert_with(|| {
                tracing::info!("Creating new broadcast channel for: {}", channel_name);
                let (tx, _rx) = tokio::sync::broadcast::channel(100);
                tx
            })
            .clone();

        if is_new && channel_name != "system.channels" {
            tracing::info!("Publishing new channel to NATS: {}", channel_name);
            let nats_client = state.nats_client.clone();
            let name = channel_name.clone();
            tokio::spawn(async move {
                let _ = nats_client.publish("topic.system.channels", Bytes::from(name)).await;
            });
        }
        tx
    };

    let mut rx = tx.subscribe();
    let tx_internal_clone = tx_internal.clone();
    let channel_name_clone = channel_name.clone();

    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let stomp_msg = format!("MESSAGE\ndestination:/topic/{}\n\n{}\0", channel_name_clone, msg);
            if tx_internal_clone.send(stomp_msg).await.is_err() {
                break;
            }
        }
    });
}

async fn process_and_publish_message(subject: String, body: String, message_id: Option<String>, username: &str, state: &Arc<AppState>) {
    // Enrich the message with user and timestamp
    let id = message_id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let chat_msg = ChatMessage {
        id,
        user: username.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        message: body,
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
