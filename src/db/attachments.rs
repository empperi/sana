use sqlx::{Postgres, Transaction, Row};
use uuid::Uuid;
use crate::messages::AttachmentMeta;

pub async fn insert_attachment(
    tx: &mut Transaction<'_, Postgres>,
    original_filename: &str,
    stored_filename: &str,
    file_size: i64,
    mime_type: &str,
    uploaded_by: Uuid,
) -> Result<AttachmentMeta, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO attachments (original_filename, stored_filename, file_size, mime_type, uploaded_by) 
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, original_filename, file_size, mime_type"
    )
    .bind(original_filename)
    .bind(stored_filename)
    .bind(file_size)
    .bind(mime_type)
    .bind(uploaded_by)
    .fetch_one(&mut **tx)
    .await?;

    Ok(AttachmentMeta {
        id: row.get("id"),
        original_filename: row.get("original_filename"),
        file_size: row.get("file_size"),
        mime_type: row.get("mime_type"),
    })
}

pub async fn get_attachment_by_id(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<AttachmentMeta, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, original_filename, file_size, mime_type 
         FROM attachments 
         WHERE id = $1"
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(AttachmentMeta {
        id: row.get("id"),
        original_filename: row.get("original_filename"),
        file_size: row.get("file_size"),
        mime_type: row.get("mime_type"),
    })
}

pub async fn get_attachments_by_message_id(
    tx: &mut Transaction<'_, Postgres>,
    message_id: Uuid,
) -> Result<Vec<AttachmentMeta>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, original_filename, file_size, mime_type 
         FROM attachments 
         WHERE message_id = $1"
    )
    .bind(message_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows.into_iter().map(|row| AttachmentMeta {
        id: row.get("id"),
        original_filename: row.get("original_filename"),
        file_size: row.get("file_size"),
        mime_type: row.get("mime_type"),
    }).collect())
}

pub async fn get_attachments_for_messages(
    tx: &mut Transaction<'_, Postgres>,
    message_ids: &[Uuid],
) -> Result<Vec<(Uuid, AttachmentMeta)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, message_id, original_filename, file_size, mime_type 
         FROM attachments 
         WHERE message_id = ANY($1)"
    )
    .bind(message_ids)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows.into_iter().map(|row| {
        let msg_id: Uuid = row.get("message_id");
        let meta = AttachmentMeta {
            id: row.get("id"),
            original_filename: row.get("original_filename"),
            file_size: row.get("file_size"),
            mime_type: row.get("mime_type"),
        };
        (msg_id, meta)
    }).collect())
}

pub async fn link_attachments_to_message(
    tx: &mut Transaction<'_, Postgres>,
    attachment_ids: &[Uuid],
    message_id: Uuid,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE attachments 
         SET message_id = $1 
         WHERE id = ANY($2) AND uploaded_by = $3"
    )
    .bind(message_id)
    .bind(attachment_ids)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
