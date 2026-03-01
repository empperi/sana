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
