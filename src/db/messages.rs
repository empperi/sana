use sqlx::{Postgres, Transaction};
use crate::messages::ChatMessage;

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
    .await?;

    Ok(())
}
