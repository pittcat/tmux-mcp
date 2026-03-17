use std::net::SocketAddr;
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::config::Config;
use crate::state::command_registry::CommandRegistry;

mod config;
mod error;
mod mcp;
mod state;
mod tmux;
mod transport;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let config = Config::from_env()?;
    info!("Starting tmux-mcp-server on {}", config.bind_addr);

    let command_registry = Arc::new(CommandRegistry::new(
        config.max_commands,
        config.command_ttl_seconds,
    ));

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

    let app = create_router(command_registry);

    let addr: SocketAddr = config.bind_addr.parse()?;
    let listener = TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
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
