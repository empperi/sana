use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, Duration};
use crate::state::AppState;
use crate::db::sessions as db_sessions;

pub const SESSION_LIFETIME: Duration = Duration::days(30);

pub async fn start_session(pool: &PgPool, user_id: Uuid) -> Result<Uuid, sqlx::Error> {
    let expires_at = Utc::now() + SESSION_LIFETIME;
    let mut tx = pool.begin().await?;
    let session = db_sessions::create_session(&mut tx, user_id, expires_at).await?;
    tx.commit().await?;
    Ok(session.id)
}

pub async fn validate(state: &AppState, session_id: Uuid) -> Option<Uuid> {
    // 1. Check cache
    if let Some(entry) = state.session_cache.get(&session_id) {
        let (user_id, cached_at) = *entry;
        if Utc::now() - cached_at < Duration::seconds(60) {
            return Some(user_id);
        }
        // Evict stale entry (> 60s) before DB lookup
        drop(entry);
        state.session_cache.remove(&session_id);
    }

    // 2. Check DB
    let mut tx = match state.db_pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return None,
    };

    match db_sessions::get_valid_session(&mut tx, session_id).await {
        Ok(Some(session)) => {
            let _ = tx.commit().await;
            state.session_cache.insert(session_id, (session.user_id, Utc::now()));
            Some(session.user_id)
        }
        Ok(None) => {
            // Commit transaction so lazy delete of expired session is persisted
            let _ = tx.commit().await;
            state.session_cache.remove(&session_id);
            None
        }
        Err(_) => {
            state.session_cache.remove(&session_id);
            None
        }
    }
}

pub async fn end_session(state: &AppState, session_id: Uuid) -> Result<(), sqlx::Error> {
    state.session_cache.remove(&session_id);
    let mut tx = state.db_pool.begin().await?;
    db_sessions::delete_session(&mut tx, session_id).await?;
    tx.commit().await?;
    Ok(())
}
