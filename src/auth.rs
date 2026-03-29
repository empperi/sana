use axum::{
    extract::{State, FromRef},
    http::{StatusCode, request::Parts},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
    async_trait,
    extract::FromRequestParts,
};
use axum_extra::extract::{cookie::{Cookie, Key}, SignedCookieJar};
use serde::{Deserialize, Serialize};
use crate::state::{AppState, CombinedState};
use crate::db::users;
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;

pub struct UserSession {
    pub user_id: Uuid,
}

#[async_trait]
impl<S> FromRequestParts<S> for UserSession
where
    S: Send + Sync,
    AppState: FromRef<S>,
    Key: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar: SignedCookieJar<Key> = SignedCookieJar::from_request_parts(parts, state).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(cookie) = jar.get("session_id") {
            if let Ok(user_id) = Uuid::parse_str(cookie.value()) {
                let app_state = AppState::from_ref(state);
                if app_state.validate_session(user_id).await {
                    return Ok(UserSession { user_id });
                }
            }
        }

        Err(StatusCode::UNAUTHORIZED)
    }
}

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

fn set_session_cookie(jar: SignedCookieJar, user_id: Uuid) -> SignedCookieJar {
    let mut cookie = Cookie::new("session_id", user_id.to_string());
    cookie.set_path("/");
    jar.add(cookie)
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

    if let Ok(Some(_)) = users::get_user_by_username(&mut tx, &payload.username).await {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse { error: "Username already exists".to_string() }),
        ));
    }

    let user = users::create_user(&mut tx, &payload.username, &hashed_password).await.map_err(internal_error)?;
    
    let general_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    crate::db::channels::join_channel(&mut tx, user.id, general_id).await.map_err(internal_error)?;

    tx.commit().await.map_err(internal_error)?;

    let updated_jar = set_session_cookie(jar, user.id);

    Ok((
        updated_jar,
        Json(AuthResponse {
            user_id: user.id,
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

    users::update_last_login(&mut tx, user.id).await.map_err(internal_error)?;
    tx.commit().await.map_err(internal_error)?;

    let updated_jar = set_session_cookie(jar, user.id);

    Ok((
        updated_jar,
        Json(AuthResponse {
            user_id: user.id,
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
            if state.validate_session(user_id).await {
                // We still need the username for the response, but now this only happens 
                // for the /me endpoint, not every UserSession extraction
                let mut tx = state.db_pool.begin().await.map_err(internal_error)?;
                if let Ok(Some(user)) = users::get_user_by_id(&mut tx, user_id).await {
                    return Ok(Json(AuthResponse {
                        user_id: user.id,
                        username: user.username,
                    }));
                }
            }
        }
    }

    Err((
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse { error: "Not logged in".to_string() }),
    ))
}

async fn logout(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> impl IntoResponse {
    if let Some(cookie) = jar.get("session_id") {
        if let Ok(user_id) = Uuid::parse_str(cookie.value()) {
            state.invalidate_session(user_id);
        }
    }

    let mut cookie = Cookie::from("session_id");
    cookie.set_path("/");
    jar.remove(cookie)
}
