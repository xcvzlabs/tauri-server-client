use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PresenceEvent {
    UserOnline { user_id: String, username: String },
    UserOffline { user_id: String, username: String },
}

#[derive(Clone)]
pub struct PresenceRegistry {
    connections: Arc<RwLock<HashMap<String, usize>>>,
    events: broadcast::Sender<PresenceEvent>,
}

impl PresenceRegistry {
    pub fn new() -> Self {
        let (events, _receiver) = broadcast::channel(1024);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            events,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PresenceEvent> {
        self.events.subscribe()
    }

    pub async fn connect(&self, user_id: &str) -> bool {
        let mut connections = self.connections.write().await;
        let count = connections.entry(user_id.to_string()).or_insert(0);
        *count += 1;
        *count == 1
    }

    pub async fn disconnect(&self, user_id: &str) -> bool {
        let mut connections = self.connections.write().await;
        match connections.get_mut(user_id) {
            Some(count) if *count > 1 => {
                *count -= 1;
                false
            }
            Some(_) => {
                connections.remove(user_id);
                true
            }
            None => false,
        }
    }

    pub async fn online_user_ids(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }

    pub fn broadcast(&self, event: PresenceEvent) {
        let _ = self.events.send(event);
    }
}

impl Default for PresenceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn first_connection_transitions_user_online() {
        let registry = PresenceRegistry::new();

        let became_online = registry.connect("user-1").await;

        assert!(became_online);
        assert_eq!(registry.online_user_ids().await, vec!["user-1".to_string()]);
    }

    #[tokio::test]
    async fn additional_connections_from_same_user_do_not_reflip_online() {
        let registry = PresenceRegistry::new();

        assert!(registry.connect("user-1").await);
        assert!(
            !registry.connect("user-1").await,
            "second connection is not a new transition"
        );
    }

    #[tokio::test]
    async fn user_stays_online_until_last_connection_drops() {
        let registry = PresenceRegistry::new();
        registry.connect("user-1").await;
        registry.connect("user-1").await;

        let became_offline = registry.disconnect("user-1").await;

        assert!(
            !became_offline,
            "one of two connections dropping should not go offline"
        );
        
        assert_eq!(registry.online_user_ids().await, vec!["user-1".to_string()]);

        let became_offline = registry.disconnect("user-1").await;

        assert!(
            became_offline,
            "the last connection dropping should go offline"
        );

        assert!(registry.online_user_ids().await.is_empty());
    }

    #[tokio::test]
    async fn disconnecting_a_user_with_no_connections_is_a_noop() {
        let registry = PresenceRegistry::new();
        let became_offline = registry.disconnect("never-connected").await;

        assert!(!became_offline);
    }

    #[tokio::test]
    async fn broadcast_events_reach_subscribers() {
        let registry = PresenceRegistry::new();
        let mut subscriber = registry.subscribe();

        registry.broadcast(PresenceEvent::UserOnline {
            user_id: "user-1".to_string(),
            username: "alice".to_string(),
        });

        let event = subscriber.recv().await.expect("event delivered");
        match event {
            PresenceEvent::UserOnline { user_id, username } => {
                assert_eq!(user_id, "user-1");
                assert_eq!(username, "alice");
            }
            PresenceEvent::UserOffline { .. } => panic!("expected UserOnline event"),
        }
    }
}
