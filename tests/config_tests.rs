use sana::config::Config;
use std::env;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_config_loading() {
    // 1. Test Defaults
    {
        env::remove_var("NATS_URL");
        env::remove_var("POSTGRES_USER");
        
        let config = Config::load(None);
        
        assert_eq!(config.nats_url, "nats://localhost:4222");
        assert!(config.database_url.contains("sana_user"));
        assert!(config.database_url.contains("localhost:5432/sana_db"));
    }

    // 2. Test File Override
    {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"nats_url": "nats://file-host:4222", "postgres_user": "file_user"}}"#).unwrap();
        let path = file.path().to_str().unwrap();

        env::remove_var("NATS_URL");
        env::remove_var("POSTGRES_USER");

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
    }
}
