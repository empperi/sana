use std::net::SocketAddr;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sana::state::{AppState, CombinedState};
use sana::config::Config;
use sana::db;
use sana::logic::{nats, archiver};
use uuid::Uuid;
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

    tracing::info!("Starting Sana backend...");

    let config = Config::new();

    // Connect to NATS
    tracing::info!("Connecting to NATS at {}...", config.nats_url);
    let nats_client = async_nats::connect(&config.nats_url).await.unwrap();
    tracing::info!("Connected to NATS");
    
    let jetstream = async_nats::jetstream::new(nats_client.clone());

    // Create or get the stream
    tracing::info!("Initializing JetStream...");
    let stream_config = async_nats::jetstream::stream::Config {
        name: "SANA".to_string(),
        subjects: vec!["topic.>".to_string()],
        ..Default::default()
    };
    
    let _ = jetstream.get_or_create_stream(stream_config).await.unwrap();
    tracing::info!("JetStream initialized");

    // Connect to Database
    tracing::info!("Connecting to database...");
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
        
        // Ensure General channel exists
        let mut tx = db_pool.begin().await.unwrap();
        let general_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let general_channel = sana::db::channels::Channel {
            id: general_id,
            name: "General".to_string(),
            is_private: false,
            created_at: chrono::Utc::now(),
        };
        sana::db::channels::insert_channel(&mut tx, &general_channel).await.unwrap();
        tx.commit().await.unwrap();
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
    archiver::start(app_state.clone()).await;

    let app = sana::router::create_router(combined_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
