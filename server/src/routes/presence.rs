use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

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
    Query(query): Query<PresenceQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    let claims = tokens::verify_access_token(&query.token, &state.config.jwt_secret)?;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, claims.sub, claims.username)))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: String, username: String) {
    let (mut sender, mut receiver) = socket.split();
    let mut events = state.presence.subscribe();

    let became_online = state.presence.connect(&user_id).await;
    if became_online {
        mark_online_in_db(&state, &user_id).await;
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

    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
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

async fn mark_online_in_db(state: &AppState, user_id: &str) {
    if let Err(err) = sqlx::query("UPDATE users SET is_online = 1, updated_at = ? WHERE id = ?")
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
