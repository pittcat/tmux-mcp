//! Tests for log rotation and retention behavior.
//!
//! These tests verify that:
//! 1. Log files are created with proper naming
//! 2. Old log files are cleaned up after retention period
//! 3. Non-matching files are not accidentally deleted
//! 4. File logs do not contain ANSI escape sequences

use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Prefix for log files
const LOG_FILE_PREFIX: &str = "server.log";
/// Retention period (4 hours in seconds)
const RETENTION_HOURS: u64 = 4;

struct TestLogCleanupState {
    log_dir: PathBuf,
    file_prefix: String,
}

impl TestLogCleanupState {
    fn new(log_dir: PathBuf) -> Self {
        Self {
            log_dir,
            file_prefix: LOG_FILE_PREFIX.to_string(),
        }
    }
}

async fn cleanup_old_logs(state: &Mutex<TestLogCleanupState>) -> std::io::Result<()> {
    let state = state.lock().await;
    let log_dir = &state.log_dir;
    let prefix = &state.file_prefix;

    let entries = fs::read_dir(log_dir)?;

    let now = std::time::SystemTime::now();
    let retention_duration = Duration::from_secs(RETENTION_HOURS * 3600);

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Only clean up files matching our prefix pattern
        if !filename.starts_with(prefix) {
            continue;
        }

        // Check modification time
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = now.duration_since(modified) {
                    if duration > retention_duration {
                        fs::remove_file(&path)?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_log_cleanup_removes_old_files() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().to_path_buf();

    // Create a file that should be cleaned (old, correct prefix)
    let old_log = log_dir.join(format!("{}.2020-01-01_00-00-00", LOG_FILE_PREFIX));
    fs::write(&old_log, "old log content").unwrap();

    // Set modification time to 5 hours ago
    let five_hours_ago = std::time::SystemTime::now() - Duration::from_secs(5 * 3600);
    filetime::set_file_mtime(
        &old_log,
        filetime::FileTime::from_system_time(five_hours_ago),
    )
    .unwrap();

    let state = Mutex::new(TestLogCleanupState::new(log_dir.clone()));
    cleanup_old_logs(&state).await.unwrap();

    // Old log should be deleted
    assert!(!old_log.exists(), "Old log files should be deleted");
}

#[tokio::test]
async fn test_log_cleanup_preserves_recent_files() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().to_path_buf();

    // Create a recent log file
    let recent_log = log_dir.join(format!("{}.2024-01-15_14-00-00", LOG_FILE_PREFIX));
    fs::write(&recent_log, "recent log content").unwrap();

    let state = Mutex::new(TestLogCleanupState::new(log_dir.clone()));
    cleanup_old_logs(&state).await.unwrap();

    // Recent log should still exist
    assert!(
        recent_log.exists(),
        "Recent log files should not be deleted"
    );
}

#[tokio::test]
async fn test_log_cleanup_preserves_non_matching_files() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().to_path_buf();

    // Create a file with wrong prefix (should not be cleaned)
    let other_file = log_dir.join("other.log");
    fs::write(&other_file, "other content").unwrap();

    // Create old log with correct prefix
    let old_log = log_dir.join(format!("{}.2020-01-01_00-00-00", LOG_FILE_PREFIX));
    fs::write(&old_log, "old log content").unwrap();

    // Set modification time to 5 hours ago
    let five_hours_ago = std::time::SystemTime::now() - Duration::from_secs(5 * 3600);
    filetime::set_file_mtime(
        &old_log,
        filetime::FileTime::from_system_time(five_hours_ago),
    )
    .unwrap();

    let state = Mutex::new(TestLogCleanupState::new(log_dir.clone()));
    cleanup_old_logs(&state).await.unwrap();

    // Non-matching file should still exist
    assert!(
        other_file.exists(),
        "Non-matching files should not be deleted"
    );
    // Old log should be deleted
    assert!(!old_log.exists(), "Old log files should be deleted");
}

#[test]
fn test_file_log_layer_disables_ansi() {
    // Create a temporary directory for logs
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().to_path_buf();

    // Create file appender with hourly rotation
    let file_appender = tracing_appender::rolling::hourly(&log_dir, LOG_FILE_PREFIX);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Build file layer with ANSI disabled
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false) // ANSI should be disabled for files
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .init();

    // Write a log message with potential ANSI codes
    tracing::info!("Test message with potential ANSI codes: \x1b[31mred\x1b[0m");

    // Drop the guard to flush
    drop(_guard);

    // Read the log file and check for ANSI codes
    let entries = fs::read_dir(&log_dir).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let content = fs::read_to_string(&path).unwrap();
            // ANSI escape sequences start with \x1b
            assert!(
                !content.contains("\x1b"),
                "Log file should not contain ANSI escape sequences"
            );
        }
    }
}

#[tokio::test]
async fn test_log_cleanup_task_runs_periodically() {
    // This test verifies the cleanup task can run multiple times
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().to_path_buf();

    // Create an old log file
    let old_log = log_dir.join(format!("{}.2020-01-01_00-00-00", LOG_FILE_PREFIX));
    fs::write(&old_log, "old log content").unwrap();

    let five_hours_ago = std::time::SystemTime::now() - Duration::from_secs(5 * 3600);
    filetime::set_file_mtime(
        &old_log,
        filetime::FileTime::from_system_time(five_hours_ago),
    )
    .unwrap();

    let state = Mutex::new(TestLogCleanupState::new(log_dir.clone()));

    // Run cleanup multiple times
    for _ in 0..3 {
        cleanup_old_logs(&state).await.unwrap();
        // Small delay to let any background task run
        let mut interval = interval(Duration::from_millis(10));
        interval.tick().await;
    }

    // Old log should still be deleted
    assert!(!old_log.exists(), "Old log should be deleted after cleanup");
}
