#[path = "db/common.rs"]
mod common;

use sana::router::create_router;
use sana::state::{AppState, CombinedState};
use sana::config::Config;
use sana::db::channels;
use sana::messages::ChannelEntry;
use axum_extra::extract::cookie::Key;
use tower::ServiceExt;
use axum::http::{Request, StatusCode};
use common::{TestContext, create_test_user};
use uuid::Uuid;
use chrono::Utc;
use futures::StreamExt;
use futures::SinkExt;
use tokio_tungstenite::tungstenite::Message as WsMsg;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

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

    let state = AppState::new(nats_client.clone(), jetstream, ctx.pool.clone());
    let user_a = create_test_user(&ctx.pool, "user_a_attach").await;
    let user_b = create_test_user(&ctx.pool, "user_b_attach").await;
    let channel = common::create_test_channel(&ctx.pool, "attach_chan").await;

    state.load_channels_from_db().await.unwrap();

    // User A inserts attachment (foreign to User B)
    let mut tx = ctx.pool.begin().await.unwrap();
    let meta_a = sana::db::attachments::insert_attachment(
        &mut tx,
        "a.txt",
        "stored_a.txt",
        10,
        "text/plain",
        user_a.id
    ).await.unwrap();

    // User B inserts attachment (self-owned by User B)
    let meta_b = sana::db::attachments::insert_attachment(
        &mut tx,
        "b.txt",
        "stored_b.txt",
        20,
        "text/plain",
        user_b.id
    ).await.unwrap();
    tx.commit().await.unwrap();

    // Subscribe to NATS subject BEFORE calling process_and_publish_message
    let subject_str = format!("topic.{}", sana::nats_util::encode(&channel.name));
    let mut sub = nats_client.subscribe(subject_str.clone()).await.unwrap();

    // User B publishes message referencing both User A's (foreign) and User B's (own) attachments
    let payload = serde_json::json!({
        "message": "Mixed attachments message",
        "attachment_ids": [meta_a.id, meta_b.id]
    }).to_string();

    sana::logic::ws_logic::process_and_publish_message(
        subject_str,
        payload,
        None,
        user_b.id,
        &user_b.username,
        &channel.name,
        &state,
    ).await.unwrap();

    // Receive published NATS message with timeout
    let msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), sub.next())
        .await
        .expect("Timeout waiting for NATS message")
        .expect("NATS stream ended");

    let entry: ChannelEntry = serde_json::from_slice(&msg.payload).unwrap();
    if let ChannelEntry::Message(chat_msg) = entry {
        assert_eq!(chat_msg.attachments.len(), 1, "Only self-owned attachment should remain");
        assert_eq!(chat_msg.attachments[0].id, meta_b.id, "Self-owned attachment must be present");
        assert!(!chat_msg.attachments.iter().any(|att| att.id == meta_a.id), "Foreign attachment must be dropped");
    } else {
        panic!("Expected ChannelEntry::Message");
    }
}

#[tokio::test]
async fn test_ws_non_member_subscribe_and_send_error() {
    let ctx = TestContext::new("sana_test_ws_authz").await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    let key = Key::generate();

    let app_state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    let app = create_router(CombinedState {
        app: app_state,
        cookie_key: key.clone(),
        config,
    });

    // Ephemeral TCP server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Create user and channel (user is NOT joined)
    let non_member = create_test_user(&ctx.pool, "ws_non_member").await;
    let channel = common::create_test_channel(&ctx.pool, "ws-secret-chan").await;
    let session_id = common::create_test_session(&ctx.pool, non_member.id).await;
    let cookie_val = common::make_session_cookie(&key, session_id);

    // Connect via tokio-tungstenite
    let ws_url = format!("ws://{}/ws", addr);
    let mut req = ws_url.into_client_request().unwrap();
    req.headers_mut().insert("Cookie", cookie_val.parse().unwrap());

    let (mut ws_stream, _) = tokio_tungstenite::connect_async(req).await.unwrap();

    // 1. Non-member SUBSCRIBE -> expects STOMP ERROR frame
    let sub_frame = format!("SUBSCRIBE\ndestination:/topic/{}\n\n\0", channel.name);
    ws_stream.send(WsMsg::Text(sub_frame)).await.unwrap();

    let resp_msg = tokio::time::timeout(tokio::time::Duration::from_secs(2), ws_stream.next())
        .await
        .expect("Timeout waiting for SUBSCRIBE error response")
        .expect("Stream closed unexpectedly");

    let resp_text = resp_msg.unwrap().into_text().unwrap();
    assert!(resp_text.starts_with("ERROR"), "Expected STOMP ERROR frame for non-member SUBSCRIBE, got: {}", resp_text);
    assert!(resp_text.contains("User is not a member of the channel"), "ERROR frame message check: {}", resp_text);

    // 2. Non-member SEND -> expects STOMP ERROR frame
    let send_frame = format!("SEND\ndestination:/topic/{}\n\nUnauthorized message\0", channel.name);
    ws_stream.send(WsMsg::Text(send_frame)).await.unwrap();

    let resp_msg2 = tokio::time::timeout(tokio::time::Duration::from_secs(2), ws_stream.next())
        .await
        .expect("Timeout waiting for SEND error response")
        .expect("Stream closed unexpectedly");

    let resp_text2 = resp_msg2.unwrap().into_text().unwrap();
    assert!(resp_text2.starts_with("ERROR"), "Expected STOMP ERROR frame for non-member SEND, got: {}", resp_text2);
    assert!(resp_text2.contains("User is not a member of the channel"), "ERROR frame message check: {}", resp_text2);

    // 3. Verify in DB that no message was persisted
    let mut tx = ctx.pool.begin().await.unwrap();
    let msgs = sana::db::messages::get_messages(&mut tx, channel.id, 10, None, false).await.unwrap();
    assert!(msgs.is_empty(), "No message should be persisted to DB for non-member send");
}
