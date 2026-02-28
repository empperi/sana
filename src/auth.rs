use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::{cookie::{Cookie}, SignedCookieJar};
use serde::{Deserialize, Serialize};
use crate::state::{AppState, CombinedState};
use crate::db::users;
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;

pub fn router() -> Router<CombinedState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
        .route("/logout", post(logout))
}

#[derive(Deserialize)]
struct AuthPayload {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    user_id: Uuid,
    username: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

fn internal_error<E>(err: E) -> (StatusCode, Json<ErrorResponse>)
where
    E: std::error::Error,
{
    tracing::error!("Internal error: {}", err);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Internal server error".to_string(),
        }),
    )
}

async fn register(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Json(payload): Json<AuthPayload>,
) -> Result<(SignedCookieJar, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "Username and password are required".to_string() }),
        ));
    }

    let hashed_password = hash(&payload.password, DEFAULT_COST).map_err(internal_error)?;

    let mut tx = state.db_pool.begin().await.map_err(internal_error)?;

    // Check if user exists
    if let Ok(Some(_)) = users::get_user_by_username(&mut tx, &payload.username).await {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse { error: "Username already exists".to_string() }),
        ));
    }

    let user = users::create_user(&mut tx, &payload.username, &hashed_password).await.map_err(internal_error)?;
    tx.commit().await.map_err(internal_error)?;

    let mut cookie = Cookie::new("session_id", user.user_id.to_string());
    cookie.set_path("/");
    let updated_jar = jar.add(cookie);

    Ok((
        updated_jar,
        Json(AuthResponse {
            user_id: user.user_id,
            username: user.username,
        }),
    ))
}

async fn login(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Json(payload): Json<AuthPayload>,
) -> Result<(SignedCookieJar, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    let mut tx = state.db_pool.begin().await.map_err(internal_error)?;

    let user = match users::get_user_by_username(&mut tx, &payload.username).await.map_err(internal_error)? {
        Some(u) => u,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse { error: "Invalid username or password".to_string() }),
            ));
        }
    };

    let valid = verify(&payload.password, &user.password).unwrap_or(false);
    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse { error: "Invalid username or password".to_string() }),
        ));
    }

    users::update_last_login(&mut tx, user.user_id).await.map_err(internal_error)?;
    tx.commit().await.map_err(internal_error)?;

    let mut cookie = Cookie::new("session_id", user.user_id.to_string());
    cookie.set_path("/");
    let updated_jar = jar.add(cookie);

    Ok((
        updated_jar,
        Json(AuthResponse {
            user_id: user.user_id,
            username: user.username,
        }),
    ))
}

async fn me(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(cookie) = jar.get("session_id") {
        if let Ok(user_id) = Uuid::parse_str(cookie.value()) {
            let mut tx = state.db_pool.begin().await.map_err(internal_error)?;
            if let Ok(Some(user)) = users::get_user_by_id(&mut tx, user_id).await {
                return Ok(Json(AuthResponse {
                    user_id: user.user_id,
                    username: user.username,
                }));
            }
        }
    }

    Err((
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse { error: "Not logged in".to_string() }),
    ))
}

async fn logout(jar: SignedCookieJar) -> impl IntoResponse {
    jar.remove(Cookie::from("session_id"))
}
