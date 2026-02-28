use futures::StreamExt;
use crate::state::AppState;
use crate::messages::ChatMessage;

/// Starts the PostgreSQL archiver background task.
/// This task subscribes to the NATS JetStream "SANA" stream using a durable consumer
/// and saves every incoming chat message to the database.
pub async fn start(state: AppState) {
    let jetstream = state.jetstream.clone();
    
    // Create a pull consumer with a durable_name to ensure we don't miss messages
    // and share the load across multiple instances if they use the same name.
    let consumer = match jetstream.get_stream("SANA").await.unwrap()
        .get_or_create_consumer(
            "postgres-archiver",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("postgres-archiver".to_string()),
                deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
                ..Default::default()
            }
        )
        .await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Archiver: Failed to create/get consumer: {}", e);
                return;
            }
        };

    let mut messages = match consumer.messages().await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Archiver: Failed to get message stream from consumer: {}", e);
            return;
        }
    };

    tokio::spawn(async move {
        tracing::info!("Archiver: Background task started");
        while let Some(Ok(message)) = messages.next().await {
            handle_message(message, &state).await;
        }
    });
}

async fn handle_message(message: async_nats::jetstream::message::Message, state: &AppState) {
    let subject = message.subject.to_string();
    let Some(encoded_channel_name) = subject.strip_prefix("topic.") else { 
        let _ = message.ack().await;
        return; 
    };

    let channel_name = match crate::nats_util::decode(encoded_channel_name) {
        Some(name) => name,
        None => {
            let _ = message.ack().await;
            return;
        }
    };

    // Don't archive system channel messages 
    if channel_name == "system.channels" {
        let _ = message.ack().await;
        return;
    }

    let payload = String::from_utf8_lossy(&message.payload).to_string();
    let info = match message.info() {
        Ok(info) => info,
        Err(_) => {
            let _ = message.ack().await;
            return;
        }
    };
    let sequence = info.stream_sequence;

    if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&payload) {
        archive_chat_message(channel_name, sequence, chat_msg, message, state).await;
    } else {
        tracing::warn!("Archiver: Failed to parse message, acking and skipping: {}", payload);
        let _ = message.ack().await;
    }
}

async fn archive_chat_message(
    channel_name: String, 
    sequence: u64, 
    chat_msg: ChatMessage, 
    message: async_nats::jetstream::message::Message,
    state: &AppState
) {
    let mut tx = match state.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Archiver: Failed to start transaction: {}", e);
            return; // Do not ack, will be redelivered
        }
    };

    match crate::db::messages::insert_message(&mut tx, &channel_name, sequence, &chat_msg).await {
        Ok(_) => {
            if let Err(e) = tx.commit().await {
                tracing::error!("Archiver: Failed to commit transaction: {}", e);
            } else {
                if let Err(e) = message.ack().await {
                    tracing::error!("Archiver: Failed to ack message {}: {}", sequence, e);
                } else {
                    tracing::debug!("Archiver: Successfully archived message {} on channel {}", sequence, channel_name);
                }
            }
        },
        Err(e) => {
            tracing::error!("Archiver: Failed to insert message into DB: {}", e);
            let _ = tx.rollback().await;
        }
    }
}
