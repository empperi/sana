use sqlx::{Postgres, Transaction, PgPool, Row};
use crate::messages::ChatMessage;
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub async fn insert_message(
    tx: &mut Transaction<'_, Postgres>,
    seq: u64,
    msg: &ChatMessage,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO messages (id, channel_id, user_id, seq, content, created_at) 
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(msg.id)
    .bind(msg.channel_id)
    .bind(msg.user_id)
    .bind(seq as i64)
    .bind(&msg.message)
    .bind(msg.timestamp)
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert message into DB: id={}, channel_id={}, user_id={}, seq={}. Error: {}", 
            msg.id, msg.channel_id, msg.user_id, seq, e);
        e
    })?;

    Ok(())
}

pub async fn get_max_seq(pool: &PgPool) -> Result<Option<u64>, sqlx::Error> {
    let row: Option<(Option<i64>,)> = sqlx::query_as(
        "SELECT MAX(seq) FROM messages"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|(max_seq,)| max_seq.map(|s| s as u64)))
}

pub async fn get_messages(
    pool: &PgPool,
    channel_id: Uuid,
    limit: i64,
    before: Option<DateTime<Utc>>,
    order_asc: bool,
) -> Result<Vec<ChatMessage>, sqlx::Error> {
    let query_str = if order_asc {
        r#"
        WITH recent AS (
            SELECT 
                m.id, 
                m.channel_id, 
                m.user_id, 
                u.username as user, 
                m.content as message, 
                m.created_at as timestamp, 
                m.seq
            FROM messages m
            JOIN users u ON m.user_id = u.id
            WHERE m.channel_id = $1 
              AND ($2::timestamptz IS NULL OR m.created_at < $2)
            ORDER BY m.created_at DESC
            LIMIT $3
        )
        SELECT * FROM recent ORDER BY timestamp ASC
        "#
    } else {
        r#"
        SELECT 
            m.id, 
            m.channel_id, 
            m.user_id, 
            u.username as user, 
            m.content as message, 
            m.created_at as timestamp, 
            m.seq
        FROM messages m
        JOIN users u ON m.user_id = u.id
        WHERE m.channel_id = $1 
          AND ($2::timestamptz IS NULL OR m.created_at < $2)
        ORDER BY m.created_at DESC
        LIMIT $3
        "#
    };

    let rows = sqlx::query(query_str)
    .bind(channel_id)
    .bind(before)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let messages = rows.into_iter().map(|row| ChatMessage {
        id: row.get("id"),
        channel_id: row.get("channel_id"),
        user_id: row.get("user_id"),
        user: row.get("user"),
        timestamp: row.get("timestamp"),
        message: row.get("message"),
        seq: Some(row.get::<i64, _>("seq") as u64),
    }).collect();

    Ok(messages)
}
