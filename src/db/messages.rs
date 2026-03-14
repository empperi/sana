use sqlx::{Postgres, Transaction, Row};
use crate::messages::ChatMessage;
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub async fn insert_message(
    tx: &mut Transaction<'_, Postgres>,
    seq: u64,
    msg: &ChatMessage,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO messages (id, channel_id, user_id, seq, content, created_at, msg_type) 
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(msg.id)
    .bind(msg.channel_id)
    .bind(msg.user_id)
    .bind(seq as i64)
    .bind(&msg.message)
    .bind(msg.timestamp)
    .bind(&msg.msg_type)
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert message into DB: id={}, channel_id={}, user_id={}, seq={}. Error: {}", 
            msg.id, msg.channel_id, msg.user_id, seq, e);
        e
    })?;

    Ok(())
}

pub async fn get_max_seq(tx: &mut Transaction<'_, Postgres>) -> Result<Option<u64>, sqlx::Error> {
    let row: Option<(Option<i64>,)> = sqlx::query_as(
        "SELECT MAX(seq) FROM messages"
    )
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.and_then(|(max_seq,)| max_seq.map(|s| s as u64)))
}

pub async fn get_messages(
    tx: &mut Transaction<'_, Postgres>,
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
                m.seq,
                m.msg_type
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
            m.seq,
            m.msg_type
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
    .fetch_all(&mut **tx)
    .await?;

    // In our domain logic, ChatMessage represents user chat. 
    // Join events are technically stored as regular messages ("<username> joined") 
    // but they shouldn't show the user's name as the sender, they should just show as a system message.
    // However, get_messages returns Vec<ChatMessage>, not Vec<ChannelEntry>.
    // To cleanly separate them on the frontend, the history loading API would ideally return ChannelEntry.
    // Let's keep returning ChatMessage but we can differentiate them by content if needed.
    
    let messages = rows.into_iter().map(|row| ChatMessage {
        id: row.get("id"),
        channel_id: row.get("channel_id"),
        user_id: row.get("user_id"),
        user: row.get("user"),
        timestamp: row.get("timestamp"),
        message: row.get("message"),
        seq: Some(row.get::<i64, _>("seq") as u64),
        msg_type: row.get("msg_type"),
    }).collect();

    Ok(messages)
}

pub async fn update_last_message_read(
    tx: &mut Transaction<'_, Postgres>,
    channel_id: Uuid,
    user_id: Uuid,
    message_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE user_channels 
         SET last_message_read = $1 
         WHERE channel_id = $2 AND user_id = $3"
    )
    .bind(message_id)
    .bind(channel_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn update_last_message_read_by_name(
    tx: &mut Transaction<'_, Postgres>,
    channel_name: &str,
    user_id: Uuid,
    message_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE user_channels 
         SET last_message_read = $1 
         WHERE user_id = $2 
           AND channel_id = (SELECT id FROM channels WHERE name = $3 LIMIT 1)"
    )
    .bind(message_id)
    .bind(user_id)
    .bind(channel_name)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn get_last_message_read(
    tx: &mut Transaction<'_, Postgres>,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    let row: Option<(Option<Uuid>,)> = sqlx::query_as(
        "SELECT last_message_read FROM user_channels WHERE channel_id = $1 AND user_id = $2"
    )
    .bind(channel_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.and_then(|(r,)| r))
}
