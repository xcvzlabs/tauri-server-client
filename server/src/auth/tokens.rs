use chrono::{DateTime, Duration as ChronoDuration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

pub fn issue_access_token(
    user_id: &str,
    username: &str,
    role: &str,
    ttl: Duration,
    secret: &str,
) -> Result<String, AppError> {
    let now = Utc::now();
    let expires_at = now + to_chrono_duration(ttl);

    let claims = AccessClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        iat: now.timestamp(),
        exp: expires_at.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| AppError::Internal(format!("failed to issue access token: {err}")))
}

pub fn verify_access_token(token: &str, secret: &str) -> Result<AccessClaims, AppError> {
    decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized)
}

pub struct IssuedRefreshToken {
    pub token: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

pub fn issue_refresh_token(ttl: Duration) -> IssuedRefreshToken {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = to_hex(&bytes);
    let token_hash = hash_refresh_token(&token);
    let expires_at = Utc::now() + to_chrono_duration(ttl);

    IssuedRefreshToken {
        token,
        token_hash,
        expires_at,
    }
}

pub fn hash_refresh_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    to_hex(&digest)
}

fn to_chrono_duration(ttl: Duration) -> ChronoDuration {
    ChronoDuration::from_std(ttl).unwrap_or_else(|_| ChronoDuration::minutes(15))
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-that-is-at-least-32-characters-long";

    #[test]
    fn access_token_round_trip() {
        let token = issue_access_token(
            "user-1",
            "alice",
            "member",
            Duration::from_secs(900),
            SECRET,
        )
        .expect("issue succeeds");

        let claims = verify_access_token(&token, SECRET).expect("verify succeeds");

        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.role, "member");
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn access_token_signed_with_different_secret_is_rejected() {
        let token = issue_access_token(
            "user-1",
            "alice",
            "member",
            Duration::from_secs(900),
            SECRET,
        )
        .expect("issue succeeds");

        let result = verify_access_token(&token, "a-completely-different-secret-value");

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[test]
    fn expired_access_token_is_rejected() {
        let now = Utc::now();
        let claims = AccessClaims {
            sub: "user-1".to_string(),
            username: "alice".to_string(),
            role: "member".to_string(),
            iat: (now - ChronoDuration::minutes(30)).timestamp(),
            exp: (now - ChronoDuration::minutes(15)).timestamp(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .expect("encode succeeds");

        let result = verify_access_token(&token, SECRET);

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[test]
    fn malformed_token_is_rejected() {
        let result = verify_access_token("not-a-jwt", SECRET);

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[test]
    fn refresh_token_hash_is_deterministic() {
        let token = "some-raw-refresh-token";

        assert_eq!(hash_refresh_token(token), hash_refresh_token(token));
    }

    #[test]
    fn issued_refresh_tokens_are_unique_and_hash_matches() {
        let first = issue_refresh_token(Duration::from_secs(60 * 60 * 24 * 7));
        let second = issue_refresh_token(Duration::from_secs(60 * 60 * 24 * 7));

        assert_ne!(first.token, second.token);
        assert_ne!(first.token_hash, second.token_hash);
        assert_eq!(hash_refresh_token(&first.token), first.token_hash);
        assert!(first.expires_at > Utc::now());
    }
}
