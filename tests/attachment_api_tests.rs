use sana::router::create_router;
use sana::state::{AppState, CombinedState};
use sana::config::Config;
use sana::db;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use serde_json::Value;
use axum_extra::extract::cookie::Key;
use uuid::Uuid;

#[path = "db/common.rs"]
mod common;
use common::TestContext;

async fn setup_app(ctx: &TestContext, max_size_bytes: Option<u64>) -> (axum::Router, Key, Config) {
    let mut config = Config::load(None);
    if let Some(size) = max_size_bytes {
        config.max_attachment_size_bytes = size;
    }
    
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    
    let key = Key::generate();
    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let combined_state = CombinedState {
        app: app_state,
        cookie_key: key.clone(),
        config: config.clone(),
    };
    (create_router(combined_state), key, config)
}

async fn get_auth_header(pool: &sqlx::PgPool, user_id: Uuid, key: Key) -> String {
    let session_id = common::create_test_session(pool, user_id).await;
    common::make_session_cookie(&key, session_id)
}

#[tokio::test]
async fn test_upload_attachment_api() {
    let ctx = TestContext::new("api_upload_test").await;
    let (app, key, _) = setup_app(&ctx, None).await;

    let mut tx = ctx.pool.begin().await.unwrap();
    let user = db::users::create_user(&mut tx, "uploader", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let auth = get_auth_header(&ctx.pool, user.id, key).await;

    let boundary = "---------------------------1234567890";
    let body = format!(
        "--{boundary}\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
        Content-Type: text/plain\r\n\
        \r\n\
        hello world\r\n\
        --{boundary}--\r\n"
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attachments")
                .header("Cookie", auth)
                .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
                .body(axum::body::Body::from(body))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let meta: Value = serde_json::from_slice(&body_bytes).unwrap();
    
    assert_eq!(meta["original_filename"], "test.txt");
    assert_eq!(meta["mime_type"], "text/plain");
    assert_eq!(meta["file_size"], 11);
    assert!(meta["id"].is_string());
}

#[tokio::test]
async fn test_download_attachment_api() {
    let ctx = TestContext::new("api_download_test").await;
    let (app, key, config) = setup_app(&ctx, None).await;

    let mut tx = ctx.pool.begin().await.unwrap();
    let user = db::users::create_user(&mut tx, "downloader", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let auth = get_auth_header(&ctx.pool, user.id, key).await;

    // 1. Manually insert an attachment into DB and create file on disk
    let stored_filename = format!("{}.txt", Uuid::new_v4());
    let file_path = std::path::PathBuf::from(&config.attachment_storage_dir).join(&stored_filename);
    std::fs::write(&file_path, "secret data").unwrap();

    let mut tx = ctx.pool.begin().await.unwrap();
    let meta = db::attachments::insert_attachment(
        &mut tx,
        "secret.txt",
        &stored_filename,
        11,
        "text/plain",
        user.id
    ).await.unwrap();
    tx.commit().await.unwrap();

    // 2. Download via API
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/attachments/{}", meta.id))
                .header("Cookie", auth)
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("Content-Type").unwrap(), "text/plain");
    assert!(response.headers().get("Content-Disposition").unwrap().to_str().unwrap().contains("secret.txt"));

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body_bytes, "secret data");
}

#[tokio::test]
async fn test_upload_unauthorized() {
    let ctx = TestContext::new("api_upload_unauth").await;
    let (app, _, _) = setup_app(&ctx, None).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attachments")
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_upload_too_large() {
    let ctx = TestContext::new("api_upload_large").await;
    // Set max size to 5 bytes
    let (app, key, _) = setup_app(&ctx, Some(5)).await;

    let mut tx = ctx.pool.begin().await.unwrap();
    let user = db::users::create_user(&mut tx, "uploader_large", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let auth = get_auth_header(&ctx.pool, user.id, key).await;

    let boundary = "---------------------------1234567890";
    let body = format!(
        "--{boundary}\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\
        Content-Type: text/plain\r\n\
        \r\n\
        hello world\r\n\
        --{boundary}--\r\n"
    ); // "hello world\r\n" is 13 bytes, > 5

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attachments")
                .header("Cookie", auth)
                .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
                .body(axum::body::Body::from(body))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_upload_invalid_mime() {
    let ctx = TestContext::new("api_upload_mime").await;
    let (app, key, _) = setup_app(&ctx, None).await;

    let mut tx = ctx.pool.begin().await.unwrap();
    let user = db::users::create_user(&mut tx, "uploader_mime", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let auth = get_auth_header(&ctx.pool, user.id, key).await;

    let boundary = "---------------------------1234567890";
    let body = format!(
        "--{boundary}\r\n\
        Content-Disposition: form-data; name=\"file\"; filename=\"test.sh\"\r\n\
        Content-Type: application/x-sh\r\n\
        \r\n\
        echo hello\r\n\
        --{boundary}--\r\n"
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/attachments")
                .header("Cookie", auth)
                .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
                .body(axum::body::Body::from(body))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_download_not_found() {
    let ctx = TestContext::new("api_download_404").await;
    let (app, key, _) = setup_app(&ctx, None).await;

    let mut tx = ctx.pool.begin().await.unwrap();
    let user = db::users::create_user(&mut tx, "downloader_404", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let auth = get_auth_header(&ctx.pool, user.id, key).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/attachments/{}", Uuid::new_v4()))
                .header("Cookie", auth)
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_download_unauthorized() {
    let ctx = TestContext::new("api_download_unauth").await;
    let (app, _, _) = setup_app(&ctx, None).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/attachments/{}", Uuid::new_v4()))
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_download_unlinked_attachment_forbidden_for_other_user() {
    let ctx = TestContext::new("api_download_unlinked_forbidden").await;
    let (app, key, config) = setup_app(&ctx, None).await;

    let mut tx = ctx.pool.begin().await.unwrap();
    let uploader = db::users::create_user(&mut tx, "uploader_unlinked", "pass").await.unwrap();
    let other_user = db::users::create_user(&mut tx, "other_unlinked", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let other_auth = get_auth_header(&ctx.pool, other_user.id, key).await;

    let stored_filename = format!("{}.txt", Uuid::new_v4());
    let file_path = std::path::PathBuf::from(&config.attachment_storage_dir).join(&stored_filename);
    std::fs::write(&file_path, "unlinked file data").unwrap();

    let mut tx = ctx.pool.begin().await.unwrap();
    let meta = db::attachments::insert_attachment(
        &mut tx,
        "unlinked.txt",
        &stored_filename,
        18,
        "text/plain",
        uploader.id
    ).await.unwrap();
    tx.commit().await.unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/attachments/{}", meta.id))
                .header("Cookie", other_auth)
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_download_linked_attachment_channel_membership() {
    let ctx = TestContext::new("api_download_linked_authz").await;
    let (app, key, config) = setup_app(&ctx, None).await;

    let member_user = common::create_test_user(&ctx.pool, "chan_member").await;
    let non_member_user = common::create_test_user(&ctx.pool, "chan_non_member").await;
    let channel = common::create_test_channel(&ctx.pool, "linked_chan").await;
    common::join_test_channel(&ctx.pool, member_user.id, channel.id).await;

    let member_auth = get_auth_header(&ctx.pool, member_user.id, key.clone()).await;
    let non_member_auth = get_auth_header(&ctx.pool, non_member_user.id, key).await;

    let stored_filename = format!("{}.txt", Uuid::new_v4());
    let file_path = std::path::PathBuf::from(&config.attachment_storage_dir).join(&stored_filename);
    std::fs::write(&file_path, "linked file data").unwrap();

    let mut tx = ctx.pool.begin().await.unwrap();
    let meta = db::attachments::insert_attachment(
        &mut tx,
        "linked.txt",
        &stored_filename,
        16,
        "text/plain",
        member_user.id
    ).await.unwrap();

    let msg_id = Uuid::new_v4();
    let msg = sana::messages::ChatMessage {
        id: msg_id,
        channel_id: channel.id,
        user_id: member_user.id,
        user: member_user.username.clone(),
        timestamp: chrono::Utc::now(),
        message: "Check attachment".to_string(),
        seq: Some(1),
        msg_type: sana::messages::MessageType::Chat,
        attachments: vec![],
    };
    db::messages::insert_message(&mut tx, 1, &msg).await.unwrap();
    db::attachments::link_attachments_to_message(&mut tx, &[meta.id], msg_id, member_user.id).await.unwrap();
    tx.commit().await.unwrap();

    // Member gets 200 OK
    let member_resp = app.clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/attachments/{}", meta.id))
                .header("Cookie", member_auth)
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    assert_eq!(member_resp.status(), StatusCode::OK);

    // Non-member gets 403 FORBIDDEN
    let non_member_resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/attachments/{}", meta.id))
                .header("Cookie", non_member_auth)
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();
    assert_eq!(non_member_resp.status(), StatusCode::FORBIDDEN);
}
