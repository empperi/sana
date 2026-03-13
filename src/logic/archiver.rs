use futures::StreamExt;
use crate::state::AppState;
use crate::messages::ChatMessage;

/// Starts the PostgreSQL archiver background task.
/// This task subscribes to the NATS JetStream "SANA" stream using a durable consumer
/// and saves every incoming chat message to the database.
pub async fn start(state: AppState) {
    let jetstream = state.jetstream.clone();
    
    let deliver_policy = match jetstream.get_stream("SANA").await {
        Ok(mut stream) => {
            let info = stream.info().await.ok();
            let first_stream_seq = info.map(|i| i.state.first_sequence).unwrap_or(1);
            let last_db_seq = crate::db::messages::get_max_seq(&state.db_pool).await.unwrap_or(None);

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

    tracing::info!("Archiver: Starting with deliver policy: {:?}", deliver_policy);

    // Create a pull consumer with a durable_name to ensure we don't miss messages
    let consumer = match jetstream.get_stream("SANA").await.unwrap()
        .get_or_create_consumer(
            "postgres-archiver",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("postgres-archiver".to_string()),
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
        if let Ok(channel) = serde_json::from_str::<crate::db::channels::Channel>(&payload) {
            archive_channel(channel, message, state).await;
        } else {
            tracing::warn!("Archiver: Failed to parse channel, acking and skipping: {}", payload);
            let _ = message.ack().await;
        }
    } else {
        match serde_json::from_str::<crate::messages::ChannelEntry>(&payload) {
            Ok(crate::messages::ChannelEntry::Message(chat_msg)) => {
                archive_chat_message(sequence, chat_msg, message, state).await;
            },
            Ok(_) => {
                // Skip archiving system notifications (joins, etc)
                let _ = message.ack().await;
            },
            Err(e) => {
                tracing::warn!("Archiver: Failed to parse entry as ChannelEntry: {}. Error: {}", payload, e);
                let _ = message.ack().await;
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
            tracing::error!("Archiver: Failed to insert message into DB: {}", e);
            let _ = tx.rollback().await;
        }
    }
}
