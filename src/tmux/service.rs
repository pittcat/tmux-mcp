use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use crate::error::{Result, TmuxMcpError};
use crate::tmux::models::{ShellType, TmuxPane, TmuxSession, TmuxWindow};
use crate::tmux::parser;

/// Default timeout for tmux commands (10 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// Execute a tmux command asynchronously with timeout.
/// Uses bash to avoid zsh plugin issues.
/// Does NOT use tmux -C (control mode) which can hang.
pub async fn execute_tmux(tmux_command: &str) -> Result<String> {
    execute_tmux_with_timeout(tmux_command, DEFAULT_TIMEOUT_SECS).await
}

/// Execute a tmux command with a custom timeout
pub async fn execute_tmux_with_timeout(tmux_command: &str, timeout_secs: u64) -> Result<String> {
    let mut cmd = Command::new("bash");
    cmd.args(["-c", &format!("tmux {}", tmux_command)]);

    let result = timeout(Duration::from_secs(timeout_secs), cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if !output.status.success() {
                let err_msg = if stderr.is_empty() {
                    stdout.clone()
                } else {
                    stderr
                };

                if err_msg.contains("no server running") || err_msg.contains("no such server") {
                    return Err(TmuxMcpError::TmuxNotAvailable);
                } else if err_msg.contains("can't find session")
                    || err_msg.contains("no such session")
                {
                    return Err(TmuxMcpError::SessionNotFound(err_msg));
                } else if err_msg.contains("can't find window")
                    || err_msg.contains("no such window")
                {
                    return Err(TmuxMcpError::WindowNotFound(err_msg));
                } else if err_msg.contains("can't find pane") || err_msg.contains("no such pane") {
                    return Err(TmuxMcpError::PaneNotFound(err_msg));
                }

                return Err(TmuxMcpError::TmuxError(err_msg));
            }

            Ok(stdout.trim().to_string())
        }
        Ok(Err(e)) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(TmuxMcpError::TmuxNotAvailable)
            } else {
                Err(TmuxMcpError::InternalError(e.to_string()))
            }
        }
        Err(_) => {
            // Timeout elapsed
            Err(TmuxMcpError::TmuxTimeout(timeout_secs))
        }
    }
}

pub async fn list_sessions() -> Result<Vec<TmuxSession>> {
    let format = "#{session_id}:#{session_name}:#{?session_attached,1,0}:#{session_windows}";
    let output = execute_tmux(&format!("list-sessions -F '{}'", format)).await?;
    Ok(parser::parse_sessions(&output))
}

pub async fn find_session_by_name(name: &str) -> Result<Option<TmuxSession>> {
    let sessions = list_sessions().await?;
    Ok(sessions.into_iter().find(|s| s.name == name))
}

pub async fn list_windows(session_id: &str) -> Result<Vec<TmuxWindow>> {
    let format = "#{window_id}:#{window_name}:#{?window_active,1,0}";
    let output = execute_tmux(&format!(
        "list-windows -t '{}' -F '{}'",
        escape_target(session_id),
        format
    ))
    .await?;
    Ok(parser::parse_windows(&output, session_id))
}

pub async fn list_panes(window_id: &str) -> Result<Vec<TmuxPane>> {
    let format = "#{pane_id}:#{pane_title}:#{?pane_active,1,0}";
    let output = execute_tmux(&format!(
        "list-panes -t '{}' -F '{}'",
        escape_target(window_id),
        format
    ))
    .await?;
    Ok(parser::parse_panes(&output, window_id))
}

pub async fn capture_pane_content(
    pane_id: &str,
    lines: Option<usize>,
    include_colors: bool,
) -> Result<String> {
    let lines = lines.unwrap_or(200);
    let color_flag = if include_colors { "-e" } else { "" };
    let cmd = format!(
        "capture-pane -p {} -t '{}' -S -{} -E -",
        color_flag,
        escape_target(pane_id),
        lines
    );
    execute_tmux(cmd.trim()).await
}

pub async fn create_session(name: &str) -> Result<TmuxSession> {
    execute_tmux(&format!("new-session -d -s '{}'", name)).await?;
    find_session_by_name(name)
        .await?
        .ok_or_else(|| TmuxMcpError::TmuxError("Failed to create session".to_string()))
}

pub async fn create_window(session_id: &str, name: &str) -> Result<TmuxWindow> {
    execute_tmux(&format!(
        "new-window -t '{}' -n '{}'",
        escape_target(session_id),
        name
    ))
    .await?;
    let windows = list_windows(session_id).await?;
    windows
        .into_iter()
        .find(|w| w.name == name)
        .ok_or_else(|| TmuxMcpError::TmuxError("Failed to create window".to_string()))
}

pub async fn kill_session(session_id: &str) -> Result<()> {
    execute_tmux(&format!("kill-session -t '{}'", escape_target(session_id)))
        .await
        .map(|_| ())
}

pub async fn kill_window(window_id: &str) -> Result<()> {
    execute_tmux(&format!("kill-window -t '{}'", escape_target(window_id)))
        .await
        .map(|_| ())
}

pub async fn kill_pane(pane_id: &str) -> Result<()> {
    execute_tmux(&format!("kill-pane -t '{}'", escape_target(pane_id)))
        .await
        .map(|_| ())
}

pub async fn split_pane(
    target_pane_id: &str,
    direction: &str,
    size: Option<u8>,
) -> Result<TmuxPane> {
    let dir_flag = if direction == "horizontal" {
        "-h"
    } else {
        "-v"
    };
    let size_flag = size
        .filter(|s| *s > 0 && *s < 100)
        .map(|s| format!(" -p {}", s))
        .unwrap_or_default();

    let output = execute_tmux(&format!(
        "split-window {} -P -F '#{{pane_id}}:#{{pane_title}}:#{{?pane_active,1,0}}:#{{window_id}}' -t '{}'{}",
        dir_flag,
        escape_target(target_pane_id),
        size_flag
    ))
    .await?;
    parser::parse_pane(&output)
}

pub async fn send_keys(pane_id: &str, keys: &str, is_special: bool) -> Result<()> {
    if is_special {
        execute_tmux(&format!(
            "send-keys -t '{}' {}",
            escape_target(pane_id),
            keys
        ))
        .await
        .map(|_| ())
    } else {
        for ch in keys.chars() {
            let escaped = if ch == '\'' {
                "'\\''".to_string()
            } else {
                ch.to_string()
            };
            execute_tmux(&format!(
                "send-keys -t '{}' '{}'",
                escape_target(pane_id),
                escaped
            ))
            .await?;
        }
        Ok(())
    }
}

pub async fn send_keys_enter(pane_id: &str, command: &str) -> Result<()> {
    let escaped = command.replace('\'', "'\\''");
    execute_tmux(&format!(
        "send-keys -t '{}' '{}' Enter",
        escape_target(pane_id),
        escaped
    ))
    .await
    .map(|_| ())
}

fn escape_target(target: &str) -> String {
    target.replace('\'', "'\\''")
}

pub fn get_end_marker_text(shell_type: ShellType) -> String {
    format!("TMUX_MCP_DONE_{}", shell_type.exit_code_var())
}
