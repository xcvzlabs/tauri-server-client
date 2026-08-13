use std::net::SocketAddr;
use std::sync::Arc;

use server::config::Config;
use server::presence::PresenceRegistry;
use server::state::AppState;
use server::{db, routes};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env().unwrap_or_else(|err| {
        tracing::error!(error = %err, "failed to load configuration");
        std::process::exit(1);
    });

    let host = config.host;
    let port = config.port;

    let db = db::connect(&config.database_url)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "failed to connect to database");
            std::process::exit(1);
        });

    let state = AppState {
        db,
        config: Arc::new(config),
        presence: PresenceRegistry::new(),
    };
    let app = routes::build_router(state);

    let addr = SocketAddr::new(host, port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, %addr, "failed to bind listener");
            std::process::exit(1);
        });

    tracing::info!(%addr, "server listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap_or_else(|err| {
        tracing::error!(error = %err, "server error");
        std::process::exit(1);
    });
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => tracing::error!(error = %err, "failed to install SIGTERM handler"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received Ctrl+C, shutting down"),
        _ = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}
