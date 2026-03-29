use std::env;
use std::fs;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Config {
    pub nats_url: String,
    pub database_url: String,
    pub cors_origin: String,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Creates a new Config by loading from defaults, then an optional config file, then environment variables.
    pub fn new() -> Self {
        let args: Vec<String> = env::args().collect();
        let config_path = args.get(1).map(|s| s.as_str());
        Self::load(config_path)
    }

    /// Loads configuration with specific precedence.
    pub fn load(config_path: Option<&str>) -> Self {
        let config_file = config_path
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|content| serde_json::from_str::<Value>(&content).ok());

        let nats_url = Self::get_value("nats_url", "nats://127.0.0.1:4222", &config_file);
        let cors_origin = Self::get_value("cors_origin", "http://localhost:8080", &config_file);
        
        // Prioritize a direct DATABASE_URL environment variable
        let database_url = if let Ok(url) = env::var("DATABASE_URL") {
            url
        } else {
            Self::init_database_url(&config_file)
        };

        Self {
            nats_url,
            database_url,
            cors_origin,
        }
    }

    fn init_database_url(config_file: &Option<Value>) -> String {
        let user = Self::get_value("postgres_user", "sana_user", config_file);
        let password = Self::get_value("postgres_password", "sana_password", config_file);
        let host = Self::get_value("postgres_host", "127.0.0.1", config_file);
        let port = Self::get_value("postgres_port", "5432", config_file);
        let db = Self::get_value("postgres_db", "sana_db", config_file);

        format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, db)
    }

    /// Generic utility to extract configuration value with precedence:
    /// 1. Environment variable (property_name converted to UPPER_CASE)
    /// 2. Configuration file (exact property_name)
    /// 3. Default value
    fn get_value(property_name: &str, default_value: &str, config_file: &Option<Value>) -> String {
        // 1. Environment Variable (Highest Precedence)
        let env_key = property_name.to_uppercase();
        if let Ok(val) = env::var(&env_key) {
            return val;
        }

        // 2. Configuration File
        if let Some(file_json) = config_file {
            if let Some(val) = file_json.get(property_name) {
                if let Some(s) = val.as_str() {
                    return s.to_string();
                }
                if let Some(n) = val.as_i64() {
                    return n.to_string();
                }
                if let Some(n) = val.as_u64() {
                    return n.to_string();
                }
            }
        }

        // 3. Default Value
        default_value.to_string()
    }
}
