#[path = "db/common.rs"]
mod common;

use sana::logic::authz::{self, AuthzError};
use sana::state::AppState;
use sana::config::Config;
use common::{TestContext, create_test_user, create_test_channel, join_test_channel};
use uuid::Uuid;

#[tokio::test]
async fn test_member_passes() {
    let ctx = TestContext::new("sana_test_authz_member").await;
    let pool = &ctx.pool;

    let user = create_test_user(pool, "authz_user1").await;
    let channel = create_test_channel(pool, "chan_authz1").await;
    join_test_channel(pool, user.id, channel.id).await;

    let res = authz::ensure_channel_member(pool, user.id, channel.id).await;
    assert_eq!(res, Ok(()));
}

#[tokio::test]
async fn test_non_member_fails() {
    let ctx = TestContext::new("sana_test_authz_non_member").await;
    let pool = &ctx.pool;

    let user = create_test_user(pool, "authz_user2").await;
    let channel = create_test_channel(pool, "chan_authz2").await;

    let res = authz::ensure_channel_member(pool, user.id, channel.id).await;
    assert_eq!(res, Err(AuthzError::NotAMember));
}

#[tokio::test]
async fn test_unknown_channel_fails() {
    let ctx = TestContext::new("sana_test_authz_unknown_channel").await;
    let pool = &ctx.pool;

    let user = create_test_user(pool, "authz_user3").await;
    let fake_channel_id = Uuid::new_v4();

    let res = authz::ensure_channel_member(pool, user.id, fake_channel_id).await;
    assert_eq!(res, Err(AuthzError::ChannelNotFound));
}

#[tokio::test]
async fn test_system_channels_always_allowed() {
    let (ctx, state) = setup_app_state("sana_test_authz_system").await;
    let user = create_test_user(&ctx.pool, "system_user").await;

    let res = authz::ensure_channel_member_by_name(&state, user.id, "system.channels").await;
    assert_eq!(res, Ok(()));
}

#[tokio::test]
async fn test_by_name_member_and_non_member() {
    let (ctx, state) = setup_app_state("sana_test_authz_by_name").await;
    let pool = &ctx.pool;

    let user1 = create_test_user(pool, "by_name_user1").await;
    let user2 = create_test_user(pool, "by_name_user2").await;
    let channel = create_test_channel(pool, "by-name-channel").await;
    join_test_channel(pool, user1.id, channel.id).await;

    state.load_channels_from_db().await.unwrap();

    let res1 = authz::ensure_channel_member_by_name(&state, user1.id, "by-name-channel").await;
    assert_eq!(res1, Ok(()));

    let res2 = authz::ensure_channel_member_by_name(&state, user2.id, "by-name-channel").await;
    assert_eq!(res2, Err(AuthzError::NotAMember));

    let res3 = authz::ensure_channel_member_by_name(&state, user1.id, "nonexistent-channel").await;
    assert_eq!(res3, Err(AuthzError::ChannelNotFound));
}

async fn setup_app_state(db_name: &str) -> (TestContext, AppState) {
    let ctx = TestContext::new(db_name).await;
    let config = Config::load(None);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    let state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    (ctx, state)
}
