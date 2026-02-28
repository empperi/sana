use futures::StreamExt;
use crate::state::AppState;
use crate::messages::ChatMessage;

pub async fn start_nats_subscriber(state: AppState) {
    let jetstream = state.jetstream.clone();
    
    // Use an ephemeral ordered consumer to get ALL messages from the stream
    // This ensures that even if the backend restarts, it will catch up and broadcast everything.
    // Frontend idempotency will handle duplicates.
    let mut messages = jetstream.get_stream("SANA").await.unwrap()
        .create_consumer(async_nats::jetstream::consumer::pull::OrderedConfig {
            deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
            ..Default::default()
        })
        .await
        .unwrap()
        .messages()
        .await
        .unwrap();

    tokio::spawn(async move {
        while let Some(Ok(message)) = messages.next().await {
            handle_nats_message(message, &state).await;
        }
    });
}

pub async fn start_postgres_archiver(state: AppState) {
    let jetstream = state.jetstream.clone();
    
    // Create a pull consumer with a deliver_group for distributed processing
    let consumer = jetstream.get_stream("SANA").await.unwrap()
        .get_or_create_consumer(
            "postgres-archiver",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("postgres-archiver".to_string()),
                deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
                ..Default::default()
            }
        )
        .await
        .unwrap();

    let mut messages = consumer.messages().await.unwrap();

    tokio::spawn(async move {
        while let Some(Ok(message)) = messages.next().await {
            let subject = message.subject.to_string();
            let Some(encoded_channel_name) = subject.strip_prefix("topic.") else { 
                let _ = message.ack().await;
                continue; 
            };

            let channel_name = match crate::nats_util::decode(encoded_channel_name) {
                Some(name) => name,
                None => {
                    let _ = message.ack().await;
                    continue;
                }
            };

            // Don't archive system channel messages 
            if channel_name == "system.channels" {
                let _ = message.ack().await;
                continue;
            }

            let payload = String::from_utf8_lossy(&message.payload).to_string();
            let info = match message.info() {
                Ok(info) => info,
                Err(_) => {
                    let _ = message.ack().await;
                    continue;
                }
            };
            let sequence = info.stream_sequence;

            if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&payload) {
                let mut tx = match state.db_pool.begin().await {
                    Ok(tx) => tx,
                    Err(e) => {
                        tracing::error!("Archiver: Failed to start transaction: {}", e);
                        continue; // Do not ack, will be redelivered
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
            } else {
                tracing::warn!("Archiver: Failed to parse message, acking and skipping: {}", payload);
                let _ = message.ack().await;
            }
        }
    });
}

async fn handle_nats_message(message: async_nats::jetstream::message::Message, state: &AppState) {
    let subject = message.subject.to_string();
    let Some(encoded_channel_name) = subject.strip_prefix("topic.") else { return; };

    let channel_name = match crate::nats_util::decode(encoded_channel_name) {
        Some(name) => name,
        None if encoded_channel_name == "system.channels" => encoded_channel_name.to_string(),
        None => return,
    };

    let payload = String::from_utf8_lossy(&message.payload).to_string();
    let info = message.info().expect("Failed to get message info");
    let sequence = info.stream_sequence;
    
    tracing::debug!("Received from NATS on {} (seq {}): {}", channel_name, sequence, payload);

    if channel_name == "system.channels" {
        handle_system_channels_message(payload, state).await;
    } else {
        handle_chat_message(channel_name, payload, sequence, state).await;
    }
}

async fn handle_system_channels_message(payload: String, state: &AppState) {
    let channel_name = crate::nats_util::decode(&payload).unwrap_or(payload);
    tracing::info!("NATS: Received new channel notification: {}", channel_name);

    let channels = state.channels.lock().unwrap();
    if let Some(tx) = channels.get("system.channels") {
        let _ = tx.send(channel_name);
    }
}

async fn handle_chat_message(channel_name: String, payload: String, sequence: u64, state: &AppState) {
    if let Ok(mut chat_msg) = serde_json::from_str::<ChatMessage>(&payload) {
        chat_msg.seq = Some(sequence);
        state.message_store.add_message(&channel_name, chat_msg.clone());

        if let Ok(final_payload) = serde_json::to_string(&chat_msg) {
            let channels = state.channels.lock().unwrap();
            if let Some(tx) = channels.get(&channel_name) {
                let _ = tx.send(final_payload);
            }
        }
    } else {
        tracing::warn!("Failed to parse message from NATS: {}", payload);
    }
}
