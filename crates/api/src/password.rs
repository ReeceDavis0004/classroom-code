use std::sync::LazyLock;

use argon2::{Argon2, PasswordHasher, PasswordVerifier};

use crate::error::ApiError;

const MIN_CHARS: usize = 12;
const MAX_CHARS: usize = 1024;

static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
    Argon2::default()
        .hash_password(crate::token::random(32).as_bytes())
        .expect("argon2 hash failed")
        .to_string()
});

pub fn check(field: &'static str, password: &str) -> Result<(), ApiError> {
    let n = password.chars().count();
    if n < MIN_CHARS {
        return Err(ApiError::field(
            field,
            "too_short",
            format!("Use at least {MIN_CHARS} characters"),
        ));
    }
    if n > MAX_CHARS {
        return Err(ApiError::field(
            field,
            "too_long",
            format!("Use at most {MAX_CHARS} characters"),
        ));
    }
    Ok(())
}

pub async fn hash(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|h| h.to_string())
            .map_err(|e| ApiError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
}

/// With no stored hash, a dummy hash is checked so unknown accounts take as long as known ones.
pub async fn verify(password: String, stored: Option<String>) -> bool {
    tokio::task::spawn_blocking(move || {
        let known = stored.is_some();
        let hash = stored.unwrap_or_else(|| DUMMY_HASH.clone());
        let ok = Argon2::default()
            .verify_password(password.as_bytes(), hash.as_str())
            .is_ok();
        known && ok
    })
    .await
    .unwrap_or(false)
}
