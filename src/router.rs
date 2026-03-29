use axum::{
    routing::get,
    Router,
};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::cors::CorsLayer;
use axum::http::{HeaderValue, Method};
use crate::state::CombinedState;
use crate::{ws, auth, channels};

pub fn create_router(combined_state: CombinedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(combined_state.config.cors_origin.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/ws", get(ws::ws_handler))
        .nest("/api/auth", auth::router())
        .nest("/api/channels", channels::router())
        .nest_service("/", 
            ServeDir::new("frontend/dist")
                .not_found_service(ServeFile::new("frontend/dist/index.html"))
        )
        .layer(cors)
        .with_state(combined_state)
}
