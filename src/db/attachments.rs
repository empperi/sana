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

pub async fn get_attachment_by_id_and_uploader(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    uploaded_by: Uuid,
) -> Result<Option<AttachmentMeta>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, original_filename, file_size, mime_type 
         FROM attachments 
         WHERE id = $1 AND uploaded_by = $2"
    )
    .bind(id)
    .bind(uploaded_by)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(|r| AttachmentMeta {
        id: r.get("id"),
        original_filename: r.get("original_filename"),
        file_size: r.get("file_size"),
        mime_type: r.get("mime_type"),
    }))
}

pub struct AttachmentDownloadInfo {
    pub meta: AttachmentMeta,
    pub stored_filename: String,
    pub message_id: Option<Uuid>,
    pub uploaded_by: Uuid,
    pub channel_id: Option<Uuid>,
}

pub async fn get_attachment_download_info(
    tx: &mut Transaction<'_, Postgres>,
    attachment_id: Uuid,
) -> Result<Option<AttachmentDownloadInfo>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT a.id, a.original_filename, a.stored_filename, a.file_size, a.mime_type, a.message_id, a.uploaded_by, m.channel_id
         FROM attachments a
         LEFT JOIN messages m ON a.message_id = m.id
         WHERE a.id = $1"
    )
    .bind(attachment_id)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(|r| AttachmentDownloadInfo {
        meta: AttachmentMeta {
            id: r.get("id"),
            original_filename: r.get("original_filename"),
            file_size: r.get("file_size"),
            mime_type: r.get("mime_type"),
        },
        stored_filename: r.get("stored_filename"),
        message_id: r.get("message_id"),
        uploaded_by: r.get("uploaded_by"),
        channel_id: r.get("channel_id"),
    }))
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
