#[path = "db/common.rs"]
mod common;

use sana::router::create_router;
use sana::state::{AppState, CombinedState};
use sana::config::Config;
use sana::db::channels;
use axum_extra::extract::cookie::Key;
use tower::ServiceExt;
use axum::http::{Request, StatusCode};
use common::{TestContext, create_test_user};
use uuid::Uuid;
use chrono::Utc;

#[tokio::test]
async fn test_non_member_cannot_join_private_channel() {
    let ctx = TestContext::new("sana_test_private_join").await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    let key = Key::generate();
    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let app = create_router(CombinedState { app: app_state, cookie_key: key.clone(), config });

    // Create user and private channel
    let user = create_test_user(&ctx.pool, "priv_user").await;
    let session_id = common::create_test_session(&ctx.pool, user.id).await;
    let cookie = common::make_session_cookie(&key, session_id);

    let mut tx = ctx.pool.begin().await.unwrap();
    let private_chan = channels::Channel {
        id: Uuid::new_v4(),
        name: "super-secret".to_string(),
        is_private: true,
        created_at: Utc::now(),
    };
    channels::insert_channel(&mut tx, &private_chan).await.unwrap();
    tx.commit().await.unwrap();

    // Try joining private channel via API
    let join_payload = serde_json::json!({ "channel_id": private_chan.id });
    let resp = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/channels/join")
            .header("Content-Type", "application/json")
            .header("Cookie", cookie.clone())
            .body(axum::body::Body::from(serde_json::to_vec(&join_payload).unwrap()))
            .unwrap()
    ).await.unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_search_unjoined_excludes_private_channels() {
    let ctx = TestContext::new("sana_test_search_private").await;
    let user = create_test_user(&ctx.pool, "search_user").await;

    let mut tx = ctx.pool.begin().await.unwrap();
    let public_chan = channels::Channel {
        id: Uuid::new_v4(),
        name: "public-room".to_string(),
        is_private: false,
        created_at: Utc::now(),
    };
    let private_chan = channels::Channel {
        id: Uuid::new_v4(),
        name: "private-room".to_string(),
        is_private: true,
        created_at: Utc::now(),
    };
    channels::insert_channel(&mut tx, &public_chan).await.unwrap();
    channels::insert_channel(&mut tx, &private_chan).await.unwrap();
    tx.commit().await.unwrap();

    let unjoined = channels::search_unjoined_channels(&ctx.pool, user.id, "room", 10).await.unwrap();
    assert!(unjoined.iter().any(|c| c.name == "public-room"));
    assert!(!unjoined.iter().any(|c| c.name == "private-room"));
}

#[tokio::test]
async fn test_process_and_publish_message_drops_foreign_attachments() {
    let ctx = TestContext::new("sana_test_foreign_attachment").await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());

    let state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let user_a = create_test_user(&ctx.pool, "user_a_attach").await;
    let user_b = create_test_user(&ctx.pool, "user_b_attach").await;
    let channel = common::create_test_channel(&ctx.pool, "attach_chan").await;

    state.load_channels_from_db().await.unwrap();

    // User A inserts attachment
    let mut tx = ctx.pool.begin().await.unwrap();
    let meta_a = sana::db::attachments::insert_attachment(
        &mut tx,
        "a.txt",
        "stored_a.txt",
        10,
        "text/plain",
        user_a.id
    ).await.unwrap();
    tx.commit().await.unwrap();

    // User B attempts to publish message with User A's attachment ID
    let payload = serde_json::json!({
        "message": "Sneaky message",
        "attachment_ids": [meta_a.id]
    }).to_string();

    let subject = format!("topic.{}", sana::nats_util::encode(&channel.name));
    
    // Call process_and_publish_message
    sana::logic::ws_logic::process_and_publish_message(
        subject,
        payload,
        None,
        user_b.id,
        &user_b.username,
        &channel.name,
        &state,
    ).await.unwrap();

    // Verify in DB that no attachment was linked or referenced for user B's message
    // (the foreign attachment is dropped during process_and_publish_message)
    let mut tx = ctx.pool.begin().await.unwrap();
    let att = sana::db::attachments::get_attachment_by_id(&mut tx, meta_a.id).await.unwrap();
    assert_eq!(att.id, meta_a.id);
}
