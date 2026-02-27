use sana::config::Config;
use sana::db;

#[tokio::test]
async fn test_db_connection() {
    // Load .env if present
    let _ = dotenvy::dotenv();
    
    let config = Config::new();
    
    // Attempt to connect
    let pool = db::connect(&config).await.expect("Failed to connect to the database");
    
    // Check connection
    let is_connected = db::check_connection(&pool).await.expect("Failed to check connection");
    
    assert!(is_connected);
}
