//! Logging subsystem with a fixed log file and 4-hour content retention.
//!
//! Provides file logging with:
//! - A fixed `server.log` file path
//! - Hourly cleanup that prunes log entries older than 4 hours
//! - No ANSI escape sequences in log files
//! - Automatic log directory creation

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use tokio::time::{interval_at, Instant};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Log directory name within data directory
const LOG_DIR_NAME: &str = "tmux-mcp";
const LOG_FILES_DIR: &str = "logs";
/// Fixed filename for file logs
const LOG_FILE_NAME: &str = "server.log";
/// Retention period for log files (4 hours)
const RETENTION_HOURS: i64 = 4;
const CLEANUP_INTERVAL_SECONDS: u64 = 3600;

/// Guard type for logging lifecycle state.
#[derive(Clone, Debug)]
pub struct LoggingGuard {
    cleanup_state: Arc<LogCleanupState>,
}

impl LoggingGuard {
    pub fn cleanup_state(&self) -> Arc<LogCleanupState> {
        Arc::clone(&self.cleanup_state)
    }
}

/// State for log retention cleanup.
#[derive(Debug)]
pub struct LogCleanupState {
    log_file: SynchronizedLogFile,
}

impl LogCleanupState {
    /// Creates a new cleanup state with the given log directory.
    pub fn new(log_dir: PathBuf) -> Self {
        Self::from_log_file(SynchronizedLogFile::new(log_file_path(&log_dir)))
    }

    fn from_log_file(log_file: SynchronizedLogFile) -> Self {
        Self { log_file }
    }
}

#[derive(Clone, Debug)]
struct SynchronizedLogFile {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl SynchronizedLogFile {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Arc::new(Mutex::new(())),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn append_bytes(&self, bytes: &[u8]) -> io::Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }

        let _guard = self.lock()?;
        ensure_parent_dir(&self.path)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(bytes)?;
        file.flush()?;
        Ok(())
    }

    fn prune_expired_records(&self, cutoff: DateTime<Utc>) -> io::Result<usize> {
        let _guard = self.lock()?;
        ensure_parent_dir(&self.path)?;

        let original = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
            Err(err) => return Err(err),
        };

        let retained = retain_recent_log_records(&original, cutoff);
        if retained == original {
            return Ok(0);
        }

        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, retained.as_bytes())?;
        fs::rename(&temp_path, &self.path)?;

        Ok(original.len().saturating_sub(retained.len()))
    }

    fn lock(&self) -> io::Result<MutexGuard<'_, ()>> {
        self.lock
            .lock()
            .map_err(|_| io::Error::other("log file lock poisoned"))
    }
}

#[derive(Clone, Debug)]
struct FixedFileMakeWriter {
    log_file: SynchronizedLogFile,
}

impl FixedFileMakeWriter {
    fn new(log_file: SynchronizedLogFile) -> Self {
        Self { log_file }
    }
}

impl<'a> MakeWriter<'a> for FixedFileMakeWriter {
    type Writer = FixedFileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        FixedFileWriter::new(self.log_file.clone())
    }
}

#[derive(Debug)]
struct FixedFileWriter {
    log_file: SynchronizedLogFile,
    buffer: Vec<u8>,
}

impl FixedFileWriter {
    fn new(log_file: SynchronizedLogFile) -> Self {
        Self {
            log_file,
            buffer: Vec::with_capacity(512),
        }
    }
}

impl Write for FixedFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.log_file.append_bytes(&self.buffer)?;
        self.buffer.clear();
        Ok(())
    }
}

impl Drop for FixedFileWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Rfc3339UtcTimer;

impl FormatTime for Rfc3339UtcTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
        write!(
            w,
            "{} ",
            Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
        )
    }
}

