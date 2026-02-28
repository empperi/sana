use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sana::state::{AppState, CombinedState};
use sana::ws;
use sana::auth;
use sana::config::Config;
use sana::db;
use sana::logic::nats;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::cors::{CorsLayer};
use axum::http::{HeaderValue, Method};
use axum_extra::extract::cookie::Key;

#[tokio::main]
async fn main() {
    // Load .env if present
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sana=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::new();

    // Connect to NATS
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    let jetstream = async_nats::jetstream::new(nats_client.clone());

    // Create or get the stream
    let stream_config = async_nats::jetstream::stream::Config {
        name: "SANA".to_string(),
        subjects: vec!["topic.>".to_string()],
        ..Default::default()
    };
    
    let _ = jetstream.get_or_create_stream(stream_config).await.unwrap();

    // Connect to Database
    let mut db_pool = None;
    for i in 0..10 {
        match db::connect(&config).await {
            Ok(pool) => {
                db_pool = Some(pool);
                break;
            }
            Err(e) => {
                tracing::warn!("Failed to connect to database (attempt {}): {}. Retrying in 2s...", i + 1, e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
    let db_pool = db_pool.expect("Failed to connect to database after retries");
    db::run_migrations(&db_pool).await.expect("Failed to run database migrations");
    
    if db::check_connection(&db_pool).await.unwrap_or(false) {
        tracing::info!("Successfully connected to database and ran migrations");
    }

    let cookie_key = match env::var("COOKIE_KEY") {
        Ok(key) => {
            if let Ok(bytes) = hex::decode(&key) {
                if bytes.len() < 64 {
                    tracing::warn!("COOKIE_KEY length is {} bytes, but at least 64 bytes are required. Generating new one", bytes.len());
                    Key::generate()
                } else {
                    Key::from(&bytes)
                }
            } else {
                tracing::warn!("Invalid COOKIE_KEY hex, generating new one");
                Key::generate()
            }
        }
        Err(_) => Key::generate(),
    };

    let app_state = AppState::new(nats_client.clone(), jetstream, db_pool);
    let combined_state = CombinedState {
        app: app_state.clone(),
        cookie_key,
    };

    // Start background tasks
    nats::start_nats_subscriber(app_state.clone()).await;
    nats::start_postgres_archiver(app_state.clone()).await;

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:8080".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_credentials(true);

    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .nest("/api/auth", auth::router())
        .nest_service("/", 
            ServeDir::new("frontend/dist")
                .not_found_service(ServeFile::new("frontend/dist/index.html"))
        )
        .layer(cors)
        .with_state(combined_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
