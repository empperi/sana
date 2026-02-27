use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use futures::StreamExt;
use std::env;
use sana::state::AppState;
use sana::ws;
use sana::messages::ChatMessage;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sana=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Connect to NATS
    let nats_url = env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
    let nats_client = async_nats::connect(&nats_url).await.unwrap();

    let app_state = Arc::new(AppState::new(nats_client.clone()));

    // Spawn NATS subscriber task
    let state_clone = app_state.clone();
    let nats_client_clone = nats_client.clone();
    tokio::spawn(async move {
        // Subscribe to all topics under "topic.>"
        let mut subscriber = nats_client_clone.subscribe("topic.>").await.unwrap();
        while let Some(message) = subscriber.next().await {
            let subject = message.subject.to_string();
            if let Some(channel_name) = subject.strip_prefix("topic.") {
                let payload = String::from_utf8_lossy(&message.payload).to_string();
                tracing::debug!("Received from NATS on {}: {}", channel_name, payload);

                if channel_name == "system.channels" {
                    tracing::info!("NATS: Received new channel notification: {}", payload);
                    // System message: just broadcast the channel name to system subscribers
                    let channels = state_clone.channels.lock().unwrap();
                    if let Some(tx) = channels.get(channel_name) {
                        let _ = tx.send(payload);
                    } else {
                        tracing::warn!("NATS: No local subscribers for system.channels");
                    }
                    continue;
                }

                // Store message in memory and only broadcast if successfully stored/parsed
                if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&payload) {
                    state_clone.message_store.add_message(channel_name, chat_msg);

                    // Broadcast to local websocket subscribers
                    let channels = state_clone.channels.lock().unwrap();
                    if let Some(tx) = channels.get(channel_name) {
                        let _ = tx.send(payload);
                    }
                } else {
                    tracing::warn!("Failed to parse message from NATS: {}", payload);
                }
            }
        }
    });

    let app = Router::new()
        .route("/hello", get(hello_world))
        .route("/ws", get(ws::ws_handler))
        .nest_service("/", ServeDir::new("frontend/dist"))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn hello_world() -> &'static str {
    "Hello, World!"
}
