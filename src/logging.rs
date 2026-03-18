//! Logging subsystem with hourly rotation and 4-hour retention
//!
//! Provides file logging with:
//! - Hourly rotation via tracing-appender
//! - 4-hour retention cleanup of old log files
//! - No ANSI escape sequences in log files
//! - Automatic log directory creation

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::interval;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Log directory name within data directory
const LOG_DIR_NAME: &str = "tmux-mcp";
const LOG_FILES_DIR: &str = "logs";
/// Prefix for log files (tracing-appender adds timestamp suffix with hourly rotation)
const LOG_FILE_PREFIX: &str = "server.log";
/// Retention period for log files (4 hours)
const RETENTION_HOURS: u64 = 4;

/// Guard type for non-blocking file logging
pub type LoggingGuard = WorkerGuard;

/// State for log retention cleanup
#[derive(Debug)]
pub struct LogCleanupState {
    pub log_dir: PathBuf,
    pub file_prefix: String,
}

impl LogCleanupState {
    /// Creates a new cleanup state with the given log directory
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            log_dir,
            file_prefix: LOG_FILE_PREFIX.to_string(),
        }
    }
}

/// Initialize the logging subsystem with hourly rotation and 4-hour retention.
///
/// # Arguments
/// * `log_dir` - Directory where log files will be written
/// * `enable_console` - Whether to also log to stderr with ANSI colors
///
/// # Returns
/// * `LoggingGuard` - Guard that must be kept alive to flush logs
pub fn init_logging(log_dir: PathBuf, enable_console: bool) -> LoggingGuard {
    // Ensure log directory exists
    std::fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    // Create file appender with hourly rotation
    let file_appender = tracing_appender::rolling::hourly(&log_dir, LOG_FILE_PREFIX);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Build subscriber layers
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false) // Files must not have ANSI escape sequences
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    if enable_console {
        let console_layer = tracing_subscriber::fmt::layer()
            .with_ansi(true)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();
    }

    guard
}

/// Start the background task for log retention cleanup.
///
/// This task periodically scans the log directory and removes log files
/// older than the retention period (4 hours by default).
///
/// # Arguments
/// * `log_dir` - Directory to scan for old log files
pub fn start_log_cleanup_task(log_dir: PathBuf) {
    let state = Arc::new(Mutex::new(LogCleanupState::new(log_dir.clone())));

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(3600)); // Check every hour

        loop {
            interval.tick().await;
            if let Err(e) = cleanup_old_logs(&state).await {
                tracing::warn!("Log cleanup failed: {}", e);
            }
        }
    });
}

async fn cleanup_old_logs(state: &Arc<Mutex<LogCleanupState>>) -> std::io::Result<()> {
    let state = state.lock().await;
    let log_dir = &state.log_dir;
    let prefix = &state.file_prefix;

    let entries = std::fs::read_dir(log_dir)?;

    let now = std::time::SystemTime::now();
    let retention_duration = Duration::from_secs(RETENTION_HOURS * 3600);

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip non-files
        if !path.is_file() {
            continue;
        }

        // Get filename as string
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Only clean up files matching our prefix pattern
        // tracing-appender hourly rotation creates files like: server.log.2024-01-15_14-00-00
        if !filename.starts_with(prefix) {
            continue;
        }

        // Check modification time
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = now.duration_since(modified) {
                    if duration > retention_duration {
                        tracing::info!(
                            "Removing old log file: {} (age: {} hours)",
                            path.display(),
                            duration.as_secs() / 3600
                        );
                        if let Err(e) = std::fs::remove_file(&path) {
                            tracing::warn!("Failed to remove {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Get the default log directory path
pub fn default_log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(LOG_DIR_NAME)
        .join(LOG_FILES_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_log_cleanup_state_new() {
        let log_dir = PathBuf::from("/test/logs");
        let state = LogCleanupState::new(log_dir.clone());
        assert_eq!(state.log_dir, log_dir);
        assert_eq!(state.file_prefix, LOG_FILE_PREFIX);
    }

    #[tokio::test]
    async fn test_cleanup_old_logs_respects_prefix() {
        // Create temp directory
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        // Create a file that should NOT be cleaned (wrong prefix)
        let other_file = log_dir.join("other.log");
        fs::write(&other_file, "test").unwrap();

        // Create a file that SHOULD be cleaned (correct prefix but old)
        let old_log = log_dir.join(format!("{}.2020-01-01_00-00-00", LOG_FILE_PREFIX));
        fs::write(&old_log, "old log content").unwrap();

        // Set the modification time to 5 hours ago
        let five_hours_ago = std::time::SystemTime::now() - Duration::from_secs(5 * 3600);
        filetime::set_file_mtime(
            &old_log,
            filetime::FileTime::from_system_time(five_hours_ago),
        )
        .unwrap();

        let state = Arc::new(Mutex::new(LogCleanupState::new(log_dir.clone())));
        cleanup_old_logs(&state).await.unwrap();

        // The "other.log" file should still exist
        assert!(
            other_file.exists(),
            "Non-matching files should not be deleted"
        );
        // The old server.log file should be deleted
        assert!(!old_log.exists(), "Old log files should be deleted");
    }

    #[tokio::test]
    async fn test_cleanup_preserves_recent_logs() {
        // Create temp directory
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        // Create a recent log file (should NOT be cleaned)
        let recent_log = log_dir.join(format!("{}.2024-01-15_14-00-00", LOG_FILE_PREFIX));
        fs::write(&recent_log, "recent log content").unwrap();

        // File is already recent by default, no need to change mtime

        let state = Arc::new(Mutex::new(LogCleanupState::new(log_dir.clone())));
        cleanup_old_logs(&state).await.unwrap();

        // The recent log file should still exist
        assert!(
            recent_log.exists(),
            "Recent log files should not be deleted"
        );
    }
}
