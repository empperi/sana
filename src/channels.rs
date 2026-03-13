use axum::{
    extract::{State, Query},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use crate::state::{AppState, CombinedState};
use crate::db::channels;
use crate::auth::UserSession;
use serde::Deserialize;
use uuid::Uuid;

use chrono::{DateTime, Utc};

const MAX_MESSAGES_PER_PAGE: i64 = 1000;

pub fn router() -> Router<CombinedState> {
    Router::new()
        .route("/", get(get_channels))
        .route("/", post(create_channel))
        .route("/unjoined", get(get_unjoined_channels))
        .route("/join", post(join_channel))
        .route("/:id/messages", get(get_channel_messages))
}

#[derive(Deserialize)]
struct CreateChannelPayload {
    name: String,
}

#[derive(Deserialize)]
struct SearchQuery {
    #[serde(default)]
    q: String,
}

#[derive(Deserialize)]
struct JoinPayload {
    channel_id: Uuid,
}

#[derive(Deserialize)]
struct MessagesQuery {
    limit: i64,
    before: Option<DateTime<Utc>>,
}

async fn get_channels(
    session: UserSession,
    State(state): State<AppState>,
) -> Result<Json<Vec<channels::Channel>>, StatusCode> {
    let channels = channels::get_user_channels(&state.db_pool, session.user_id).await
        .map_err(|e| {
            tracing::error!("Failed to fetch channels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(channels))
}

async fn create_channel(
    session: UserSession,
    State(state): State<AppState>,
    Json(payload): Json<CreateChannelPayload>,
) -> Result<(StatusCode, Json<channels::Channel>), StatusCode> {
    let mut tx = state.db_pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let channel = channels::Channel {
        id: Uuid::new_v4(),
        name: payload.name,
        is_private: false,
        created_at: chrono::Utc::now(),
    };

    // 1. Insert channel
    channels::insert_channel(&mut tx, &channel).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 2. Join creator
    channels::join_channel(&mut tx, session.user_id, channel.id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 3. Side effect: NATS notification for the new channel
    if let Ok(json) = serde_json::to_string(&channel) {
        let _ = state.nats_client.publish("topic.system.channels", bytes::Bytes::from(json)).await;
    }

    Ok((StatusCode::CREATED, Json(channel)))
}

async fn get_unjoined_channels(
    session: UserSession,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<channels::Channel>>, StatusCode> {
    let channels = channels::search_unjoined_channels(&state.db_pool, session.user_id, &query.q, 10).await
        .map_err(|e| {
            tracing::error!("Failed to search unjoined channels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(channels))
}

async fn join_channel(
    session: UserSession,
    State(state): State<AppState>,
    Json(payload): Json<JoinPayload>,
) -> Result<StatusCode, StatusCode> {
    let mut tx = state.db_pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 1. Get channel name for NATS notification and verification
    let channel = channels::get_channel_by_id(&mut tx, payload.channel_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 2. Perform join in DB
    channels::join_channel(&mut tx, session.user_id, payload.channel_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 3. Side effect: NATS notification
    let mut tx = state.db_pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user = crate::db::users::get_user_by_id(&mut tx, session.user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let subject = format!("topic.{}", crate::nats_util::encode(&channel.name));
    
    let join_event = crate::messages::ChannelEntry::UserJoined {
        id: Uuid::new_v4(),
        username: user.username,
        timestamp: chrono::Utc::now(),
    };

    if let Ok(json) = serde_json::to_string(&join_event) {
        let _ = state.nats_client.publish(subject, bytes::Bytes::from(json)).await;
    }

    Ok(StatusCode::OK)
}

async fn get_channel_messages(
    _session: UserSession,
    State(state): State<AppState>,
    axum::extract::Path(channel_id): axum::extract::Path<Uuid>,
    Query(query): Query<MessagesQuery>,
) -> Result<Json<Vec<crate::messages::ChatMessage>>, StatusCode> {
    if query.limit > MAX_MESSAGES_PER_PAGE {
        return Err(StatusCode::BAD_REQUEST);
    }

    let messages = crate::db::messages::get_messages(&state.db_pool, channel_id, query.limit, query.before, false)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch channel messages: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(messages))
}
