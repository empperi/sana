use futures::StreamExt;
use crate::state::AppState;

pub async fn start_nats_subscriber(state: AppState) {
    let jetstream = state.jetstream.clone();
    
    // Use an ephemeral ordered consumer to get ONLY NEW messages from the stream.
    // History is served from the database and bridging in-memory store.
    let mut messages = jetstream.get_stream("SANA").await.unwrap()
        .create_consumer(async_nats::jetstream::consumer::pull::OrderedConfig {
            deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::New,
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
    if let Ok(channel) = serde_json::from_str::<crate::db::channels::Channel>(&payload) {
        tracing::info!("NATS: Received new channel notification: {}", channel.name);

        state.channel_ids.insert(channel.name.clone(), channel.id);

        if let Some(tx) = state.channels.get("system.channels") {
            let _ = tx.send(payload);
        }
    } else {
        tracing::warn!("NATS: Failed to parse channel from system.channels: {}", payload);
    }
}

async fn handle_chat_message(channel_name: String, payload: String, sequence: u64, state: &AppState) {
    if let Ok(mut entry) = serde_json::from_str::<crate::messages::ChannelEntry>(&payload) {
        if let crate::messages::ChannelEntry::Message(ref mut chat_msg) = entry {
            chat_msg.seq = Some(sequence);
        }

        if let crate::messages::ChannelEntry::ReadMarker { user_id, message_id } = &entry {
            tracing::debug!("NATS: Received ReadMarker for channel {}: user {} read message {}", channel_name, user_id, message_id);
        }

        state.message_store.add_entry(&channel_name, entry.clone());

        if let Ok(final_payload) = serde_json::to_string(&entry) {
            if let Some(tx) = state.channels.get(&channel_name) {
                let _ = tx.send(final_payload);
            }
        }
    } else {
        tracing::warn!("Failed to parse entry from NATS: {}", payload);
    }
}
