use serde::Serialize;
use std::env;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

const SERVER_PORT: u16 = 8080;
const HOSTING_STORE_FILE: &str = "hosting.json";
const JWT_SECRET_KEY: &str = "jwtSecret";
const READY_TIMEOUT: Duration = Duration::from_secs(10);
const READY_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Owns the child process for a server this app instance is hosting, if any.
pub struct HostedServer(pub Mutex<Option<Child>>);

impl HostedServer {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }
}

/// Kills any still-running hosted server. Called on window close so the app never leaves
/// an orphaned server process behind after the admin quits.
pub fn kill_hosted_server(hosted: &HostedServer) {
    if let Ok(mut guard) = hosted.0.lock()
        && let Some(mut child) = guard.take()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HostInfo {
    pub host: String,
    pub port: u16,
}

#[tauri::command]
pub fn start_server(app: AppHandle, hosted: State<HostedServer>) -> Result<HostInfo, String> {
    let mut guard = hosted
        .0
        .lock()
        .map_err(|_| "hosting state is unavailable".to_string())?;

    if let Some(child) = guard.as_mut() {
        // `Ok(None)` means the process is still alive; anything else (exited or an error
        // checking) means it's not usable anymore, so fall through and start a fresh one.
        if matches!(child.try_wait(), Ok(None)) {
            return Ok(HostInfo {
                host: "127.0.0.1".to_string(),
                port: SERVER_PORT,
            });
        }
        *guard = None;
    }

    let binary_path = server_binary_path()
        .ok_or_else(|| "server binary not found — run `bun run server:build` first".to_string())?;

    let app_data_dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
    std::fs::create_dir_all(&app_data_dir).map_err(|err| err.to_string())?;
    let database_path = app_data_dir.join("server.sqlite");

    let jwt_secret = load_or_create_jwt_secret(&app)?;

    let child = Command::new(binary_path)
        .env("SERVER_HOST", "0.0.0.0")
        .env("SERVER_PORT", SERVER_PORT.to_string())
        .env(
            "DATABASE_URL",
            format!("sqlite://{}", database_path.display()),
        )
        .env("JWT_SECRET", jwt_secret)
        .env("ALLOWED_ORIGINS", "")
        .spawn()
        .map_err(|err| format!("failed to start server: {err}"))?;

    *guard = Some(child);
    drop(guard);

    if let Err(err) = wait_until_ready(SERVER_PORT) {
        // Readiness failed: don't leave a half-started process behind.
        if let Ok(mut guard) = hosted.0.lock()
            && let Some(mut child) = guard.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
        return Err(err);
    }

    Ok(HostInfo {
        host: "127.0.0.1".to_string(),
        port: SERVER_PORT,
    })
}

#[tauri::command]
pub fn stop_server(hosted: State<HostedServer>) -> Result<(), String> {
    let mut guard = hosted
        .0
        .lock()
        .map_err(|_| "hosting state is unavailable".to_string())?;
    if let Some(mut child) = guard.take() {
        child.kill().map_err(|err| err.to_string())?;
        let _ = child.wait();
    }
    Ok(())
}

fn server_binary_path() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let binary_name = format!("server{}", env::consts::EXE_SUFFIX);
    let path = manifest_dir
        .join("..")
        .join("server")
        .join("target")
        .join(profile)
        .join(binary_name);
    path.exists().then_some(path)
}

/// The server signs JWTs with this secret; reusing it across hosting sessions means
/// restarting hosting doesn't force every connected member to log in again.
fn load_or_create_jwt_secret(app: &AppHandle) -> Result<String, String> {
    let store = app
        .store(HOSTING_STORE_FILE)
        .map_err(|err| err.to_string())?;

    if let Some(existing) = store
        .get(JWT_SECRET_KEY)
        .and_then(|value| value.as_str().map(str::to_string))
    {
        return Ok(existing);
    }

    let secret = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    store.set(JWT_SECRET_KEY, secret.clone());
    store.save().map_err(|err| err.to_string())?;
    Ok(secret)
}

fn wait_until_ready(port: u16) -> Result<(), String> {
    let deadline = Instant::now() + READY_TIMEOUT;
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("server did not become ready in time".to_string());
        }
        std::thread::sleep(READY_POLL_INTERVAL);
    }
}
