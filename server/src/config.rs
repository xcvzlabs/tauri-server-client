use std::env;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable `{0}` (code: CONFIG_MISSING_VAR)")]
    Missing(&'static str),
    #[error("invalid value for `{name}` (code: CONFIG_INVALID_VALUE): {reason}")]
    Invalid { name: &'static str, reason: String },
}

#[derive(Debug, Clone)]
pub struct Config {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub access_token_ttl: Duration,
    pub refresh_token_ttl: Duration,
    pub allowed_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_source(|name| env::var(name).ok())
    }

    fn from_source(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let host = parse_var(&get, "SERVER_HOST", "0.0.0.0")?;
        let port = parse_var::<u16>(&get, "SERVER_PORT", "8080")?;
        let database_url = required_var(&get, "DATABASE_URL")?;
        let jwt_secret = required_var(&get, "JWT_SECRET")?;

        if jwt_secret.len() < 32 {
            return Err(ConfigError::Invalid {
                name: "JWT_SECRET",
                reason: "must be at least 32 characters long".to_string(),
            });
        }

        let access_token_ttl_minutes = parse_var::<u64>(&get, "ACCESS_TOKEN_TTL_MINUTES", "15")?;
        let refresh_token_ttl_days = parse_var::<u64>(&get, "REFRESH_TOKEN_TTL_DAYS", "7")?;
        let allowed_origins = get("ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(str::to_string)
            .collect();

        Ok(Self {
            host,
            port,
            database_url,
            jwt_secret,
            access_token_ttl: Duration::from_secs(access_token_ttl_minutes * 60),
            refresh_token_ttl: Duration::from_secs(refresh_token_ttl_days * 24 * 60 * 60),
            allowed_origins,
        })
    }
}

fn required_var(
    get: &impl Fn(&str) -> Option<String>,
    name: &'static str,
) -> Result<String, ConfigError> {
    match get(name) {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(ConfigError::Missing(name)),
    }
}

fn parse_var<T>(
    get: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: &str,
) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = get(name).unwrap_or_else(|| default.to_string());
    
    raw.parse::<T>().map_err(|err| ConfigError::Invalid {
        name,
        reason: err.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn source(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        move |name| map.get(name).cloned()
    }

    const VALID_SECRET: &str = "test-secret-that-is-at-least-32-characters-long";

    #[test]
    fn missing_database_url_is_rejected() {
        let result = Config::from_source(source(&[("JWT_SECRET", VALID_SECRET)]));

        assert!(matches!(result, Err(ConfigError::Missing("DATABASE_URL"))));
    }

    #[test]
    fn missing_jwt_secret_is_rejected() {
        let result = Config::from_source(source(&[("DATABASE_URL", "sqlite://./data/app.sqlite")]));

        assert!(matches!(result, Err(ConfigError::Missing("JWT_SECRET"))));
    }

    #[test]
    fn short_jwt_secret_is_rejected() {
        let result = Config::from_source(source(&[
            ("DATABASE_URL", "sqlite://./data/app.sqlite"),
            ("JWT_SECRET", "too-short"),
        ]));

        assert!(matches!(
            result,
            Err(ConfigError::Invalid {
                name: "JWT_SECRET",
                ..
            })
        ));
    }

    #[test]
    fn invalid_port_is_rejected() {
        let result = Config::from_source(source(&[
            ("DATABASE_URL", "sqlite://./data/app.sqlite"),
            ("JWT_SECRET", VALID_SECRET),
            ("SERVER_PORT", "not-a-port"),
        ]));

        assert!(matches!(
            result,
            Err(ConfigError::Invalid {
                name: "SERVER_PORT",
                ..
            })
        ));
    }

    #[test]
    fn valid_config_parses_with_defaults() {
        let config = Config::from_source(source(&[
            ("DATABASE_URL", "sqlite://./data/app.sqlite"),
            ("JWT_SECRET", VALID_SECRET),
        ]))
        .expect("valid config");

        assert_eq!(config.host, "0.0.0.0".parse::<IpAddr>().unwrap());
        assert_eq!(config.port, 8080);
        assert_eq!(config.access_token_ttl, Duration::from_secs(15 * 60));
        assert_eq!(
            config.refresh_token_ttl,
            Duration::from_secs(7 * 24 * 60 * 60)
        );
        assert!(config.allowed_origins.is_empty());
    }

    #[test]
    fn allowed_origins_are_split_trimmed_and_filtered() {
        let config = Config::from_source(source(&[
            ("DATABASE_URL", "sqlite://./data/app.sqlite"),
            ("JWT_SECRET", VALID_SECRET),
            (
                "ALLOWED_ORIGINS",
                " http://localhost:1420 ,, tauri://localhost ",
            ),
        ]))
        .expect("valid config");

        assert_eq!(
            config.allowed_origins,
            vec!["http://localhost:1420", "tauri://localhost"]
        );
    }

    #[test]
    fn custom_ttls_are_converted_to_seconds() {
        let config = Config::from_source(source(&[
            ("DATABASE_URL", "sqlite://./data/app.sqlite"),
            ("JWT_SECRET", VALID_SECRET),
            ("ACCESS_TOKEN_TTL_MINUTES", "30"),
            ("REFRESH_TOKEN_TTL_DAYS", "1"),
        ]))
        .expect("valid config");

        assert_eq!(config.access_token_ttl, Duration::from_secs(30 * 60));
        assert_eq!(config.refresh_token_ttl, Duration::from_secs(24 * 60 * 60));
    }
}
