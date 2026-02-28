use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::sync::Arc;
use crate::state::AppState;
use crate::stomp;
use crate::logic::ws_logic::{self, WsAction};
use rand::Rng;

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
                    let actions = ws_logic::decide(command, &user_id, &username);

                    for action in actions {
                        match action {
                            WsAction::SendToClient(response) => {
                                let _ = tx_internal.send(response).await;
                            }
                            WsAction::Subscribe(channel_name) => {
                                if active_subscriptions.insert(channel_name.clone()) {
                                    ws_logic::handle_subscribe(channel_name, &state, &tx_internal).await;
                                } else {
                                    tracing::debug!("Already subscribed to channel: {}", channel_name);
                                }
                            }
                            WsAction::PublishToNats(subject, body, message_id) => {
                                 ws_logic::process_and_publish_message(subject, body, message_id, &username, &state).await;
                            }
                            WsAction::SendReceipt(receipt_id) => {
                                let response = format!("RECEIPT\nreceipt-id:{}\n\n\0", receipt_id);
                                let _ = tx_internal.send(response).await;
                            }
                            WsAction::None => {}
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
