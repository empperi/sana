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

pub fn router() -> Router<CombinedState> {
    Router::new()
        .route("/", get(get_channels))
        .route("/unjoined", get(get_unjoined_channels))
        .route("/join", post(join_channel))
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
    // We need the username. Let's get it from DB. 
    // Optimization: we could store username in UserSession if we wanted.
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
