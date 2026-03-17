use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::routing::{get, post};
use axum::Router;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::Config;
use crate::state::command_registry::CommandRegistry;

mod config;
mod error;
mod mcp;
mod state;
mod tmux;
mod transport;

/// Start log truncation task, clears log file content periodically
async fn start_log_cleanup_task(log_file: PathBuf) {
    tokio::spawn(async move {
        let cleanup_interval = Duration::from_secs(3600); // Check every hour
        let max_size = 10 * 1024 * 1024; // 10 MB size threshold

        loop {
            tokio::time::sleep(cleanup_interval).await;

            match tokio::fs::metadata(&log_file).await {
                Ok(metadata) => {
                    if metadata.len() > max_size {
                        if let Err(e) = tokio::fs::OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .open(&log_file)
                            .await
                        {
                            tracing::warn!(
                                "Failed to truncate log file {}: {}",
                                log_file.display(),
                                e
                            );
                        } else {
                            tracing::info!("Truncated log file: {}", log_file.display());
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Log file not found: {}: {}", log_file.display(), e);
                }
            }
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get log directory
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tmux-mcp")
        .join("logs");

    std::fs::create_dir_all(&log_dir)?;

    // Use fixed log file (no rotation)
    let log_file = log_dir.join("server.log");
    let file_appender = tracing_appender::rolling::never(&log_dir, "server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Output to both file and console
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(false),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(true))
        .init();

    let config = Config::from_env()?;
    info!("Starting tmux-mcp-server on {}", config.bind_addr);

    let command_registry = Arc::new(CommandRegistry::new(
        config.max_commands,
        config.command_ttl_seconds,
    ));

    // Start command cleanup task
    tokio::spawn({
        let registry = command_registry.clone();
        async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                registry.cleanup_expired();
            }
        }
    });

    // Start log truncation task
    start_log_cleanup_task(log_file).await;

    let app = create_router(command_registry);

    let addr: SocketAddr = config.bind_addr.parse()?;
    let listener = TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");

    // Ensure log guard lives until program end to flush all logs
    drop(_guard);

    Ok(())
}

fn create_router(command_registry: Arc<CommandRegistry>) -> Router {
    Router::new()
        // Standard MCP JSON-RPC 2.0 protocol endpoints
        .merge(mcp::protocol::create_protocol_router(
            command_registry.clone(),
        ))
        // Legacy REST API endpoints (for compatibility)
        .route("/mcp/tools", get(mcp::tools::list_tools))
        .route("/mcp/tools/:name", post(mcp::tools::call_tool))
        .route("/mcp/resources", get(mcp::resources::list_resources))
        .route("/mcp/resources/:uri", get(mcp::resources::read_resource))
        .layer(axum::extract::Extension(command_registry))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown");
}
