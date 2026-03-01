use sana::db::users;
use crate::db::common::TestContext;

#[tokio::test]
async fn test_user_creation_and_fetching() {
    let ctx = TestContext::new("sana_test_user_create").await;
    let pool = &ctx.pool;

    let username = "user_one";
    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, username, "pass1").await.unwrap();
    tx.commit().await.unwrap();

    let mut tx_fetch = pool.begin().await.unwrap();
    let fetched = users::get_user_by_id(&mut tx_fetch, user.id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().username, username);
}

#[tokio::test]
async fn test_user_by_username() {
    let ctx = TestContext::new("sana_test_user_by_name").await;
    let pool = &ctx.pool;

    let username = "user_two";
    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, username, "pass2").await.unwrap();
    tx.commit().await.unwrap();

    let mut tx_fetch = pool.begin().await.unwrap();
    let fetched = users::get_user_by_username(&mut tx_fetch, username).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, user.id);
}

#[tokio::test]
async fn test_update_last_login() {
    let ctx = TestContext::new("sana_test_user_login").await;
    let pool = &ctx.pool;

    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, "login_user", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let mut tx_update = pool.begin().await.unwrap();
    users::update_last_login(&mut tx_update, user.id).await.unwrap();
    tx_update.commit().await.unwrap();

    let mut tx_fetch = pool.begin().await.unwrap();
    let updated = users::get_user_by_id(&mut tx_fetch, user.id).await.unwrap().unwrap();
    assert!(updated.last_login.is_some());
}

#[tokio::test]
async fn test_user_deletion() {
    let ctx = TestContext::new("sana_test_user_delete").await;
    let pool = &ctx.pool;

    let mut tx = pool.begin().await.unwrap();
    let user = users::create_user(&mut tx, "delete_me", "pass").await.unwrap();
    tx.commit().await.unwrap();

    let mut tx_del = pool.begin().await.unwrap();
    users::delete_user(&mut tx_del, user.id).await.unwrap();
    tx_del.commit().await.unwrap();

    let mut tx_fetch = pool.begin().await.unwrap();
    let fetched = users::get_user_by_id(&mut tx_fetch, user.id).await.unwrap();
    assert!(fetched.is_none());
}
