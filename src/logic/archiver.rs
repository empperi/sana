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

    if channel_name == "system.channels" {
        handle_system_channel_message(&payload, message, state).await;
    } else {
        handle_chat_entry_message(&channel_name, &payload, sequence, message, state).await;
    }
}

async fn handle_system_channel_message(payload: &str, message: async_nats::jetstream::message::Message, state: &AppState) {
    if let Ok(channel) = serde_json::from_str::<crate::db::channels::Channel>(payload) {
        archive_channel(channel, message, state).await;
    } else {
        tracing::warn!("Archiver: Failed to parse channel, acking and skipping: {}", payload);
        let _ = message.ack().await;
    }
}

async fn handle_chat_entry_message(channel_name: &str, payload: &str, sequence: u64, message: async_nats::jetstream::message::Message, state: &AppState) {
    match serde_json::from_str::<crate::messages::ChannelEntry>(payload) {
        Ok(crate::messages::ChannelEntry::Message(chat_msg)) => {
            archive_chat_message(sequence, chat_msg, message, state).await;
        },
        Ok(crate::messages::ChannelEntry::UserJoined { id, user_id, username, timestamp }) => {
            archive_join_event(channel_name, sequence, id, user_id, username, timestamp, message, state).await;
        },
        Ok(crate::messages::ChannelEntry::ReadMarker { user_id, message_id }) => {
            archive_read_marker(channel_name, user_id, message_id, message, state).await;
        }
        Ok(_) => {
            let _ = message.ack().await;
        },
        Err(e) => {
            tracing::warn!("Archiver: Failed to parse entry as ChannelEntry: {}. Error: {}", payload, e);
            let _ = message.ack().await;
        }
    }
}

async fn archive_join_event(
    channel_name: &str,
    sequence: u64,
    id: Uuid,
    user_id: Uuid,
    username: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    message: async_nats::jetstream::message::Message,
    state: &AppState
) {
    let result = async {
        let mut tx = state.db_pool.begin().await?;
        
        let channel_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM channels WHERE name = $1")
            .bind(channel_name)
            .fetch_one(&mut *tx)
            .await?;

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

        crate::db::messages::insert_message(&mut tx, sequence, &chat_msg).await?;
        tx.commit().await?;
        Ok::<(), sqlx::Error>(())
    }.await;

    match result {
        Ok(_) => {
            let _ = message.ack().await;
        }
        Err(e) => {
            if is_foreign_key_violation(&e) {
                tracing::warn!("Archiver: Skipping join event for channel '{}' due to missing foreign key: {}", channel_name, e);
                let _ = message.ack().await;
            } else {
                tracing::error!("Archiver: Failed to archive join event: {}", e);
            }
        }
    }
}

async fn archive_channel(
    channel: crate::db::channels::Channel, 
    message: async_nats::jetstream::message::Message,
    state: &AppState
) {
    let mut tx = match state.db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Archiver: Failed to start transaction for channel: {}", e);
            return; 
        }
    };

    match crate::db::channels::insert_channel(&mut tx, &channel).await {
        Ok(_) => {
            if let Err(e) = tx.commit().await {
                tracing::error!("Archiver: Failed to commit channel transaction: {}", e);
            } else {
                let _ = message.ack().await;
                tracing::debug!("Archiver: Successfully archived channel {}", channel.name);
            }
        },
        Err(e) => {
            tracing::error!("Archiver: Failed to insert channel into DB: {}", e);
            let _ = tx.rollback().await;
        }
    }
}

async fn archive_chat_message(
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

    match crate::db::messages::insert_message(&mut tx, sequence, &chat_msg).await {
        Ok(_) => {
            if let Err(e) = tx.commit().await {
                tracing::error!("Archiver: Failed to commit transaction: {}", e);
            } else {
                if let Err(e) = message.ack().await {
                    tracing::error!("Archiver: Failed to ack message {}: {}", sequence, e);
                } else {
                    tracing::debug!("Archiver: Successfully archived message {} on channel {}", sequence, chat_msg.channel_id);
                }
            }
        },
        Err(e) => {
            if is_foreign_key_violation(&e) {
                tracing::warn!("Archiver: Skipping orphan message {} due to missing foreign key (likely from previous session): {}", sequence, e);
                let _ = message.ack().await;
            } else {
                tracing::error!("Archiver: Failed to insert message into DB: {}", e);
                let _ = tx.rollback().await;
            }
        }
    }
}

async fn archive_read_marker(
    channel_name: &str,
    user_id: Uuid,
    message_id: Uuid,
    message: async_nats::jetstream::message::Message,
    state: &AppState
) {
    let result = async {
        let mut tx = state.db_pool.begin().await?;
        crate::db::messages::update_last_message_read_by_name(&mut tx, channel_name, user_id, message_id).await?;
        tx.commit().await?;
        Ok::<(), sqlx::Error>(())
    }.await;

    match result {
        Ok(_) => {
            let _ = message.ack().await;
        }
        Err(e) => {
            if is_foreign_key_violation(&e) {
                // DO NOT ACK read markers on FK violation if they are recent?
                // Actually, if it's an orphan marker for a non-existent channel name, we should ack.
                // But here we know the channel name because it came from the subject.
                
                // Let's check if the message ID exists.
                tracing::warn!("Archiver: Read marker FK violation for channel '{}'. Retrying later.", channel_name);
                // Do not ack, NATS will redeliver.
            } else {
                tracing::error!("Archiver: Failed to update read marker in DB: {}", e);
            }
        }
    }
}

fn is_foreign_key_violation(e: &sqlx::Error) -> bool {
    if let Some(db_err) = e.as_database_error() {
        // Postgres error code 23503 is foreign_key_violation
        return db_err.code().map(|c| c == "23503").unwrap_or(false);
    }
    false
}
