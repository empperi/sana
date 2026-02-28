use sqlx::{Postgres, Transaction};
use crate::messages::ChatMessage;

pub async fn insert_message(
    tx: &mut Transaction<'_, Postgres>,
    channel: &str,
    seq: u64,
    msg: &ChatMessage,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO messages (id, channel, seq, username, content, created_at_ms) 
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (id) DO NOTHING"
    )
    .bind(&msg.id)
    .bind(channel)
    .bind(seq as i64)
    .bind(&msg.user)
    .bind(&msg.message)
    .bind(msg.timestamp)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
