use sana::config::Config;
use std::env;
use std::io::Write;
use tempfile::NamedTempFile;

use std::sync::Mutex;

static ENV_MUTEX: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

fn get_env_mutex() -> &'static Mutex<()> {
    ENV_MUTEX.get_or_init(|| Mutex::new(()))
}

#[test]
fn test_cors_origin_default() {
    let _lock = get_env_mutex().lock().unwrap();
    env::remove_var("CORS_ORIGIN");
    let config = Config::load(None);
    assert_eq!(config.cors_origin, "http://localhost:8080");
}

#[test]
fn test_cors_origin_from_env() {
    let _lock = get_env_mutex().lock().unwrap();
    env::set_var("CORS_ORIGIN", "https://example.com");
    let config = Config::load(None);
    assert_eq!(config.cors_origin, "https://example.com");
    env::remove_var("CORS_ORIGIN");
}

#[test]
fn test_cookie_secure_default_and_env() {
    let _lock = get_env_mutex().lock().unwrap();
    env::remove_var("COOKIE_SECURE");
    let config_default = Config::load(None);
    assert!(!config_default.cookie_secure);

    env::set_var("COOKIE_SECURE", "true");
    let config_env = Config::load(None);
    assert!(config_env.cookie_secure);
    env::remove_var("COOKIE_SECURE");
}

#[test]
fn test_config_loading() {
    let _lock = get_env_mutex().lock().unwrap();
    // 1. Test Defaults
    {
        env::remove_var("NATS_URL");
        env::remove_var("POSTGRES_USER");
        env::remove_var("DATABASE_URL");
        
        let config = Config::load(None);
        
        assert_eq!(config.nats_url, "nats://127.0.0.1:4222");
        assert!(config.database_url.contains("sana_user"));
        assert!(config.database_url.contains("127.0.0.1:5432/sana_db"));
    }

    // 2. Test File Override
    {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"nats_url": "nats://file-host:4222", "postgres_user": "file_user"}}"#).unwrap();
        let path = file.path().to_str().unwrap();

        env::remove_var("NATS_URL");
        env::remove_var("POSTGRES_USER");
        env::remove_var("DATABASE_URL");

        let config = Config::load(Some(path));

        assert_eq!(config.nats_url, "nats://file-host:4222");
        assert!(config.database_url.contains("file_user"));
    }

    // 3. Test Precedence (Env Vars over File)
    {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"nats_url": "nats://file-host:4222", "postgres_user": "file_user"}}"#).unwrap();
        let path = file.path().to_str().unwrap();

        env::set_var("NATS_URL", "nats://env-host:4222");
        env::set_var("POSTGRES_USER", "env_user");
        
        let config = Config::load(Some(path));
        
        assert_eq!(config.nats_url, "nats://env-host:4222");
        assert!(config.database_url.contains("env_user"));
        
        env::remove_var("NATS_URL");
        env::remove_var("POSTGRES_USER");
        env::remove_var("DATABASE_URL");
    }
}
