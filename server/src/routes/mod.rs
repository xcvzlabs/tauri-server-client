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
        .route("/signup", post(auth::sign_up))
        .route("/signin", post(auth::sign_in))
        .route("/refresh", post(auth::refresh))
        .route("/signout", post(auth::sign_out))
        .layer(GovernorLayer::new(Arc::new(auth_rate_limit)));

    let user_routes = Router::new()
        .route("/me", get(users::me))
        .route("/", get(users::list));

    let admin_routes = Router::new()
        .route("/users", get(admin::list_users).post(admin::add_user))
        .route("/users/{user_id}/role", patch(admin::update_role))
        .route("/users/{user_id}/disable", patch(admin::set_disabled))
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

    fn sign_up_payload(username: &str) -> Value {
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
    async fn sign_up_then_duplicate_username_is_conflict() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);
        let payload = sign_up_payload("alice");

        let response = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/auth/signup", &payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(json_request("POST", "/api/v1/auth/signup", &payload))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn sign_up_rejects_a_password_below_the_minimum_length() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let mut payload = sign_up_payload("bob");
        payload["password"] = json!("short");

        let response = app
            .oneshot(json_request("POST", "/api/v1/auth/signup", &payload))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn sign_in_succeeds_then_rejects_a_wrong_password() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        app.clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/signup",
                &sign_up_payload("carol"),
            ))
            .await
            .unwrap();

        let sign_in_payload =
            json!({ "username_or_email": "carol", "password": "correcthorsebattery" });
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/signin",
                &sign_in_payload,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let bad_sign_in = json!({ "username_or_email": "carol", "password": "wrong-password" });
        let response = app
            .oneshot(json_request("POST", "/api/v1/auth/signin", &bad_sign_in))
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
                "/api/v1/auth/signup",
                &sign_up_payload("dave"),
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
                "/api/v1/auth/signup",
                &sign_up_payload("erin"),
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
                "/api/v1/auth/signup",
                &sign_up_payload("frank"),
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
    async fn sign_out_revokes_the_refresh_token() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/signup",
                &sign_up_payload("grace"),
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

        let mut sign_out_request = json_request(
            "POST",
            "/api/v1/auth/signout",
            &json!({ "refresh_token": refresh_token }),
        );
        sign_out_request.headers_mut().insert(
            "authorization",
            format!("Bearer {access_token}")
                .parse()
                .expect("valid header value"),
        );
        let response = app.clone().oneshot(sign_out_request).await.unwrap();
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

    async fn sign_up(app: Router, username: &str) -> Value {
        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/signup",
                &sign_up_payload(username),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        body_json(response).await
    }

    #[tokio::test]
    async fn first_signed_up_user_becomes_admin_and_second_becomes_member() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let first = sign_up(app.clone(), "alice").await;
        assert_eq!(first["user"]["role"], "admin");

        let second = sign_up(app, "bob").await;
        assert_eq!(second["user"]["role"], "member");
    }

    #[tokio::test]
    async fn admin_can_promote_a_member_but_not_themselves() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let admin_id = admin_tokens["user"]["id"].as_str().unwrap();

        let member_tokens = sign_up(app.clone(), "bob").await;
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

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let member_tokens = sign_up(app.clone(), "bob").await;
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

        sign_up(app.clone(), "alice").await; // becomes admin, irrelevant to this test
        let member_tokens = sign_up(app.clone(), "bob").await;
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

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let admin_id = admin_tokens["user"]["id"].as_str().unwrap();
        let member_tokens = sign_up(app.clone(), "bob").await;
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

    #[tokio::test]
    async fn admin_list_exposes_email_and_ip_but_public_list_does_not() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(auth_get("/api/v1/admin/users", admin_token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let admin_view = body_json(response).await;
        let admin_view = admin_view.as_array().unwrap();
        assert_eq!(admin_view[0]["email"], "alice@example.com");
        assert!(admin_view[0].get("is_disabled").is_some());
        assert!(
            admin_view[0]["ip_address"].is_string(),
            "signing up should have recorded the caller's IP"
        );

        let response = app
            .oneshot(auth_get("/api/v1/users", admin_token))
            .await
            .unwrap();
        let public_view = body_json(response).await;
        let public_view = public_view.as_array().unwrap();
        assert!(
            public_view[0].get("email").is_none(),
            "public user list must not leak email addresses"
        );
        assert!(
            public_view[0].get("ip_address").is_none(),
            "public user list must not leak IP addresses"
        );
    }

    #[tokio::test]
    async fn non_admin_is_forbidden_from_the_admin_user_list() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        sign_up(app.clone(), "alice").await; // becomes admin, irrelevant to this test
        let member_tokens = sign_up(app.clone(), "bob").await;
        let member_token = member_tokens["access_token"].as_str().unwrap();

        let response = app
            .oneshot(auth_get("/api/v1/admin/users", member_token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn admin_can_add_a_user_with_a_chosen_role() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(auth_request(
                "POST",
                "/api/v1/admin/users",
                admin_token,
                Some(&json!({
                    "username": "carol",
                    "email": "carol@example.com",
                    "password": "correcthorsebattery",
                    "display_name": "Carol",
                    "role": "admin",
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let created = body_json(response).await;
        assert_eq!(created["role"], "admin");
        assert_eq!(created["username"], "carol");

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/signin",
                &json!({ "username_or_email": "carol", "password": "correcthorsebattery" }),
            ))
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "the admin-created account must be able to sign in with the given password"
        );
    }

    #[tokio::test]
    async fn add_user_rejects_an_invalid_role() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();

        let response = app
            .oneshot(auth_request(
                "POST",
                "/api/v1/admin/users",
                admin_token,
                Some(&json!({
                    "username": "carol",
                    "email": "carol@example.com",
                    "password": "correcthorsebattery",
                    "display_name": "Carol",
                    "role": "superuser",
                })),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn admin_cannot_disable_their_own_account() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let admin_id = admin_tokens["user"]["id"].as_str().unwrap();

        let response = app
            .oneshot(auth_request(
                "PATCH",
                &format!("/api/v1/admin/users/{admin_id}/disable"),
                admin_token,
                Some(&json!({ "is_disabled": true })),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn disabling_a_user_immediately_revokes_their_existing_access_token() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let member_tokens = sign_up(app.clone(), "bob").await;
        let member_token = member_tokens["access_token"].as_str().unwrap();
        let member_id = member_tokens["user"]["id"].as_str().unwrap();

        // The member's access token is still unexpired and was valid a
        // moment ago.
        let response = app
            .clone()
            .oneshot(auth_get("/api/v1/users/me", member_token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(auth_request(
                "PATCH",
                &format!("/api/v1/admin/users/{member_id}/disable"),
                admin_token,
                Some(&json!({ "is_disabled": true })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // The same still-unexpired access token must now be rejected,
        // without waiting for it to expire.
        let response = app
            .oneshot(auth_get("/api/v1/users/me", member_token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = body_json(response).await;
        assert_eq!(body["error"]["code"], "ACCOUNT_DISABLED");
    }

    #[tokio::test]
    async fn disabling_a_user_revokes_their_refresh_token_and_blocks_signin() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let member_tokens = sign_up(app.clone(), "bob").await;
        let member_id = member_tokens["user"]["id"].as_str().unwrap();
        let refresh_token = member_tokens["refresh_token"].as_str().unwrap().to_string();

        app.clone()
            .oneshot(auth_request(
                "PATCH",
                &format!("/api/v1/admin/users/{member_id}/disable"),
                admin_token,
                Some(&json!({ "is_disabled": true })),
            ))
            .await
            .unwrap();

        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/refresh",
                &json!({ "refresh_token": refresh_token }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/signin",
                &json!({ "username_or_email": "bob", "password": "correcthorsebattery" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = body_json(response).await;
        assert_eq!(body["error"]["code"], "ACCOUNT_DISABLED");
    }

    #[tokio::test]
    async fn re_enabling_a_user_restores_signin() {
        let (state, _guard) = test_state().await;
        let app = build_router(state);

        let admin_tokens = sign_up(app.clone(), "alice").await;
        let admin_token = admin_tokens["access_token"].as_str().unwrap();
        let member_tokens = sign_up(app.clone(), "bob").await;
        let member_id = member_tokens["user"]["id"].as_str().unwrap();

        for is_disabled in [true, false] {
            app.clone()
                .oneshot(auth_request(
                    "PATCH",
                    &format!("/api/v1/admin/users/{member_id}/disable"),
                    admin_token,
                    Some(&json!({ "is_disabled": is_disabled })),
                ))
                .await
                .unwrap();
        }

        let response = app
            .oneshot(json_request(
                "POST",
                "/api/v1/auth/signin",
                &json!({ "username_or_email": "bob", "password": "correcthorsebattery" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
