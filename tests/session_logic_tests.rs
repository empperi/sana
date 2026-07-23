#[path = "db/common.rs"]
mod common;

use sana::logic::sessions;
use sana::db::users;
use sana::state::AppState;
use sana::config::Config;
use common::TestContext;
use chrono::{Utc, Duration};
use uuid::Uuid;

#[tokio::test]
async fn test_session_lifecycle() {
    let (ctx, state) = setup_app_state("sana_test_session_logic_lifecycle").await;
    let pool = ctx.pool.clone();

    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, "session_logic_user", "pass").await.unwrap();
    tx.commit().await.unwrap();

    // 1. Start session
    let session_id = sessions::start_session(&pool, user.id).await.unwrap();
    assert_ne!(session_id, Uuid::nil());

    // 2. Validate session
    let valid_user_id = sessions::validate(&state, session_id).await;
    assert_eq!(valid_user_id, Some(user.id));

    // 3. Cache validation (should hit cache)
    let cached_user_id = sessions::validate(&state, session_id).await;
    assert_eq!(cached_user_id, Some(user.id));

    // 4. End session (logout)
    sessions::end_session(&state, session_id).await.unwrap();

    // 5. Validation after logout fails
    let post_logout_user_id = sessions::validate(&state, session_id).await;
    assert_eq!(post_logout_user_id, None);
}

#[tokio::test]
async fn test_validate_random_session_id() {
    let (_ctx, state) = setup_app_state("sana_test_session_logic_random").await;

    let random_id = Uuid::new_v4();
    let res = sessions::validate(&state, random_id).await;
    assert_eq!(res, None);
}

#[tokio::test]
async fn test_session_lifetime_constant() {
    let ctx = TestContext::new("sana_test_session_lifetime").await;
    let pool = ctx.pool.clone();

    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, "lifetime_user", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let before = Utc::now();
    let session_id = sessions::start_session(&pool, user.id).await.unwrap();
    let after = Utc::now();

    let mut tx_fetch = pool.begin().await.unwrap();
    let db_session = sana::db::sessions::get_valid_session(&mut tx_fetch, session_id).await.unwrap().unwrap();
    tx_fetch.commit().await.unwrap();

    let expected_min = before + Duration::days(30) - Duration::seconds(5);
    let expected_max = after + Duration::days(30) + Duration::seconds(5);
    assert!(db_session.expires_at >= expected_min && db_session.expires_at <= expected_max);
}

#[tokio::test]
async fn test_validate_lazy_delete_committed() {
    let (ctx, state) = setup_app_state("sana_test_session_lazy_delete_commit").await;
    let pool = ctx.pool.clone();

    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, "expired_user_lazy", "pass").await.unwrap();
    let expired_at = Utc::now() - Duration::hours(1);
    let session = sana::db::sessions::create_session(&mut tx, user.id, expired_at).await.unwrap();
    tx.commit().await.unwrap();

    // Validating an expired session should return None and commit lazy delete
    let val_res = sessions::validate(&state, session.id).await;
    assert_eq!(val_res, None);

    // Verify row was deleted in DB (not rolled back)
    let mut tx_check = pool.begin().await.unwrap();
    let raw_row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM sessions WHERE id = $1")
        .bind(session.id)
        .fetch_optional(&mut *tx_check)
        .await
        .unwrap();
    tx_check.commit().await.unwrap();
    assert!(raw_row.is_none());
}

async fn setup_app_state(db_name: &str) -> (TestContext, AppState) {
    let ctx = TestContext::new(db_name).await;
    let config = Config::new();
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    let state = AppState::new(nats_client, jetstream, ctx.pool.clone());
    (ctx, state)
}
