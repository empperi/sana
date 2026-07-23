use sana::db::sessions;
use sana::db::users;
use crate::db::common::TestContext;
use chrono::{Utc, Duration};
use uuid::Uuid;

#[tokio::test]
async fn test_create_and_get_session() {
    let ctx = TestContext::new("sana_test_session_create").await;
    let pool = &ctx.pool;

    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, "session_user", "pass").await.unwrap();
    let expires_at = Utc::now() + Duration::days(30);
    let session = sessions::create_session(&mut tx, user.id, expires_at).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(session.user_id, user.id);
    assert!(session.expires_at > Utc::now());

    let mut tx_fetch = pool.begin().await.unwrap();
    let fetched = sessions::get_valid_session(&mut tx_fetch, session.id).await.unwrap();
    tx_fetch.commit().await.unwrap();

    assert!(fetched.is_some());
    let fetched_session = fetched.unwrap();
    assert_eq!(fetched_session.id, session.id);
    assert_eq!(fetched_session.user_id, user.id);
}

#[tokio::test]
async fn test_get_unknown_session() {
    let ctx = TestContext::new("sana_test_session_unknown").await;
    let pool = &ctx.pool;

    let mut tx_fetch = pool.begin().await.unwrap();
    let fetched = sessions::get_valid_session(&mut tx_fetch, Uuid::new_v4()).await.unwrap();
    tx_fetch.commit().await.unwrap();

    assert!(fetched.is_none());
}

#[tokio::test]
async fn test_get_expired_session() {
    let ctx = TestContext::new("sana_test_session_expired").await;
    let pool = &ctx.pool;

    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, "expired_user", "pass").await.unwrap();
    let past_expiry = Utc::now() - Duration::hours(1);
    let session = sessions::create_session(&mut tx, user.id, past_expiry).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx_fetch = pool.begin().await.unwrap();
    let fetched = sessions::get_valid_session(&mut tx_fetch, session.id).await.unwrap();
    tx_fetch.commit().await.unwrap();

    assert!(fetched.is_none());
}

#[tokio::test]
async fn test_delete_session() {
    let ctx = TestContext::new("sana_test_session_delete").await;
    let pool = &ctx.pool;

    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, "delete_session_user", "pass").await.unwrap();
    let expires_at = Utc::now() + Duration::days(30);
    let session = sessions::create_session(&mut tx, user.id, expires_at).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx_del = pool.begin().await.unwrap();
    sessions::delete_session(&mut tx_del, session.id).await.unwrap();
    tx_del.commit().await.unwrap();

    let mut tx_fetch = pool.begin().await.unwrap();
    let fetched = sessions::get_valid_session(&mut tx_fetch, session.id).await.unwrap();
    tx_fetch.commit().await.unwrap();

    assert!(fetched.is_none());
}
