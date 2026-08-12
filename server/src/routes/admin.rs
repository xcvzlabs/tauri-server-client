use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::Utc;
use serde::Deserialize;
use std::borrow::Cow;
use validator::{Validate, ValidationError};

use crate::auth::AdminUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRoleRequest {
    #[validate(custom(function = "validate_role"))]
    pub role: String,
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
