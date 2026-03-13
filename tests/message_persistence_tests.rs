use sana::router::create_router;
use sana::state::{AppState, CombinedState};
use sana::config::Config;
use sana::db;
use sana::logic::archiver;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use tower::ServiceExt;
use serde_json::Value;
use axum_extra::extract::cookie::{Cookie, Key};
use uuid::Uuid;

#[path = "db/common.rs"]
mod common;
use common::{TestContext, create_test_user, create_test_channel};

async fn setup_app(ctx: &TestContext, key: Key) -> (axum::Router, AppState) {
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    
    // Ensure stream exists
    let _ = jetstream.get_or_create_stream(async_nats::jetstream::stream::Config {
        name: "SANA".to_string(),
        subjects: vec!["topic.>".to_string()],
        ..Default::default()
    }).await.unwrap();

    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    app_state.load_channels_from_db().await.unwrap();

    let combined_state = CombinedState {
        app: app_state.clone(),
        cookie_key: key,
    };
    
    // Start archiver
    archiver::start(app_state.clone()).await;
    
    (create_router(combined_state), app_state)
}

fn auth_header(user_id: Uuid, key: Key) -> String {
    let cookie = Cookie::new("session_id", user_id.to_string());
    let jar = axum_extra::extract::cookie::SignedCookieJar::new(key).add(cookie);
    use axum::response::IntoResponse;
    jar.into_response().headers().get("Set-Cookie").unwrap().to_str().unwrap().to_string()
}

#[tokio::test]
async fn test_message_persistence_to_db() {
    let ctx = TestContext::new("persistence_test").await;
    let key = Key::generate();
    let (app, state) = setup_app(&ctx, key.clone()).await;

    let user = create_test_user(&ctx.pool, "p_user").await;
    let channel = create_test_channel(&ctx.pool, "p_chan").await;
    let cookie = auth_header(user.id, key);

    // 1. Publish a message to NATS via the logic function
    let subject = format!("topic.{}", sana::nats_util::encode(&channel.name));
    sana::logic::ws_logic::process_and_publish_message(
        subject,
        "Persistent Message".to_string(),
        None, // random message ID
        user.id,
        &user.username,
        &channel.name,
        &state
    ).await;

    // 2. Wait for archiver to process (it's async background task)
    // We'll poll the DB for a bit
    let mut persisted = false;
    for _ in 0..20 {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let msgs = db::messages::get_messages(&ctx.pool, channel.id, 10, None, false).await.unwrap();
        if msgs.iter().any(|m| m.message == "Persistent Message") {
            persisted = true;
            break;
        }
    }
    
    assert!(persisted, "Message should be persisted to database by archiver");

    // 3. Query via REST API
    let response: Response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/channels/{}/messages?limit=10", channel.id))
                .header("Cookie", cookie)
                .body(axum::body::Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let messages: Value = serde_json::from_slice(&body).unwrap();
    
    assert!(messages.as_array().unwrap().iter().any(|m| m["message"] == "Persistent Message"), 
            "REST API should return the persistent message");
}