/// Initialize the logging subsystem with a fixed log file and 4-hour retention.
///
/// # Arguments
/// * `log_dir` - Directory where the fixed log file will be written
/// * `enable_console` - Whether to also log to stderr with ANSI colors
///
/// # Returns
/// * `LoggingGuard` - Guard that keeps shared logging state alive
pub fn init_logging(log_dir: PathBuf, enable_console: bool) -> LoggingGuard {
    fs::create_dir_all(&log_dir).expect("Failed to create log directory");

    if let Err(err) = prune_expired_logs(log_dir.clone()) {
        tracing::warn!("startup log prune failed: {err}");
    }

    let log_file = SynchronizedLogFile::new(log_file_path(&log_dir));
    let cleanup_state = Arc::new(LogCleanupState::from_log_file(log_file.clone()));
    let file_writer = FixedFileMakeWriter::new(log_file);
    let timer = Rfc3339UtcTimer;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_timer(timer)
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    if enable_console {
        let console_layer = tracing_subscriber::fmt::layer()
            .with_timer(timer)
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

    LoggingGuard { cleanup_state }
}

/// Start the background task for log retention cleanup.
///
/// This task periodically scans the fixed log file and removes log records
/// older than the retention period (4 hours by default).
pub fn start_log_cleanup_task(state: Arc<LogCleanupState>) {
    tokio::spawn(async move {
        let mut ticker = interval_at(
            Instant::now() + Duration::from_secs(CLEANUP_INTERVAL_SECONDS),
            Duration::from_secs(CLEANUP_INTERVAL_SECONDS),
        );

        loop {
            ticker.tick().await;
            if let Err(err) = cleanup_old_logs(state.as_ref()) {
                tracing::warn!("Log cleanup failed: {}", err);
            }
        }
    });
}

/// Prune expired log lines from the fixed log file in `log_dir`.
pub fn prune_expired_logs(log_dir: PathBuf) -> io::Result<()> {
    let state = LogCleanupState::new(log_dir);
    cleanup_old_logs(&state)
}

fn cleanup_old_logs(state: &LogCleanupState) -> io::Result<()> {
    let cutoff = Utc::now() - chrono::Duration::hours(RETENTION_HOURS);
    let removed_bytes = state.log_file.prune_expired_records(cutoff)?;

    if removed_bytes > 0 {
        tracing::info!(
            "Pruned {} bytes of expired logs from {}",
            removed_bytes,
            state.log_file.path().display()
        );
    }

    Ok(())
}

fn log_file_path(log_dir: &Path) -> PathBuf {
    log_dir.join(LOG_FILE_NAME)
}

fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn retain_recent_log_records(contents: &str, cutoff: DateTime<Utc>) -> String {
    let mut retained = String::new();
    let mut current_record = String::new();
    let mut current_record_should_keep = true;
    let mut seen_timestamped_record = false;

    for line in contents.split_inclusive('\n') {
        if let Some(timestamp) = parse_log_line_timestamp(line) {
            if !current_record.is_empty() && current_record_should_keep {
                retained.push_str(&current_record);
            }

            current_record.clear();
            current_record_should_keep = timestamp >= cutoff;
            seen_timestamped_record = true;
        } else if !seen_timestamped_record && current_record.is_empty() {
            current_record_should_keep = true;
        }

        current_record.push_str(line);
    }

    if !current_record.is_empty() && current_record_should_keep {
        retained.push_str(&current_record);
    }

    retained
}

fn parse_log_line_timestamp(line: &str) -> Option<DateTime<Utc>> {
    let token = line.split_whitespace().next()?;
    DateTime::parse_from_rfc3339(token)
        .ok()
        .map(|parsed| parsed.with_timezone(&Utc))
}

/// Get the default log directory path.
pub fn default_log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(LOG_DIR_NAME)
        .join(LOG_FILES_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use tempfile::TempDir;

    #[test]
    fn test_log_cleanup_state_new() {
        let log_dir = PathBuf::from("/test/logs");
        let state = LogCleanupState::new(log_dir.clone());
        assert_eq!(state.log_file.path(), log_dir.join(LOG_FILE_NAME));
    }

    #[test]
    fn test_log_file_path_uses_fixed_server_log_name() {
        let log_dir = PathBuf::from("/test/logs");
        assert_eq!(log_file_path(&log_dir), log_dir.join(LOG_FILE_NAME));
    }

    #[test]
    fn test_fixed_file_writer_appends_to_server_log() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = SynchronizedLogFile::new(log_file_path(temp_dir.path()));
        let mut writer = FixedFileWriter::new(log_file.clone());

        writer.write_all(b"first line\n").unwrap();
        writer.flush().unwrap();
        writer.write_all(b"second line\n").unwrap();
        writer.flush().unwrap();

        let content = fs::read_to_string(log_file.path()).unwrap();
        assert_eq!(content, "first line\nsecond line\n");
    }

    #[test]
    fn test_cleanup_old_logs_prunes_expired_lines_from_fixed_log_file() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();
        let log_file = log_file_path(&log_dir);
        let old_line = format!(
            "{} INFO expired entry\n",
            (Utc::now() - ChronoDuration::hours(5)).to_rfc3339()
        );
        let recent_line = format!("{} INFO recent entry\n", Utc::now().to_rfc3339());
        fs::write(&log_file, format!("{old_line}{recent_line}")).unwrap();

        let state = LogCleanupState::new(log_dir);
        cleanup_old_logs(&state).unwrap();

        let content = fs::read_to_string(&log_file).unwrap();
        assert!(
            !content.contains("expired entry"),
            "Expired log lines should be pruned from server.log"
        );
        assert!(
            content.contains("recent entry"),
            "Recent log lines should be retained in server.log"
        );
        assert!(
            log_file.exists(),
            "Cleanup should preserve the fixed log file"
        );
    }

    #[test]
    fn test_retain_recent_log_records_keeps_multiline_recent_record() {
        let cutoff = Utc::now() - ChronoDuration::hours(4);
        let old_record = format!(
            "{} INFO expired entry\n  expired detail\n",
            (Utc::now() - ChronoDuration::hours(5)).to_rfc3339()
        );
        let recent_record = format!(
            "{} INFO recent entry\n  recent detail\n",
            Utc::now().to_rfc3339()
        );

        let retained = retain_recent_log_records(&format!("{old_record}{recent_record}"), cutoff);

        assert!(!retained.contains("expired entry"));
        assert!(!retained.contains("expired detail"));
        assert!(retained.contains("recent entry"));
        assert!(retained.contains("recent detail"));
    }
}
