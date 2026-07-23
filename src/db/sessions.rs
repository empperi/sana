use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub async fn create_session(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<Session, sqlx::Error> {
    let session = sqlx::query_as::<_, Session>(
        "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2) RETURNING id, user_id, created_at, expires_at"
    )
    .bind(user_id)
    .bind(expires_at)
    .fetch_one(&mut **tx)
    .await?;

    Ok(session)
}

pub async fn get_valid_session(
    tx: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
) -> Result<Option<Session>, sqlx::Error> {
    let session = sqlx::query_as::<_, Session>(
        "SELECT id, user_id, created_at, expires_at FROM sessions WHERE id = $1"
    )
    .bind(session_id)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(session) = session {
        if session.expires_at <= Utc::now() {
            sqlx::query("DELETE FROM sessions WHERE id = $1")
                .bind(session_id)
                .execute(&mut **tx)
                .await?;
            return Ok(None);
        }
        Ok(Some(session))
    } else {
        Ok(None)
    }
}

pub async fn delete_session(
    tx: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}
