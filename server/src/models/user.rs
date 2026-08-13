use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub role: String,
    pub is_online: bool,
    pub is_disabled: bool,
    pub ip_address: Option<String>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub is_online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeProfile {
    pub id: String,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub is_online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminUserProfile {
    pub id: String,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub is_online: bool,
    pub is_disabled: bool,
    pub ip_address: Option<String>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            role: user.role,
            is_online: user.is_online,
            last_seen_at: user.last_seen_at,
        }
    }
}

impl From<User> for MeProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            role: user.role,
            is_online: user.is_online,
            last_seen_at: user.last_seen_at,
            created_at: user.created_at,
        }
    }
}

impl From<User> for AdminUserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            role: user.role,
            is_online: user.is_online,
            is_disabled: user.is_disabled,
            ip_address: user.ip_address,
            last_seen_at: user.last_seen_at,
            created_at: user.created_at,
        }
    }
}
