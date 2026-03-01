use sana::router::create_router;
use sana::state::{AppState, CombinedState};
use sana::config::Config;
use sana::db;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use tower::ServiceExt;
use serde_json::Value;
use axum_extra::extract::cookie::{Cookie, Key};

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
