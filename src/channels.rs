use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use crate::state::{AppState, CombinedState};
use crate::db::channels;
use crate::auth::UserSession;

pub fn router() -> Router<CombinedState> {
    Router::new()
        .route("/", get(get_channels))
}

async fn get_channels(
    _session: UserSession,
    State(state): State<AppState>,
) -> Result<Json<Vec<channels::Channel>>, StatusCode> {
    let channels = channels::get_all_channels(&state.db_pool).await
        .map_err(|e| {
            tracing::error!("Failed to fetch channels: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(channels))
}
