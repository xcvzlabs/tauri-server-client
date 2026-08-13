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

## Seeding

```bash
cargo run --bin seed
```

Creates two accounts if they don't already exist — safe to re-run (existing usernames are skipped, not overwritten):

| Role     | Username | Password (default) | Override env var       |
| -------- | -------- | ------------------ | ---------------------- |
| `admin`  | `admin`  | `adminpass123`     | `SEED_ADMIN_PASSWORD`  |
| `member` | `client` | `clientpass123`    | `SEED_CLIENT_PASSWORD` |

## API

All request/response bodies are JSON. Errors are shaped as `{ "error": { "code": "STABLE_CODE", "message": "..." } }`.

- `GET /health` — liveness + database connectivity probe.
- `POST /api/v1/auth/signup` — `{ username, email, password, display_name }` → token pair.
- `POST /api/v1/auth/signin` — `{ username_or_email, password }` → token pair.
- `POST /api/v1/auth/refresh` — `{ refresh_token }` → rotated token pair (single-use; reuse is rejected).
- `POST /api/v1/auth/signout` — `Authorization: Bearer <access>` + `{ refresh_token }` → revokes that session.
- `GET /api/v1/users/me` — `Authorization: Bearer <access>` → your own profile (includes email).
- `GET /api/v1/users` — `Authorization: Bearer <access>` → all users with their current online state. Does not include email or IP address.
- `GET /api/v1/admin/users` — `Authorization: Bearer <access>`, admin only → all users, additionally including `email`, `ip_address` (last known, LAN address), and `is_disabled`.
- `POST /api/v1/admin/users` — admin only → `{ username, email, password, display_name, role }` → creates a user directly with the given role (`"admin"` or `"member"`), no signup flow needed.
- `PATCH /api/v1/admin/users/{id}/role` — admin only → `{ role }` → promotes/demotes a user. An admin cannot change their own role.
- `PATCH /api/v1/admin/users/{id}/disable` — admin only → `{ is_disabled }` → disables or re-enables an account. Disabling immediately revokes all of that user's refresh tokens and force-closes any open WebSocket connection (see Security notes). An admin cannot disable their own account.
- `DELETE /api/v1/admin/users/{id}` — admin only → deletes a user. An admin cannot remove their own account.
- `GET /ws/presence?token=<access_token>` — WebSocket. On connect/disconnect the server broadcasts `{"type":"user_online"|"user_offline", user_id, username}` to every connected client and persists `is_online`/`last_seen_at`/`ip_address` on the user record. On connect you also receive one `{"type":"snapshot", "online_user_ids":[...]}` message. The token travels as a query parameter because browser/webview `WebSocket` cannot set an `Authorization` header.

Auth endpoints (`/api/v1/auth/*`) are rate-limited per client IP to blunt credential-stuffing/brute-force attempts.

## Security notes

- Passwords are hashed with Argon2id.
- Refresh tokens are random 256-bit values; only their SHA-256 hash is stored, and each refresh rotates (revokes-then-reissues) the token so a stolen, already-used token can't be replayed.
- Access tokens are short-lived JWTs (HS256), but they are **not** purely stateless: every authenticated request re-checks the account's `is_disabled` flag in the database. This means disabling an account takes effect on that user's very next request rather than only once their token expires, at the cost of one indexed lookup per request.
- Disabling an account also revokes its refresh tokens and broadcasts a force-disconnect over the presence WebSocket, closing any connection that account currently has open.

## Development

```bash
cargo build
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
```
