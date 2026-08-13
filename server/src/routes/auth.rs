use axum::Json;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;
use validator::Validate;

use crate::auth::{AuthUser, password, tokens};
use crate::error::AppError;
use crate::models::{MeProfile, RefreshTokenRecord, User};
use crate::state::AppState;

const USER_COLUMNS: &str = "id, username, email, password_hash, display_name, role, is_online, \
                             is_disabled, ip_address, last_seen_at, created_at";

#[derive(Debug, Deserialize, Validate)]
pub struct SignUpRequest {
    #[validate(length(min = 3, max = 32, message = "must be between 3 and 32 characters"))]
    pub username: String,
    #[validate(email(message = "must be a valid email address"))]
    pub email: String,
    #[validate(length(min = 8, max = 128, message = "must be at least 8 characters"))]
    pub password: String,
    #[validate(length(min = 1, max = 64, message = "must not be empty"))]
    pub display_name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SignInRequest {
    #[validate(length(min = 1, message = "is required"))]
    pub username_or_email: String,
    #[validate(length(min = 1, message = "is required"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RefreshRequest {
    #[validate(length(min = 1, message = "is required"))]
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub user: MeProfile,
}

pub async fn sign_up(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<SignUpRequest>,
) -> Result<(StatusCode, Json<TokenResponse>), AppError> {
    payload.validate()?;

    let username = payload.username.trim().to_lowercase();
    let email = payload.email.trim().to_lowercase();
    let display_name = payload.display_name.trim().to_string();
    let user_id = Uuid::new_v4().to_string();
    let ip_address = addr.ip().to_string();

    let password_hash = password::hash_password_blocking(payload.password).await?;

    let existing_user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    let role = if existing_user_count == 0 {
        "admin"
    } else {
        "member"
    };

    let insert_result = sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, display_name, role, ip_address) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .bind(&display_name)
    .bind(role)
    .bind(&ip_address)
    .execute(&state.db)
    .await;

    if let Err(sqlx::Error::Database(db_err)) = &insert_result
        && db_err.is_unique_violation()
    {
        return Err(AppError::Conflict(
            "username or email is already taken".to_string(),
        ));
    }
    insert_result?;

    let user = fetch_user_by_id(&state, &user_id).await?;
    let tokens = issue_token_pair(&state, &user).await?;

    Ok((StatusCode::CREATED, Json(tokens)))
}

pub async fn sign_in(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<SignInRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    payload.validate()?;

    let identifier = payload.username_or_email.trim().to_lowercase();

    let user = sqlx::query_as::<_, User>(&format!(
        "SELECT {USER_COLUMNS} FROM users WHERE username = ? OR email = ?"
    ))
    .bind(&identifier)
    .bind(&identifier)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::InvalidCredentials)?;

    let password_hash = user.password_hash.clone();
    let is_valid = password::verify_password_blocking(payload.password, password_hash).await?;
    if !is_valid {
        return Err(AppError::InvalidCredentials);
    }

    if user.is_disabled {
        return Err(AppError::AccountDisabled);
    }

    let ip_address = addr.ip().to_string();
    sqlx::query("UPDATE users SET ip_address = ?, updated_at = ? WHERE id = ?")
        .bind(&ip_address)
        .bind(Utc::now())
        .bind(&user.id)
        .execute(&state.db)
        .await?;

    let tokens = issue_token_pair(&state, &user).await?;
    Ok(Json(tokens))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    payload.validate()?;

    let token_hash = tokens::hash_refresh_token(&payload.refresh_token);

    let record = sqlx::query_as::<_, RefreshTokenRecord>(
        "SELECT id, user_id, expires_at, revoked_at FROM refresh_tokens WHERE token_hash = ?",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if record.revoked_at.is_some() || record.expires_at < Utc::now() {
        return Err(AppError::Unauthorized);
    }

    let user = fetch_user_by_id(&state, &record.user_id).await?;

    if user.is_disabled {
        return Err(AppError::AccountDisabled);
    }

    sqlx::query("UPDATE refresh_tokens SET revoked_at = ? WHERE id = ?")
        .bind(Utc::now())
        .bind(&record.id)
        .execute(&state.db)
        .await?;

    let tokens = issue_token_pair(&state, &user).await?;
    Ok(Json(tokens))
}

pub async fn sign_out(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<RefreshRequest>,
) -> Result<StatusCode, AppError> {
    payload.validate()?;

    let token_hash = tokens::hash_refresh_token(&payload.refresh_token);

    let result = sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = ? WHERE token_hash = ? AND user_id = ? AND revoked_at IS NULL",
    )
    .bind(Utc::now())
    .bind(&token_hash)
    .bind(&auth_user.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::Unauthorized);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn issue_token_pair(state: &AppState, user: &User) -> Result<TokenResponse, AppError> {
    let access_token = tokens::issue_access_token(
        &user.id,
        &user.username,
        &user.role,
        state.config.access_token_ttl,
        &state.config.jwt_secret,
    )?;
    let issued_refresh = tokens::issue_refresh_token(state.config.refresh_token_ttl);

    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at) VALUES (?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user.id)
    .bind(&issued_refresh.token_hash)
    .bind(issued_refresh.expires_at)
    .execute(&state.db)
    .await?;

    Ok(TokenResponse {
        access_token,
        refresh_token: issued_refresh.token,
        token_type: "Bearer",
        expires_in: state.config.access_token_ttl.as_secs() as i64,
        user: MeProfile::from(user.clone()),
    })
}

async fn fetch_user_by_id(state: &AppState, user_id: &str) -> Result<User, AppError> {
    sqlx::query_as::<_, User>(&format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?"))
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found"))
}
