//! Tests for fixed-file log retention behavior.

use std::fs;

use chrono::{Duration as ChronoDuration, Utc};
use tempfile::TempDir;
use tmux_mcp_server::logging::prune_expired_logs;

const LOG_FILE_NAME: &str = "server.log";

#[test]
fn test_prune_expired_logs_removes_old_lines_from_server_log() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().to_path_buf();
    let log_file = log_dir.join(LOG_FILE_NAME);
    let old_line = format!(
        "{} INFO expired entry\n",
        (Utc::now() - ChronoDuration::hours(5)).to_rfc3339()
    );
    let recent_line = format!("{} INFO recent entry\n", Utc::now().to_rfc3339());

    fs::write(&log_file, format!("{old_line}{recent_line}")).unwrap();
    prune_expired_logs(log_dir).unwrap();

    let content = fs::read_to_string(&log_file).unwrap();
    assert!(!content.contains("expired entry"));
    assert!(content.contains("recent entry"));
}

#[test]
fn test_prune_expired_logs_preserves_non_log_files() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().to_path_buf();
    let log_file = log_dir.join(LOG_FILE_NAME);
    let other_file = log_dir.join("other.log");
    let old_line = format!(
        "{} INFO expired entry\n",
        (Utc::now() - ChronoDuration::hours(5)).to_rfc3339()
    );

    fs::write(&log_file, old_line).unwrap();
    fs::write(&other_file, "keep me").unwrap();
    prune_expired_logs(log_dir).unwrap();

    assert!(other_file.exists());
    assert_eq!(fs::read_to_string(other_file).unwrap(), "keep me");
    assert!(log_file.exists());
}

#[test]
fn test_prune_expired_logs_can_run_repeatedly() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().to_path_buf();
    let log_file = log_dir.join(LOG_FILE_NAME);
    let old_line = format!(
        "{} INFO expired entry\n",
        (Utc::now() - ChronoDuration::hours(5)).to_rfc3339()
    );

    fs::write(&log_file, old_line).unwrap();

    for _ in 0..3 {
        prune_expired_logs(log_dir.clone()).unwrap();
    }

    assert_eq!(fs::read_to_string(log_file).unwrap(), "");
}
