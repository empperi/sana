use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
    http::StatusCode,
};
use axum_extra::extract::SignedCookieJar;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use crate::state::AppState;
use crate::stomp;
use crate::logic::ws_logic::{self, WsAction};
use crate::db::users;
use uuid::Uuid;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    jar: SignedCookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let cookie = match jar.get("session_id") {
        Some(c) => c,
        None => {
            tracing::warn!("WebSocket: Session cookie missing or invalid signature");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    let user_id_str = cookie.value();
    let user_id = match Uuid::parse_str(user_id_str) {
        Ok(id) => id,
        Err(_) => {
            tracing::warn!("WebSocket: Invalid user_id in session cookie: {}", user_id_str);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let mut tx = state.db_pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user = match users::get_user_by_id(&mut tx, user_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        Some(u) => u,
        None => {
            tracing::warn!("WebSocket: User not found for session user_id: {}", user_id);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user.id, user.username)))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid, username: String) {
    let (mut sender, mut receiver) = socket.split();
    let (tx_internal, mut rx_internal) = tokio::sync::mpsc::channel::<String>(100);

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
                    let actions = ws_logic::decide(command, user_id, &username);

                    for action in actions {
                        match action {
                            WsAction::SendToClient(response) => {
                                let _ = tx_internal.send(response).await;
                            }
                            WsAction::Subscribe(channel_name, last_seen_seq) => {
                                if active_subscriptions.insert(channel_name.clone()) {
                                    ws_logic::handle_subscribe(channel_name, last_seen_seq, &state, &tx_internal).await;
                                } else {
                                    tracing::debug!("Already subscribed to channel: {}", channel_name);
                                }
                            }
                            WsAction::PublishToNats(subject, body, message_id, channel_name) => {
                                 ws_logic::process_and_publish_message(subject, body, message_id, user_id, &username, &channel_name, &state).await;
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
