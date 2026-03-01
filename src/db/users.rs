use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password: String,
    pub last_login: Option<DateTime<Utc>>,
}

pub async fn create_user(tx: &mut Transaction<'_, Postgres>, username: &str, password_hash: &str) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password) VALUES ($1, $2) RETURNING id, username, password, last_login"
    )
    .bind(username)
    .bind(password_hash)
    .fetch_one(&mut **tx)
    .await?;

    Ok(user)
}

pub async fn get_user_by_id(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, password, last_login FROM users WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(user)
}

pub async fn get_user_by_username(tx: &mut Transaction<'_, Postgres>, username: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, password, last_login FROM users WHERE username = $1"
    )
    .bind(username)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(user)
}

pub async fn update_last_login(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET last_login = $1 WHERE id = $2")
        .bind(Utc::now())
        .bind(id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

pub async fn delete_user(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}
