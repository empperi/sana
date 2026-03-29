use axum::http::{Request, StatusCode};
use axum_extra::extract::cookie::{Cookie, Key};
use chrono::{DateTime, Duration, Utc};
use sana::db;
use sana::router::create_router;
use sana::state::{AppState, CombinedState};
use sana::messages::{ChatMessage, MessageType};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "db/common.rs"]
mod common;
use common::{create_test_channel, create_test_user, join_test_channel, TestContext};

#[tokio::test]
async fn test_get_messages_success() {
    let ctx = TestContext::new("api_msg_success").await;
    let key = Key::generate();
    let (u, c) = (create_test_user(&ctx.pool, "u").await, create_test_channel(&ctx.pool, "c").await);
    join_test_channel(&ctx.pool, u.id, c.id).await;
    insert_msg(&ctx, c.id, &u, "M1", Utc::now(), 1).await;

    let app = setup_app(&ctx, key.clone()).await;
    let (status, body) = request(&app, &format!("/api/channels/{}/messages?limit=1", c.id), &auth_header(u.id, key)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["message"], "M1");
}

#[tokio::test]
async fn test_get_messages_forbidden() {
    let ctx = TestContext::new("api_msg_forbidden").await;
    let key = Key::generate();
    let (u, c) = (create_test_user(&ctx.pool, "u").await, create_test_channel(&ctx.pool, "c").await);
    // User DOES NOT join channel

    let app = setup_app(&ctx, key.clone()).await;
    let (status, _) = request(&app, &format!("/api/channels/{}/messages?limit=1", c.id), &auth_header(u.id, key)).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_messages_missing_limit() {
    let ctx = TestContext::new("api_msg_no_limit").await;
    let key = Key::generate();
    let (u, c) = (create_test_user(&ctx.pool, "u").await, create_test_channel(&ctx.pool, "c").await);
    join_test_channel(&ctx.pool, u.id, c.id).await;

    let app = setup_app(&ctx, key.clone()).await;
    let (status, _) = request(&app, &format!("/api/channels/{}/messages", c.id), &auth_header(u.id, key)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_messages_pagination() {
    let ctx = TestContext::new("api_msg_pag").await;
    let key = Key::generate();
    let (u, c) = (create_test_user(&ctx.pool, "u").await, create_test_channel(&ctx.pool, "c").await);
    join_test_channel(&ctx.pool, u.id, c.id).await;
    let now = Utc::now();
    insert_msg(&ctx, c.id, &u, "M1", now - Duration::minutes(1), 1).await;
    insert_msg(&ctx, c.id, &u, "M2", now, 2).await;

    let app = setup_app(&ctx, key.clone()).await;
    let auth = auth_header(u.id, key);
    let (_, body1) = request(&app, &format!("/api/channels/{}/messages?limit=1", c.id), &auth).await;
    let last_ts = body1[0]["timestamp"].as_str().unwrap();
    let last_ts_encoded = last_ts.replace(':', "%3A").replace('+', "%2B");
    let (_, body2) = request(&app, &format!("/api/channels/{}/messages?limit=1&before={}", c.id, last_ts_encoded), &auth).await;

    assert_eq!(body2[0]["message"], "M1");
}

#[tokio::test]
async fn test_get_messages_invalid_uuid() {
    let ctx = TestContext::new("api_msg_bad_uuid").await;
    let key = Key::generate();
    let u = create_test_user(&ctx.pool, "u").await;

    let app = setup_app(&ctx, key.clone()).await;
    let (status, _) = request(&app, "/api/channels/invalid/messages?limit=1", &auth_header(u.id, key)).await;

    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_messages_limit_too_high() {
    let ctx = TestContext::new("api_msg_limit_high").await;
    let key = Key::generate();
    let (u, c) = (create_test_user(&ctx.pool, "u").await, create_test_channel(&ctx.pool, "c").await);
    join_test_channel(&ctx.pool, u.id, c.id).await;

    let app = setup_app(&ctx, key.clone()).await;
    let (status, _) = request(&app, &format!("/api/channels/{}/messages?limit=1001", c.id), &auth_header(u.id, key)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// Helper functions and fixtures below

async fn setup_app(ctx: &TestContext, key: Key) -> axum::Router {
    let config = sana::config::Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    create_router(CombinedState { app: app_state, cookie_key: key, config })
}

fn auth_header(user_id: Uuid, key: Key) -> String {
    let cookie = Cookie::new("session_id", user_id.to_string());
    let jar = axum_extra::extract::cookie::SignedCookieJar::new(key).add(cookie);
    use axum::response::IntoResponse;
    jar.into_response().headers().get("Set-Cookie").unwrap().to_str().unwrap().to_string()
}

async fn insert_msg(ctx: &TestContext, chan_id: Uuid, user: &db::users::User, text: &str, ts: DateTime<Utc>, seq: u64) {
    let msg = ChatMessage {
        id: Uuid::new_v4(), 
        channel_id: chan_id, 
        user_id: user.id,
        user: user.username.clone(), 
        timestamp: ts, 
        message: text.to_string(), 
        seq: Some(seq),
        msg_type: MessageType::Chat,
    };
    let mut tx = ctx.pool.begin().await.unwrap();
    db::messages::insert_message(&mut tx, seq, &msg).await.unwrap();
    tx.commit().await.unwrap();
}

async fn request(app: &axum::Router, uri: &str, auth: &str) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(Request::builder().uri(uri).header("Cookie", auth).body(axum::body::Body::empty()).unwrap()).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}
