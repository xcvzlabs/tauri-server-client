use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

use crate::error::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::Hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| AppError::Hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub async fn hash_password_blocking(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|err| AppError::Internal(format!("password hashing task panicked: {err}")))?
}

pub async fn verify_password_blocking(password: String, hash: String) -> Result<bool, AppError> {
    tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|err| AppError::Internal(format!("password verification task panicked: {err}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_round_trip() {
        let hash = hash_password("correcthorsebattery").expect("hash succeeds");

        assert!(verify_password("correcthorsebattery", &hash).expect("verify succeeds"));
    }

    #[test]
    fn verify_rejects_wrong_password() {
        let hash = hash_password("correcthorsebattery").expect("hash succeeds");

        assert!(!verify_password("wrong-password", &hash).expect("verify succeeds"));
    }

    #[test]
    fn same_password_hashed_twice_produces_different_hashes() {
        let first = hash_password("correcthorsebattery").expect("hash succeeds");
        let second = hash_password("correcthorsebattery").expect("hash succeeds");

        assert_ne!(first, second, "each hash must use a fresh random salt");
        assert!(verify_password("correcthorsebattery", &first).expect("verify succeeds"));
        assert!(verify_password("correcthorsebattery", &second).expect("verify succeeds"));
    }

    #[test]
    fn verify_rejects_malformed_hash() {
        let result = verify_password("correcthorsebattery", "not-a-valid-phc-hash");

        assert!(matches!(result, Err(AppError::Hash)));
    }
}
