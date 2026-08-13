use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Query, State};
use axum::response::IntoResponse;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::net::SocketAddr;

use crate::auth::tokens;
use crate::error::AppError;
use crate::presence::PresenceEvent;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PresenceQuery {
    token: String,
}

pub async fn ws_handler(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(query): Query<PresenceQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    let claims = tokens::verify_access_token(&query.token, &state.config.jwt_secret)?;

    let is_disabled: Option<bool> =
        sqlx::query_scalar("SELECT is_disabled FROM users WHERE id = ?")
            .bind(&claims.sub)
            .fetch_optional(&state.db)
            .await
            .map_err(AppError::Database)?;

    match is_disabled {
        Some(true) => return Err(AppError::AccountDisabled),
        Some(false) => {}
        None => return Err(AppError::Unauthorized),
    }

    Ok(ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            state,
            claims.sub,
            claims.username,
            addr.ip().to_string(),
        )
    }))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    user_id: String,
    username: String,
    ip_address: String,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut events = state.presence.subscribe();

    let became_online = state.presence.connect(&user_id).await;
    if became_online {
        mark_online_in_db(&state, &user_id, &ip_address).await;
        state.presence.broadcast(PresenceEvent::UserOnline {
            user_id: user_id.clone(),
            username: username.clone(),
        });
    }

    if let Ok(snapshot) = serde_json::to_string(&serde_json::json!({
        "type": "snapshot",
        "online_user_ids": state.presence.online_user_ids().await,
    })) {
        let _ = sender.send(Message::Text(snapshot.into())).await;
    }

    let my_user_id = user_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            if let PresenceEvent::ForceDisconnect { user_id: target_id } = &event {
                if *target_id == my_user_id {
                    break;
                }
                continue;
            }

            let Ok(payload) = serde_json::to_string(&event) else {
                continue;
            };
            if sender.send(Message::Text(payload.into())).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            if matches!(message, Message::Close(_)) {
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    let became_offline = state.presence.disconnect(&user_id).await;
    if became_offline {
        mark_offline_in_db(&state, &user_id).await;
        state
            .presence
            .broadcast(PresenceEvent::UserOffline { user_id, username });
    }
}

async fn mark_online_in_db(state: &AppState, user_id: &str, ip_address: &str) {
    if let Err(err) =
        sqlx::query("UPDATE users SET is_online = 1, ip_address = ?, updated_at = ? WHERE id = ?")
            .bind(ip_address)
            .bind(Utc::now())
            .bind(user_id)
            .execute(&state.db)
            .await
    {
        tracing::error!(error = %err, user_id, "failed to mark user online");
    }
}

async fn mark_offline_in_db(state: &AppState, user_id: &str) {
    let now = Utc::now();
    if let Err(err) =
        sqlx::query("UPDATE users SET is_online = 0, last_seen_at = ?, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(now)
            .bind(user_id)
            .execute(&state.db)
            .await
    {
        tracing::error!(error = %err, user_id, "failed to mark user offline");
    }
}
