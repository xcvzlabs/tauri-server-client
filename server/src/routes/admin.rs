use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::Utc;
use serde::Deserialize;
use std::borrow::Cow;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::auth::{AdminUser, password};
use crate::error::AppError;
use crate::models::{AdminUserProfile, User};
use crate::presence::PresenceEvent;
use crate::state::AppState;

const USER_COLUMNS: &str = "id, username, email, password_hash, display_name, role, is_online, \
                             is_disabled, ip_address, last_seen_at, created_at";

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRoleRequest {
    #[validate(custom(function = "validate_role"))]
    pub role: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AddUserRequest {
    #[validate(length(min = 3, max = 32, message = "must be between 3 and 32 characters"))]
    pub username: String,
    #[validate(email(message = "must be a valid email address"))]
    pub email: String,
    #[validate(length(min = 8, max = 128, message = "must be at least 8 characters"))]
    pub password: String,
    #[validate(length(min = 1, max = 64, message = "must not be empty"))]
    pub display_name: String,
    #[validate(custom(function = "validate_role"))]
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct SetDisabledRequest {
    pub is_disabled: bool,
}

fn validate_role(role: &str) -> Result<(), ValidationError> {
    if role == "admin" || role == "member" {
        Ok(())
    } else {
        let mut error = ValidationError::new("invalid_role");
        error.message = Some(Cow::Borrowed("must be \"admin\" or \"member\""));
        Err(error)
    }
}

pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<AdminUserProfile>>, AppError> {
    let users = sqlx::query_as::<_, User>(&format!(
        "SELECT {USER_COLUMNS} FROM users ORDER BY username ASC"
    ))
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        users.into_iter().map(AdminUserProfile::from).collect(),
    ))
}

pub async fn add_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(payload): Json<AddUserRequest>,
) -> Result<(StatusCode, Json<AdminUserProfile>), AppError> {
    payload.validate()?;

    let username = payload.username.trim().to_lowercase();
    let email = payload.email.trim().to_lowercase();
    let display_name = payload.display_name.trim().to_string();
    let user_id = Uuid::new_v4().to_string();

    let password_hash = password::hash_password_blocking(payload.password).await?;

    let insert_result = sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, display_name, role) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&user_id)
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .bind(&display_name)
    .bind(&payload.role)
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

    let user = sqlx::query_as::<_, User>(&format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?"))
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found"))?;

    Ok((StatusCode::CREATED, Json(AdminUserProfile::from(user))))
}

pub async fn update_role(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateRoleRequest>,
) -> Result<StatusCode, AppError> {
    payload.validate()?;

    if user_id == admin.user_id {
        return Err(AppError::Conflict(
            "cannot change your own role".to_string(),
        ));
    }

    let result = sqlx::query("UPDATE users SET role = ?, updated_at = ? WHERE id = ?")
        .bind(&payload.role)
        .bind(Utc::now())
        .bind(&user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("user not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn set_disabled(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(user_id): Path<String>,
    Json(payload): Json<SetDisabledRequest>,
) -> Result<StatusCode, AppError> {
    if user_id == admin.user_id {
        return Err(AppError::Conflict(
            "cannot disable your own account".to_string(),
        ));
    }

    let result = sqlx::query("UPDATE users SET is_disabled = ?, updated_at = ? WHERE id = ?")
        .bind(payload.is_disabled)
        .bind(Utc::now())
        .bind(&user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("user not found"));
    }

    if payload.is_disabled {
        // Revoke every outstanding refresh token so /auth/refresh stops
        // working, and kick any live WebSocket connection now rather than
        // waiting for the access token to expire.
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL",
        )
        .bind(Utc::now())
        .bind(&user_id)
        .execute(&state.db)
        .await?;

        state
            .presence
            .broadcast(PresenceEvent::ForceDisconnect { user_id });
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(user_id): Path<String>,
) -> Result<StatusCode, AppError> {
    if user_id == admin.user_id {
        return Err(AppError::Conflict(
            "cannot remove your own account".to_string(),
        ));
    }

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("user not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
