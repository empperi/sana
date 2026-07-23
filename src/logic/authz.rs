use sqlx::PgPool;
use uuid::Uuid;
use crate::state::AppState;
use crate::db::channels;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum AuthzError {
    NotAMember,
    ChannelNotFound,
    DbError(String),
}

impl fmt::Display for AuthzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthzError::NotAMember => write!(f, "User is not a member of the channel"),
            AuthzError::ChannelNotFound => write!(f, "Channel not found"),
            AuthzError::DbError(err) => write!(f, "Database error: {}", err),
        }
    }
}

impl std::error::Error for AuthzError {}

pub async fn ensure_channel_member(
    pool: &PgPool,
    user_id: Uuid,
    channel_id: Uuid,
) -> Result<(), AuthzError> {
    let mut tx = pool.begin().await.map_err(|e| AuthzError::DbError(e.to_string()))?;

    let channel_exists = channels::get_channel_by_id(&mut tx, channel_id)
        .await
        .map_err(|e| AuthzError::DbError(e.to_string()))?
        .is_some();

    if !channel_exists {
        return Err(AuthzError::ChannelNotFound);
    }

    let is_member = channels::is_channel_member(&mut tx, user_id, channel_id)
        .await
        .map_err(|e| AuthzError::DbError(e.to_string()))?;

    tx.commit().await.map_err(|e| AuthzError::DbError(e.to_string()))?;

    if is_member {
        Ok(())
    } else {
        Err(AuthzError::NotAMember)
    }
}

pub async fn ensure_channel_member_by_name(
    state: &AppState,
    user_id: Uuid,
    channel_name: &str,
) -> Result<(), AuthzError> {
    if channel_name == "system.channels" {
        return Ok(());
    }

    let cached_id = state.channel_ids.get(channel_name).map(|r| *r.value());
    if let Some(channel_id) = cached_id {
        return ensure_channel_member(&state.db_pool, user_id, channel_id).await;
    }

    // Fallback to DB lookup if not in state cache
    let mut tx = state.db_pool.begin().await.map_err(|e| AuthzError::DbError(e.to_string()))?;
    let channel = channels::get_channel_by_name(&mut tx, channel_name)
        .await
        .map_err(|e| AuthzError::DbError(e.to_string()))?;
    tx.commit().await.map_err(|e| AuthzError::DbError(e.to_string()))?;

    let channel = match channel {
        Some(c) => c,
        None => return Err(AuthzError::ChannelNotFound),
    };

    state.channel_ids.insert(channel.name.clone(), channel.id);
    ensure_channel_member(&state.db_pool, user_id, channel.id).await
}
