# server

Standalone REST + WebSocket API for the LAN server. Built with [Axum](https://docs.rs/axum), SQLite (via [sqlx](https://docs.rs/sqlx)), JWT authentication, and real-time user presence tracking. Runs as its own process; the Tauri app and Nuxt frontend talk to it over HTTP/WebSocket on the LAN.

## Setup

```bash
cp .env.example .env
# edit .env, at minimum set a real JWT_SECRET (openssl rand -base64 48)
cargo run
```

The SQLite database file and its parent directory are created automatically, and schema migrations run on startup.

## Configuration

All configuration is via environment variables (see `.env.example`):

| Variable                   | Description                                              | Default      |
| -------------------------- | -------------------------------------------------------- | ------------ |
| `SERVER_HOST`              | Bind address                                             | `0.0.0.0`    |
| `SERVER_PORT`              | Bind port                                                | `8080`       |
| `DATABASE_URL`             | SQLite connection string                                 | _(required)_ |
| `JWT_SECRET`               | HS256 signing secret, min. 32 characters                 | _(required)_ |
| `ACCESS_TOKEN_TTL_MINUTES` | Access token lifetime                                    | `15`         |
| `REFRESH_TOKEN_TTL_DAYS`   | Refresh token lifetime                                   | `7`          |
| `ALLOWED_ORIGINS`          | Comma-separated CORS allow-list; empty allows any origin | _(empty)_    |

## API

All request/response bodies are JSON. Errors are shaped as `{ "error": { "code": "STABLE_CODE", "message": "..." } }`.

- `GET /health` — liveness + database connectivity probe.
- `POST /api/v1/auth/register` — `{ username, email, password, display_name }` → token pair.
- `POST /api/v1/auth/login` — `{ username_or_email, password }` → token pair.
- `POST /api/v1/auth/refresh` — `{ refresh_token }` → rotated token pair (single-use; reuse is rejected).
- `POST /api/v1/auth/logout` — `Authorization: Bearer <access>` + `{ refresh_token }` → revokes that session.
- `GET /api/v1/users/me` — `Authorization: Bearer <access>` → your own profile (includes email).
- `GET /api/v1/users` — `Authorization: Bearer <access>` → all users with their current online state.
- `GET /ws/presence?token=<access_token>` — WebSocket. On connect/disconnect the server broadcasts `{"type":"user_online"|"user_offline", user_id, username}` to every connected client and persists `is_online`/`last_seen_at` on the user record. On connect you also receive one `{"type":"snapshot", "online_user_ids":[...]}` message. The token travels as a query parameter because browser/webview `WebSocket` cannot set an `Authorization` header.

Auth endpoints (`/api/v1/auth/*`) are rate-limited per client IP to blunt credential-stuffing/brute-force attempts.

## Security notes

- Passwords are hashed with Argon2id.
- Refresh tokens are random 256-bit values; only their SHA-256 hash is stored, and each refresh rotates (revokes-then-reissues) the token so a stolen, already-used token can't be replayed.
- Access tokens are short-lived, stateless JWTs (HS256); nothing sensitive is embedded in the payload beyond user id/username.

## Development

```bash
cargo build
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
```
