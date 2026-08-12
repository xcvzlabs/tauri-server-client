use sqlx::SqlitePool;
use std::sync::Arc;

use crate::config::Config;
use crate::presence::PresenceRegistry;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub presence: PresenceRegistry,
}
