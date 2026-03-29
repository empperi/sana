use sana::router::create_router;
use sana::state::{AppState, CombinedState};
use sana::config::Config;
use sana::db;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use tower::ServiceExt;
use serde_json::Value;
use axum_extra::extract::cookie::{Cookie, Key};
use uuid::Uuid;
use chrono::Utc;

#[path = "db/common.rs"]
mod common;
use common::TestContext;

#[tokio::test]
async fn test_get_channels_unauthorized() {
    let ctx = TestContext::new("sana_test_api_unauth").await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    
    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let combined_state = CombinedState {
        app: app_state,
        cookie_key: Key::generate(),
        config,
    };
    let app = create_router(combined_state);

    let response: Response = app
        .oneshot(Request::builder().uri("/api/channels").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_channels_authorized() {
    let ctx = TestContext::new("sana_test_api_auth").await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    
    let key = Key::generate();
    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let combined_state = CombinedState {
        app: app_state,
        cookie_key: key.clone(),
        config,
    };
    let app = create_router(combined_state);

    // 1. Create a user in DB
    let mut tx = ctx.pool.begin().await.unwrap();
    let user = db::users::create_user(&mut tx, "api_user", "pass").await.unwrap();
    tx.commit().await.unwrap();

    // 2. Create a signed cookie
    let cookie = Cookie::new("session_id", user.id.to_string());
    let signed_jar = axum_extra::extract::cookie::SignedCookieJar::new(key).add(cookie);
    
    use axum::response::IntoResponse;
    let response = signed_jar.into_response();
    let cookie_header = response.headers().get("Set-Cookie").unwrap().to_str().unwrap().to_string();

    let response: Response = app
        .oneshot(
            Request::builder()
                .uri("/api/channels")
                .header("Cookie", cookie_header)
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let channels: Value = serde_json::from_slice(&body).unwrap();
    assert!(channels.is_array());
}

#[tokio::test]
async fn test_search_unjoined_channels() {
    let ctx = TestContext::new("sana_test_api_search").await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    
    let key = Key::generate();
    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let combined_state = CombinedState {
        app: app_state,
        cookie_key: key.clone(),
        config,
    };
    let app = create_router(combined_state);

    let mut tx = ctx.pool.begin().await.unwrap();
    let user = db::users::create_user(&mut tx, "search_user", "pass").await.unwrap();
    let _c1 = db::channels::Channel { id: Uuid::new_v4(), name: "find-me".to_string(), is_private: false, created_at: Utc::now() };
    db::channels::insert_channel(&mut tx, &_c1).await.unwrap();
    tx.commit().await.unwrap();

    let cookie = Cookie::new("session_id", user.id.to_string());
    let signed_jar = axum_extra::extract::cookie::SignedCookieJar::new(key).add(cookie);
    use axum::response::IntoResponse;
    let cookie_header = signed_jar.into_response().headers().get("Set-Cookie").unwrap().to_str().unwrap().to_string();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/channels/unjoined?q=find")
                .header("Cookie", cookie_header)
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let channels: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(channels.as_array().unwrap().len(), 1);
    assert_eq!(channels[0]["name"], "find-me");
}

#[tokio::test]
async fn test_join_channel_api() {
    let ctx = TestContext::new("sana_test_api_join").await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    
    let key = Key::generate();
    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let combined_state = CombinedState {
        app: app_state,
        cookie_key: key.clone(),
        config,
    };
    let app = create_router(combined_state);

    let mut tx = ctx.pool.begin().await.unwrap();
    let user = db::users::create_user(&mut tx, "join_user", "pass").await.unwrap();
    let c1 = db::channels::Channel { id: Uuid::new_v4(), name: "join-me".to_string(), is_private: false, created_at: Utc::now() };
    db::channels::insert_channel(&mut tx, &c1).await.unwrap();
    tx.commit().await.unwrap();

    let cookie = Cookie::new("session_id", user.id.to_string());
    let signed_jar = axum_extra::extract::cookie::SignedCookieJar::new(key).add(cookie);
    use axum::response::IntoResponse;
    let cookie_header = signed_jar.into_response().headers().get("Set-Cookie").unwrap().to_str().unwrap().to_string();

    let join_payload = serde_json::json!({
        "channel_id": c1.id
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/channels/join")
                .header("Cookie", cookie_header)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(serde_json::to_vec(&join_payload).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify in DB
    let joined = db::channels::get_user_channels(&ctx.pool, user.id).await.unwrap();
    assert!(joined.iter().any(|c| c.id == c1.id));
}

#[tokio::test]
async fn test_create_channel_api() {
    let ctx = TestContext::new("sana_test_api_create").await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    
    let key = Key::generate();
    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let combined_state = CombinedState {
        app: app_state,
        cookie_key: key.clone(),
        config,
    };
    let app = create_router(combined_state);

    let mut tx = ctx.pool.begin().await.unwrap();
    let user = db::users::create_user(&mut tx, "creator", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let cookie = Cookie::new("session_id", user.id.to_string());
    let signed_jar = axum_extra::extract::cookie::SignedCookieJar::new(key).add(cookie);
    use axum::response::IntoResponse;
    let cookie_header = signed_jar.into_response().headers().get("Set-Cookie").unwrap().to_str().unwrap().to_string();

    let create_payload = serde_json::json!({
        "name": "brand-new-channel"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/channels")
                .header("Cookie", cookie_header)
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(serde_json::to_vec(&create_payload).unwrap()))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    // Verify channel exists AND user has joined
    let joined = db::channels::get_user_channels(&ctx.pool, user.id).await.unwrap();
    assert!(joined.iter().any(|c| c.name == "brand-new-channel"), "Creator should be automatically joined to the channel");
}

#[tokio::test]
async fn test_health_endpoint() {
    let ctx = TestContext::new("sana_test_api_health").await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());

    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let combined_state = CombinedState {
        app: app_state,
        cookie_key: Key::generate(),
        config,
    };
    let app = create_router(combined_state);

    let response: Response = app
        .oneshot(Request::builder().uri("/health").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body, "OK");
}
