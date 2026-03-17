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

/// 启动日志清理任务，删除超过 4 小时的日志文件
async fn start_log_cleanup_task(log_dir: PathBuf) {
    tokio::spawn(async move {
        let cleanup_interval = Duration::from_secs(300); // 每 5 分钟检查一次
        let retention_duration = Duration::from_secs(4 * 3600); // 4 小时

        loop {
            tokio::time::sleep(cleanup_interval).await;

            if let Ok(entries) = tokio::fs::read_dir(&log_dir).await {
                let mut entries = entries;
                let now = std::time::SystemTime::now();

                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("log") {
                        match entry.metadata().await {
                            Ok(metadata) => {
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(age) = now.duration_since(modified) {
                                        if age > retention_duration {
                                            if let Err(e) = tokio::fs::remove_file(&path).await {
                                                tracing::warn!(
                                                    "Failed to remove old log file {}: {}",
                                                    path.display(),
                                                    e
                                                );
                                            } else {
                                                tracing::info!(
                                                    "Removed old log file: {}",
                                                    path.display()
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to read metadata for {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 获取日志目录
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tmux-mcp")
        .join("logs");

    std::fs::create_dir_all(&log_dir)?;

    // 使用 tracing-appender 实现按小时轮转
    let file_appender = tracing_appender::rolling::hourly(&log_dir, "server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 同时输出到文件和控制台
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

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

    // 启动命令清理任务
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

    // 启动日志清理任务
    start_log_cleanup_task(log_dir).await;

    let app = create_router(command_registry);

    let addr: SocketAddr = config.bind_addr.parse()?;
    let listener = TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");

    // 确保日志 guard 存活到程序结束，保证所有日志都被写入
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
