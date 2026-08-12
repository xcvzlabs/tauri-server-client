use axum::Json;
use axum::extract::State;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{MeProfile, User, UserProfile};
use crate::state::AppState;

const USER_COLUMNS: &str =
    "id, username, email, password_hash, display_name, role, is_online, last_seen_at, created_at";

pub async fn me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<MeProfile>, AppError> {
    let user = sqlx::query_as::<_, User>(&format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?"))
        .bind(&auth_user.user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found"))?;

    Ok(Json(user.into()))
}

pub async fn list(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<Vec<UserProfile>>, AppError> {
    let users = sqlx::query_as::<_, User>(&format!(
        "SELECT {USER_COLUMNS} FROM users ORDER BY username ASC"
    ))
    .fetch_all(&state.db)
    .await?;

    Ok(Json(users.into_iter().map(UserProfile::from).collect()))
}
