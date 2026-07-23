use std::path::PathBuf;
use uuid::Uuid;
use bytes::Bytes;
use sqlx::PgPool;
use sqlx::Row;
use crate::config::Config;
use crate::messages::AttachmentMeta;
use crate::db::attachments;

#[derive(Debug, PartialEq, Eq)]
pub enum AppError {
    BadRequest(String),
    NotFound(String),
    Forbidden(String),
    Internal(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal Error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

pub async fn upload_attachment(
    pool: &PgPool,
    config: &Config,
    user_id: Uuid,
    filename: String,
    mime_type: String,
    data: Bytes,
) -> Result<AttachmentMeta, AppError> {
    // 1. Validate size
    if data.len() as u64 > config.max_attachment_size_bytes {
        return Err(AppError::BadRequest(format!(
            "File size {} exceeds limit of {}",
            data.len(),
            config.max_attachment_size_bytes
        )));
    }

    // 2. Validate MIME type
    let allowed_mimes = [
        "image/jpeg", "image/png", "image/gif", "image/webp",
        "video/mp4", "video/webm",
        "audio/mpeg", "audio/wav", "audio/ogg",
        "application/pdf", "application/octet-stream",
        "text/plain",
    ];
    if !allowed_mimes.contains(&mime_type.as_str()) {
        return Err(AppError::BadRequest(format!("MIME type {} not allowed", mime_type)));
    }

    // 3. Sanitize filename
    let sanitized_filename = PathBuf::from(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    let sanitized_filename = if sanitized_filename.len() > 255 {
        sanitized_filename[..255].to_string()
    } else {
        sanitized_filename
    };

    // 4. Generate stored filename
    let path_for_ext = PathBuf::from(&sanitized_filename);
    let extension = path_for_ext
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    let stored_filename = if extension.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        format!("{}.{}", Uuid::new_v4(), extension)
    };

    // 5. DB Transaction
    let mut tx = pool.begin().await.map_err(|e| AppError::Internal(e.to_string()))?;
    let meta = attachments::insert_attachment(
        &mut tx,
        &sanitized_filename,
        &stored_filename,
        data.len() as i64,
        &mime_type,
        user_id
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;
    tx.commit().await.map_err(|e| AppError::Internal(e.to_string()))?;

    // 6. Write to disk
    let file_path = PathBuf::from(&config.attachment_storage_dir).join(&stored_filename);
    tokio::fs::write(&file_path, data).await.map_err(|e| {
        tracing::error!("Failed to write attachment to disk at {:?}: {}", file_path, e);
        AppError::Internal("Failed to save file".to_string())
    })?;

    Ok(meta)
}

pub async fn get_attachment_for_download(
    pool: &PgPool,
    config: &Config,
    requester_id: Uuid,
    attachment_id: Uuid,
) -> Result<(AttachmentMeta, PathBuf), AppError> {
    let mut tx = pool.begin().await.map_err(|e| AppError::Internal(e.to_string()))?;
    
    let row = sqlx::query(
        "SELECT a.id, a.original_filename, a.stored_filename, a.file_size, a.mime_type, a.message_id, a.uploaded_by, m.channel_id
         FROM attachments a
         LEFT JOIN messages m ON a.message_id = m.id
         WHERE a.id = $1"
    )
    .bind(attachment_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let row = match row {
        Some(r) => r,
        None => return Err(AppError::NotFound("Attachment not found".to_string())),
    };

    let meta = AttachmentMeta {
        id: row.get("id"),
        original_filename: row.get("original_filename"),
        file_size: row.get("file_size"),
        mime_type: row.get("mime_type"),
    };
    let stored_filename: String = row.get("stored_filename");
    let message_id: Option<Uuid> = row.get("message_id");
    let uploaded_by: Uuid = row.get("uploaded_by");
    let channel_id: Option<Uuid> = row.get("channel_id");

    tx.commit().await.map_err(|e| AppError::Internal(e.to_string()))?;

    if message_id.is_some() {
        if let Some(chan_id) = channel_id {
            match crate::logic::authz::ensure_channel_member(pool, requester_id, chan_id).await {
                Ok(()) => {},
                Err(_) => return Err(AppError::Forbidden("Not authorized to access attachment".to_string())),
            }
        } else {
            return Err(AppError::Forbidden("Not authorized to access attachment".to_string()));
        }
    } else if uploaded_by != requester_id {
        return Err(AppError::Forbidden("Not authorized to access attachment".to_string()));
    }

    let file_path = PathBuf::from(&config.attachment_storage_dir).join(stored_filename);
    Ok((meta, file_path))
}
