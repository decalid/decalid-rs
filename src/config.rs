use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Configuration for the Decalid application
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Database connection string
    pub database_url: String,

    /// Server configuration
    pub server: ServerConfig,

    /// Telemetry configuration
    pub telemetry: TelemetryConfig,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Authentication configuration
    pub auth: AuthConfig,
}

impl Config {
    pub fn with_port(self, port: u16) -> Self {
        Self {
            server: ServerConfig {
                port,
                ..self.server
            },
            ..self
        }
    }

    pub fn with_host(self, host: &str) -> Self {
        Self {
            server: ServerConfig {
                host: host.to_string(),
                ..self.server
            },
            ..self
        }
    }

    #[cfg(test)]
    pub(crate) fn test_config() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            server: ServerConfig {
                port: 8080,
                host: "127.0.0.1".to_string(),
            },
            telemetry: TelemetryConfig {
                enable_opentelemetry: false,
                enable_sentry: false,
                sentry_dsn: None,
                log_level: "info".to_string(),
            },
            cache: CacheConfig {
                enable: false,
                ttl_seconds: 3600,
                max_size: 100,
            },
            auth: AuthConfig {
                jwt_secret: "secret".to_string(),
                jwt_expiration_seconds: 3600,
                pairing_code_expiration_seconds: 300,
                webauthn: WebauthnConfig {
                    rp_id: "test.decalid.com".to_string(),
                    rp_origin: reqwest::Url::parse("https://127.0.0.1")
                        .expect("Failed to parse RP origin"),
                    rp_name: "Decalid Synthetic Tests".to_string(),
                    additional_allowed_origins: vec![],
                },
            },
        }
    }
}

/// Server configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    /// Port to listen on
    pub port: u16,

    /// Host to bind to
    pub host: String,
}

/// Telemetry configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryConfig {
    /// Enable OpenTelemetry
    pub enable_opentelemetry: bool,

    /// Enable Sentry
    pub enable_sentry: bool,

    /// Sentry DSN
    pub sentry_dsn: Option<String>,

    /// Log level
    pub log_level: String,
}

/// Cache configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheConfig {
    /// Enable caching
    pub enable: bool,

    /// Cache TTL in seconds
    pub ttl_seconds: u64,

    /// Maximum cache size
    pub max_size: usize,
}

/// Authentication configuration
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AuthConfig {
    /// JWT secret
    pub jwt_secret: String,

    /// JWT expiration in seconds
    pub jwt_expiration_seconds: u64,

    /// Pairing code expiration in seconds
    pub pairing_code_expiration_seconds: u64,

    /// WebAuthn configuration
    pub webauthn: WebauthnConfig,
}
impl AuthConfig {
    pub fn with_expiration(&self, expiration: u64) -> AuthConfig {
        Self {
            jwt_expiration_seconds: expiration,
            ..self.clone()
        }
    }
}

/// WebAuthn configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebauthnConfig {
    /// RP ID
    pub rp_id: String,

    /// RP Origin (URL)
    #[serde(with = "url_serde")]
    pub rp_origin: reqwest::Url,

    /// RP name
    pub rp_name: String,

    /// Additional allowed origins
    #[serde(with = "url_serde_vec", default)]
    pub additional_allowed_origins: Vec<reqwest::Url>,
}

// Helper module for URL serialization
mod url_serde {
    use reqwest::Url;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(url: &Url, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(url.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Url, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Url::parse(&s).map_err(serde::de::Error::custom)
    }
}

mod url_serde_vec {
    use reqwest::Url;
    use serde::{self, Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(urls: &[Url], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(urls.iter().map(|url| url.as_str()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Url>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = Vec::<String>::deserialize(deserializer)?;
        s.into_iter()
            .map(|s| Url::parse(&s).map_err(serde::de::Error::custom))
            .collect()
    }
}

impl Default for WebauthnConfig {
    fn default() -> Self {
        Self {
            rp_id: "decalid.com".to_string(),
            rp_origin: reqwest::Url::parse("https://cal.decalid.com")
                .expect("Failed to parse RP origin"),
            rp_name: "Decalid".to_string(),
            additional_allowed_origins: vec![],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "sqlite://./db.sqlite".to_string(),
            server: ServerConfig {
                port: 8080,
                host: "127.0.0.1".to_string(),
            },
            telemetry: TelemetryConfig {
                enable_opentelemetry: false,
                enable_sentry: false,
                sentry_dsn: None,
                log_level: "info".to_string(),
            },
            cache: CacheConfig {
                enable: true,
                ttl_seconds: 3600,
                max_size: 1000,
            },
            auth: AuthConfig {
                jwt_secret: "change_me_in_production".to_string(),
                jwt_expiration_seconds: 86400,        // 24 hours
                pairing_code_expiration_seconds: 300, // 5 minutes
                webauthn: WebauthnConfig::default(),
            },
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config_str = fs::read_to_string(path).context("Failed to read config file")?;

        let config: Config =
            serde_json::from_str(&config_str).context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save configuration to a file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let config_str =
            serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(path, config_str).context("Failed to write config file")?;

        Ok(())
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Config::default();

        if let Ok(database_url) = std::env::var("DECALID_DATABASE_URL") {
            config.database_url = database_url;
        }

        if let Ok(port) = std::env::var("DECALID_SERVER_PORT") {
            if let Ok(port) = port.parse() {
                config.server.port = port;
            }
        }

        if let Ok(host) = std::env::var("DECALID_SERVER_HOST") {
            config.server.host = host;
        }

        if let Ok(jwt_secret) = std::env::var("DECALID_JWT_SECRET") {
            config.auth.jwt_secret = jwt_secret;
        }

        // More environment variables can be added as needed

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.database_url, "sqlite://./db.sqlite");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.host, "127.0.0.1");
    }

    #[test]
    fn test_config_from_env() {
        // Save original environment variables
        let original_db_url = env::var("DECALID_DATABASE_URL").ok();
        let original_port = env::var("DECALID_SERVER_PORT").ok();

        // Set test environment variables
        env::set_var("DECALID_DATABASE_URL", "sqlite://./test.db");
        env::set_var("DECALID_SERVER_PORT", "9090");

        // Load config from environment
        let config = Config::from_env();

        // Verify config values
        assert_eq!(config.database_url, "sqlite://./test.db");
        assert_eq!(config.server.port, 9090);

        // Restore original environment variables
        match original_db_url {
            Some(val) => env::set_var("DECALID_DATABASE_URL", val),
            None => env::remove_var("DECALID_DATABASE_URL"),
        }

        match original_port {
            Some(val) => env::set_var("DECALID_SERVER_PORT", val),
            None => env::remove_var("DECALID_SERVER_PORT"),
        }
    }
}
