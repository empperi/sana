use futures::StreamExt;
use crate::state::AppState;
use crate::messages::ChatMessage;
use uuid::Uuid;

/// Starts the PostgreSQL archiver background task.
/// This task subscribes to the NATS JetStream "SANA" stream using a durable consumer
/// and saves every incoming chat message to the database.
pub async fn start(state: AppState) {
    start_with_durable(state, "postgres-archiver".to_string()).await;
}

pub async fn start_with_durable(state: AppState, durable_name: String) {
    let jetstream = state.jetstream.clone();
    
    let deliver_policy = match jetstream.get_stream("SANA").await {
        Ok(mut stream) => {
            let info = stream.info().await.ok();
            let first_stream_seq = info.map(|i| i.state.first_sequence).unwrap_or(1);
            
            let tx = state.db_pool.begin().await.ok();
            let last_db_seq = if let Some(mut t) = tx {
                crate::db::messages::get_max_seq(&mut t).await.unwrap_or(None)
            } else {
                None
            };

            match last_db_seq {
                Some(db_seq) if db_seq + 1 >= first_stream_seq => {
                    async_nats::jetstream::consumer::DeliverPolicy::ByStartSequence { start_sequence: db_seq + 1 }
                },
                _ => async_nats::jetstream::consumer::DeliverPolicy::All
            }
        },
        Err(e) => {
            tracing::error!("Archiver: Failed to get stream: {}", e);
            async_nats::jetstream::consumer::DeliverPolicy::All
        }
    };

    tracing::info!("Archiver: Starting with durable '{}' and deliver policy: {:?}", durable_name, deliver_policy);

    // Create a pull consumer with a durable_name to ensure we don't miss messages
    let consumer = match jetstream.get_stream("SANA").await.unwrap()
        .get_or_create_consumer(
            &durable_name,
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some(durable_name.clone()),
                deliver_policy,
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
        tracing::warn!("Archiver: Received message with invalid subject: {}", subject);
        let _ = message.ack().await;
        return; 
    };

    let channel_name = match crate::nats_util::decode(encoded_channel_name) {
        Some(name) => name,
        None if encoded_channel_name == "system.channels" => encoded_channel_name.to_string(),
        None => {
            tracing::warn!("Archiver: Failed to decode channel name from subject: {}", encoded_channel_name);
            let _ = message.ack().await;
            return;
        }
    };

    let payload = String::from_utf8_lossy(&message.payload).to_string();
    let info = match message.info() {
        Ok(info) => info,
        Err(e) => {
            tracing::error!("Archiver: Failed to get message info: {}", e);
            let _ = message.ack().await;
            return;
        }
    };
    let sequence = info.stream_sequence;

    let result = if channel_name == "system.channels" {
        handle_system_channel_message(&payload, state).await
    } else {
        handle_chat_entry_message(&channel_name, &payload, sequence, state).await
    };

    match result {
        Ok(_) => {
            let _ = message.ack().await;
        }
        Err(e) => {
            tracing::error!("Archiver: Permanent failure processing message {}: {}. Acking to avoid poison message.", sequence, e);
            let _ = message.ack().await;
        }
    }
}

async fn handle_system_channel_message(payload: &str, state: &AppState) -> Result<(), String> {
    let channel = serde_json::from_str::<crate::db::channels::Channel>(payload)
        .map_err(|e| format!("Failed to parse channel: {}", e))?;
    
    let mut tx = state.db_pool.begin().await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    crate::db::channels::insert_channel(&mut tx, &channel).await
        .map_err(|e| format!("Failed to insert channel into DB: {}", e))?;

    tx.commit().await
        .map_err(|e| format!("Failed to commit channel transaction: {}", e))?;

    tracing::debug!("Archiver: Successfully archived channel {}", channel.name);
    Ok(())
}

async fn handle_chat_entry_message(channel_name: &str, payload: &str, sequence: u64, state: &AppState) -> Result<(), String> {
    let entry = serde_json::from_str::<crate::messages::ChannelEntry>(payload)
        .map_err(|e| format!("Failed to parse ChannelEntry: {}", e))?;

    match entry {
        crate::messages::ChannelEntry::Message(chat_msg) => {
            archive_chat_message(sequence, chat_msg, state).await
        },
        crate::messages::ChannelEntry::UserJoined { id, user_id, username, timestamp } => {
            archive_join_event(channel_name, sequence, id, user_id, username, timestamp, state).await
        },
        crate::messages::ChannelEntry::ReadMarker { user_id, message_id } => {
            archive_read_marker(channel_name, user_id, message_id, state).await
        }
        _ => Ok(()),
    }
}

async fn archive_join_event(
    channel_name: &str,
    sequence: u64,
    id: Uuid,
    user_id: Uuid,
    username: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    state: &AppState
) -> Result<(), String> {
    let mut tx = state.db_pool.begin().await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    
    let channel_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM channels WHERE name = $1")
        .bind(channel_name)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Failed to resolve channel ID: {}", e))?;

    let chat_msg = ChatMessage {
        id,
        channel_id,
        user_id,
        user: username.clone(),
        timestamp,
        message: format!("{} joined", username),
        seq: Some(sequence),
        msg_type: crate::messages::MessageType::Join,
    };

    crate::db::messages::insert_message_with_fk_check(&mut tx, sequence, &chat_msg).await
        .map_err(|e| format!("Failed to archive join event: {}", e))?;

    tx.commit().await
        .map_err(|e| format!("Failed to commit join event transaction: {}", e))?;

    Ok(())
}

async fn archive_chat_message(
    sequence: u64, 
    chat_msg: ChatMessage, 
    state: &AppState
) -> Result<(), String> {
    let mut tx = state.db_pool.begin().await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    crate::db::messages::insert_message_with_fk_check(&mut tx, sequence, &chat_msg).await
        .map_err(|e| format!("Failed to insert message: {}", e))?;

    tx.commit().await
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    tracing::debug!("Archiver: Successfully archived message {} on channel {}", sequence, chat_msg.channel_id);
    Ok(())
}

async fn archive_read_marker(
    channel_name: &str,
    user_id: Uuid,
    message_id: Uuid,
    state: &AppState
) -> Result<(), String> {
    let mut tx = state.db_pool.begin().await
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    crate::db::messages::update_last_message_read_by_name(&mut tx, channel_name, user_id, message_id).await
        .map_err(|e| format!("Failed to update read marker in DB: {}", e))?;

    tx.commit().await
        .map_err(|e| format!("Failed to commit read marker transaction: {}", e))?;

    Ok(())
}
