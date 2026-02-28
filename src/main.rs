use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sana::state::AppState;
use sana::ws;
use sana::config::Config;
use sana::db;
use sana::logic::nats;
use tower_http::services::ServeDir;

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
    let db_pool = db::connect(&config).await.expect("Failed to connect to database");
    db::run_migrations(&db_pool).await.expect("Failed to run database migrations");
    
    if db::check_connection(&db_pool).await.unwrap_or(false) {
        tracing::info!("Successfully connected to database and ran migrations");
    }

    let app_state = Arc::new(AppState::new(nats_client.clone(), jetstream));

    // Start background tasks
    nats::start_nats_subscriber(app_state.clone()).await;

    let app = Router::new()
        .route("/hello", get(hello_world))
        .route("/ws", get(ws::ws_handler))
        .nest_service("/", ServeDir::new("frontend/dist"))
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn hello_world() -> &'static str {
    "Hello, World!"
}
