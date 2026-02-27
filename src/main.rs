use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use futures::StreamExt;
use sana::state::AppState;
use sana::ws;
use sana::messages::ChatMessage;
use sana::config::Config;
use sana::db;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    // Load .env if present
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sana=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::new();

    // Connect to NATS
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();

    // Connect to Database
    let db_pool = db::connect(&config).await.expect("Failed to connect to database");
    if db::check_connection(&db_pool).await.unwrap_or(false) {
        tracing::info!("Successfully connected to database");
    }

    let app_state = Arc::new(AppState::new(nats_client.clone()));

    // Spawn NATS subscriber task
    let state_clone = app_state.clone();
    let nats_client_clone = nats_client.clone();
    tokio::spawn(async move {
        // Subscribe to all topics under "topic.>"
        let mut subscriber = nats_client_clone.subscribe("topic.>").await.unwrap();
        while let Some(message) = subscriber.next().await {
            let subject = message.subject.to_string();
            if let Some(encoded_channel_name) = subject.strip_prefix("topic.") {
                let channel_name = match sana::nats_util::decode(encoded_channel_name) {
                    Some(name) => name,
                    None => {
                        // Compatibility for non-encoded subjects (like system.channels if we decide so)
                        // but actually we decided to encode everything.
                        // Let's try to see if it's "system.channels" directly or encoded.
                        if encoded_channel_name == "system.channels" {
                            encoded_channel_name.to_string()
                        } else {
                            continue;
                        }
                    }
                };

                let payload = String::from_utf8_lossy(&message.payload).to_string();
                tracing::debug!("Received from NATS on {}: {}", channel_name, payload);

                if channel_name == "system.channels" {
                    let payload_channel_name = match sana::nats_util::decode(&payload) {
                        Some(name) => name,
                        None => payload, // Fallback
                    };
                    tracing::info!("NATS: Received new channel notification: {}", payload_channel_name);
                    // System message: just broadcast the channel name to system subscribers
                    let channels = state_clone.channels.lock().unwrap();
                    if let Some(tx) = channels.get("system.channels") {
                        let _ = tx.send(payload_channel_name);
                    } else {
                        tracing::warn!("NATS: No local subscribers for system.channels");
                    }
                    continue;
                }

                // Store message in memory and only broadcast if successfully stored/parsed
                if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&payload) {
                    state_clone.message_store.add_message(&channel_name, chat_msg);

                    // Broadcast to local websocket subscribers
                    let channels = state_clone.channels.lock().unwrap();
                    if let Some(tx) = channels.get(&channel_name) {
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
