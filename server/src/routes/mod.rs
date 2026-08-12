pub mod admin;
pub mod auth;
pub mod health;
pub mod presence;
pub mod users;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let auth_rate_limit = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(5)
        .finish()
        .expect("valid rate limiter configuration");

    let auth_routes = Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/refresh", post(auth::refresh))
        .route("/logout", post(auth::logout))
        .layer(GovernorLayer::new(Arc::new(auth_rate_limit)));

    let user_routes = Router::new()
        .route("/me", get(users::me))
        .route("/", get(users::list));

    let admin_routes = Router::new()
        .route("/users/{user_id}/role", patch(admin::update_role))
        .route("/users/{user_id}", delete(admin::remove_user));

    let api_v1 = Router::new()
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/admin", admin_routes);

    Router::new()
        .route("/health", get(health::health))
        .nest("/api/v1", api_v1)
        .route("/ws/presence", get(presence::ws_handler))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(10),
                ))
                .layer(CatchPanicLayer::new())
                .layer(build_cors_layer(&state.config.allowed_origins)),
        )
        .with_state(state)
}

fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    if allowed_origins.is_empty() {
        return CorsLayer::permissive();
    }

    let origins: Vec<_> = allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(Any)
        .allow_headers(Any)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::extract::ConnectInfo;
    use axum::http::Request;
    use serde_json::{Value, json};
    use std::net::SocketAddr;
    use tower::ServiceExt;

    use crate::config::Config;
    use crate::presence::PresenceRegistry;

    async fn test_state() -> (AppState, tempfile::TempPath) {
        let temp = tempfile::NamedTempFile::new().expect("create temp db file");
        let path = temp.into_temp_path();
        let database_url = format!("sqlite://{}", path.display());
        let db = crate::db::connect(&database_url)
            .await
            .expect("connect test db");

        let config = Config {
            host: "127.0.0.1".parse().expect("valid ip"),
            port: 0,
            database_url,
            jwt_secret: "test-secret-that-is-at-least-32-characters-long".to_string(),
            access_token_ttl: Duration::from_secs(900),
            refresh_token_ttl: Duration::from_secs(60 * 60 * 24 * 7),
            allowed_origins: Vec::new(),
        };

        (
            AppState {
                db,
                config: Arc::new(config),
                presence: PresenceRegistry::new(),
            },
            path,
        )
    }

    fn with_fake_peer(mut request: Request<Body>) -> Request<Body> {
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
        request
    }

    fn json_request(method: &str, uri: &str, body: &Value) -> Request<Body> {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(body).expect("serialize body"),
            ))
            .expect("build request");
        with_fake_peer(request)
    }

    fn auth_get(uri: &str, access_token: &str) -> Request<Body> {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .header("authorization", format!("Bearer {access_token}"))
            .body(Body::empty())
            .expect("build request");
        with_fake_peer(request)
    }

    fn auth_request(
        method: &str,
        uri: &str,
        access_token: &str,
        body: Option<&Value>,
    ) -> Request<Body> {
        let payload = body.map_or_else(Vec::new, |value| {
            serde_json::to_vec(value).expect("serialize body")
        });
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {access_token}"))
            .header("content-type", "application/json")
            .body(Body::from(payload))
            .expect("build request");
        with_fake_peer(request)
    }

    async fn body_json(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("valid json body")
    }

    fn register_payload(username: &str) -> Value {
        json!({
            "username": username,
            "email": format!("{username}@example.com"),
            "password": "correcthorsebattery",
            "display_name": username,
        })
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let response = app
            .oneshot(with_fake_peer(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn register_then_duplicate_username_is_conflict() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);
        let payload = register_payload("alice");

        let response = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/auth/register", &payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(json_request("POST", "/api/v1/auth/register", &payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn register_rejects_a_password_below_the_minimum_length() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let mut payload = register_payload("bob");
        payload["password"] = json!("short");

        let response = app
            .oneshot(json_request("POST", "/api/v1/auth/register", &payload))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn login_succeeds_then_rejects_a_wrong_password() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        app.clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/register",
                &register_payload("carol"),
            ))
            .await
            .unwrap();

        let login_payload =
            json!({ "username_or_email": "carol", "password": "correcthorsebattery" });
        let response = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/auth/login", &login_payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bad_login = json!({ "username_or_email": "carol", "password": "wrong-password" });
        let response = app
            .oneshot(json_request("POST", "/api/v1/auth/login", &bad_login))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_requires_a_bearer_token() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let response = app
            .oneshot(with_fake_peer(
                Request::builder()
                    .uri("/api/v1/users/me")
                    .body(Body::empty())
                    .unwrap(),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn me_returns_the_authenticated_users_own_profile() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/register",
                &register_payload("dave"),
            ))
            .await
            .unwrap();
        let tokens = body_json(response).await;
        let access_token = tokens["access_token"]
            .as_str()
            .expect("access token present");

        let response = app
            .oneshot(auth_get("/api/v1/users/me", access_token))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let profile = body_json(response).await;
        assert_eq!(profile["username"], "dave");
        assert_eq!(profile["email"], "dave@example.com");
    }

    #[tokio::test]
    async fn users_list_omits_email_addresses() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/register",
                &register_payload("erin"),
            ))
            .await
            .unwrap();
        let tokens = body_json(response).await;
        let access_token = tokens["access_token"]
            .as_str()
            .expect("access token present")
            .to_string();

        let response = app
            .oneshot(auth_get("/api/v1/users", &access_token))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let users = body_json(response).await;
        let list = users.as_array().expect("array body");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["username"], "erin");
        assert!(
            list[0].get("email").is_none(),
            "public user list must not leak email addresses"
        );
    }

    #[tokio::test]
    async fn refresh_rotates_the_token_and_rejects_reuse() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/register",
                &register_payload("frank"),
            ))
            .await
            .unwrap();
        let tokens = body_json(response).await;
        let refresh_token = tokens["refresh_token"]
            .as_str()
            .expect("refresh token present")
            .to_string();
        let refresh_payload = json!({ "refresh_token": refresh_token });

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/refresh",
                &refresh_payload,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let rotated = body_json(response).await;
        assert_ne!(rotated["refresh_token"].as_str().unwrap(), refresh_token);

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/refresh",
                &refresh_payload,
            ))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "a rotated-away token must not be reusable"
        );
    }

    #[tokio::test]
    async fn logout_revokes_the_refresh_token() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/register",
                &register_payload("grace"),
            ))
            .await
            .unwrap();
        let tokens = body_json(response).await;
        let access_token = tokens["access_token"]
            .as_str()
            .expect("access token present")
            .to_string();
        let refresh_token = tokens["refresh_token"]
            .as_str()
            .expect("refresh token present")
            .to_string();

        let mut logout_request = json_request(
            "POST",
            "/api/v1/auth/logout",
            &json!({ "refresh_token": refresh_token }),
        );
        logout_request.headers_mut().insert(
            "authorization",
            format!("Bearer {access_token}")
                .parse()
                .expect("valid header value"),
        );
        let response = app.clone().oneshot(logout_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let refresh_payload = json!({ "refresh_token": refresh_token });
        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/refresh",
                &refresh_payload,
            ))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "a revoked refresh token must not work"
        );
    }

    async fn register(app: Router, username: &str) -> Value {
        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/register",
                &register_payload(username),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        body_json(response).await
    }

    #[tokio::test]
    async fn first_registered_user_becomes_admin_and_second_becomes_member() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let first = register(app.clone(), "alice").await;
        assert_eq!(first["user"]["role"], "admin");

        let second = register(app, "bob").await;
        assert_eq!(second["user"]["role"], "member");
    }

    #[tokio::test]
    async fn admin_can_promote_a_member_but_not_themselves() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = register(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let admin_id = admin_tokens["user"]["id"].as_str().unwrap();

        let member_tokens = register(app.clone(), "bob").await;
        let member_id = member_tokens["user"]["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(auth_request(
                "PATCH",
                &format!("/api/v1/admin/users/{member_id}/role"),
                admin_token,
                Some(&json!({ "role": "admin" })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app
            .clone()
            .oneshot(auth_get("/api/v1/users", admin_token))
            .await
            .unwrap();
        let users = body_json(response).await;
        let promoted = users
            .as_array()
            .unwrap()
            .iter()
            .find(|user| user["id"] == member_id)
            .unwrap();
        assert_eq!(promoted["role"], "admin");

        let response = app
            .oneshot(auth_request(
                "PATCH",
                &format!("/api/v1/admin/users/{admin_id}/role"),
                admin_token,
                Some(&json!({ "role": "member" })),
            ))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::CONFLICT,
            "an admin must not be able to change their own role"
        );
    }

    #[tokio::test]
    async fn update_role_rejects_an_invalid_value() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = register(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let member_tokens = register(app.clone(), "bob").await;
        let member_id = member_tokens["user"]["id"].as_str().unwrap();

        let response = app
            .oneshot(auth_request(
                "PATCH",
                &format!("/api/v1/admin/users/{member_id}/role"),
                admin_token,
                Some(&json!({ "role": "superuser" })),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn non_admin_is_forbidden_from_admin_endpoints() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        register(app.clone(), "alice").await; // becomes admin, irrelevant to this test
        let member_tokens = register(app.clone(), "bob").await;
        let member_token = member_tokens["access_token"].as_str().unwrap();
        let member_id = member_tokens["user"]["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(auth_request(
                "PATCH",
                &format!("/api/v1/admin/users/{member_id}/role"),
                member_token,
                Some(&json!({ "role": "admin" })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = app
            .oneshot(auth_request(
                "DELETE",
                &format!("/api/v1/admin/users/{member_id}"),
                member_token,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn admin_can_remove_a_user_but_not_themselves() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = register(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let admin_id = admin_tokens["user"]["id"].as_str().unwrap();
        let member_tokens = register(app.clone(), "bob").await;
        let member_id = member_tokens["user"]["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(auth_request(
                "DELETE",
                &format!("/api/v1/admin/users/{admin_id}"),
                admin_token,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::CONFLICT,
            "an admin must not be able to remove their own account"
        );

        let response = app
            .clone()
            .oneshot(auth_request(
                "DELETE",
                &format!("/api/v1/admin/users/{member_id}"),
                admin_token,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app
            .oneshot(auth_get("/api/v1/users", admin_token))
            .await
            .unwrap();
        let users = body_json(response).await;
        assert_eq!(
            users.as_array().unwrap().len(),
            1,
            "the removed user must be gone"
        );
    }
}
