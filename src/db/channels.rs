use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub is_private: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn insert_channel(
    tx: &mut Transaction<'_, Postgres>, 
    channel: &Channel
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO channels (id, name, is_private, created_at) 
         VALUES ($1, $2, $3, $4) 
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(channel.id)
    .bind(&channel.name)
    .bind(channel.is_private)
    .bind(channel.created_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn get_channel_by_name(
    tx: &mut Transaction<'_, Postgres>, 
    name: &str
) -> Result<Option<Channel>, sqlx::Error> {
    let channel = sqlx::query_as::<_, Channel>(
        "SELECT id, name, is_private, created_at FROM channels WHERE name = $1"
    )
    .bind(name)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(channel)
}

pub async fn get_channel_by_id(
    tx: &mut Transaction<'_, Postgres>, 
    id: Uuid
) -> Result<Option<Channel>, sqlx::Error> {
    let channel = sqlx::query_as::<_, Channel>(
        "SELECT id, name, is_private, created_at FROM channels WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(channel)
}

pub async fn get_all_channels(
    pool: &sqlx::PgPool
) -> Result<Vec<Channel>, sqlx::Error> {
    let channels = sqlx::query_as::<_, Channel>(
        "SELECT id, name, is_private, created_at FROM channels ORDER BY name ASC"
    )
    .fetch_all(pool)
    .await?;

    Ok(channels)
}

pub async fn join_channel(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    channel_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_channels (user_id, channel_id) 
         VALUES ($1, $2) 
         ON CONFLICT DO NOTHING"
    )
    .bind(user_id)
    .bind(channel_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn get_user_channels(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<Vec<Channel>, sqlx::Error> {
    let channels = sqlx::query_as::<_, Channel>(
        "SELECT c.id, c.name, c.is_private, c.created_at 
         FROM channels c
         JOIN user_channels cj ON c.id = cj.channel_id
         WHERE cj.user_id = $1
         ORDER BY c.name ASC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(channels)
}

pub async fn is_channel_member(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    channel_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "SELECT 1 FROM user_channels WHERE user_id = $1 AND channel_id = $2"
    )
    .bind(user_id)
    .bind(channel_id)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(result.is_some())
}

pub async fn search_unjoined_channels(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    query: &str,
    limit: i64,
) -> Result<Vec<Channel>, sqlx::Error> {
    let search_pattern = format!("%{}%", query);
    let channels = sqlx::query_as::<_, Channel>(
        "SELECT id, name, is_private, created_at 
         FROM channels 
         WHERE id NOT IN (SELECT channel_id FROM user_channels WHERE user_id = $1)
         AND name ILIKE $2
         ORDER BY name ASC
         LIMIT $3"
    )
    .bind(user_id)
    .bind(search_pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(channels)
}
