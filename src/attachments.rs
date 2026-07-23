use axum::{
    extract::{State, Path, Multipart},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use crate::state::{AppState, CombinedState};
use crate::logic::attachments::{self, AppError};
use crate::auth::UserSession;
use uuid::Uuid;
use serde::Serialize;

pub fn router() -> Router<CombinedState> {
    Router::new()
        .route("/", post(upload_attachment))
        .route("/:id", get(download_attachment))
}

#[derive(Serialize)]
struct AttachmentError {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(AttachmentError { error: message });
        (status, body).into_response()
    }
}

async fn upload_attachment(
    session: UserSession,
    State(state): State<AppState>,
    State(combined_state): State<CombinedState>,
    mut multipart: Multipart,
) -> Result<Json<crate::messages::AttachmentMeta>, AppError> {
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        if let Some(name) = field.name() {
            if name == "file" {
                let filename = field.file_name().map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());
                let mime_type = field.content_type().map(|s| s.to_string()).unwrap_or_else(|| "application/octet-stream".to_string());
                let data = field.bytes().await.map_err(|e| AppError::Internal(e.to_string()))?;
                
                let meta = attachments::upload_attachment(
                    &state.db_pool,
                    &combined_state.config,
                    session.user_id,
                    filename,
                    mime_type,
                    data
                ).await?;
                
                return Ok(Json(meta));
            }
        }
    }

    Err(AppError::BadRequest("No file field in multipart form".to_string()))
}

async fn download_attachment(
    session: UserSession,
    State(state): State<AppState>,
    State(combined_state): State<CombinedState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let (meta, path) = attachments::get_attachment_for_download(
        &state.db_pool,
        &combined_state.config,
        session.user_id,
        id
    ).await?;

    let file_bytes = tokio::fs::read(&path).await.map_err(|e| {
        tracing::error!("Failed to read file from disk at {:?}: {}", path, e);
        AppError::Internal("Failed to read file".to_string())
    })?;

    let content_disposition = format!("attachment; filename=\"{}\"", meta.original_filename);

    Ok((
        [
            (header::CONTENT_TYPE, meta.mime_type),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        file_bytes,
    ))
}
