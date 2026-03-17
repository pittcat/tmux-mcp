use std::process::Command;

use crate::error::{Result, TmuxMcpError};
use crate::tmux::models::{ShellType, TmuxPane, TmuxSession, TmuxWindow};
use crate::tmux::parser;

/// Execute a tmux command using bash to avoid zsh plugin issues
pub fn execute_tmux(tmux_command: &str) -> Result<String> {
    // Use bash -c to execute tmux, avoiding zsh plugin conflicts
    let output = Command::new("bash")
        .args(["-c", &format!("tmux -C {}", tmux_command)])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TmuxMcpError::TmuxNotAvailable
            } else {
                TmuxMcpError::TmuxError(e.to_string())
            }
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let err_msg = if stderr.is_empty() { stdout } else { stderr };

        if err_msg.contains("no server running") || err_msg.contains("no such server") {
            return Err(TmuxMcpError::TmuxNotAvailable);
        } else if err_msg.contains("can't find session") || err_msg.contains("no such session") {
            return Err(TmuxMcpError::SessionNotFound(err_msg));
        } else if err_msg.contains("can't find window") || err_msg.contains("no such window") {
            return Err(TmuxMcpError::WindowNotFound(err_msg));
        } else if err_msg.contains("can't find pane") || err_msg.contains("no such pane") {
            return Err(TmuxMcpError::PaneNotFound(err_msg));
        }

        return Err(TmuxMcpError::TmuxError(err_msg));
    }

    Ok(stdout.trim().to_string())
}

#[allow(dead_code)]
pub fn is_tmux_running() -> bool {
    execute_tmux("list-sessions -F '#{session_name}'").is_ok()
}

pub fn list_sessions() -> Result<Vec<TmuxSession>> {
    let format = "#{session_id}:#{session_name}:#{?session_attached,1,0}:#{session_windows}";
    let output = execute_tmux(&format!("list-sessions -F '{}'", format))?;
    Ok(parser::parse_sessions(&output))
}

pub fn find_session_by_name(name: &str) -> Result<Option<TmuxSession>> {
    let sessions = list_sessions()?;
    Ok(sessions.into_iter().find(|s| s.name == name))
}

pub fn list_windows(session_id: &str) -> Result<Vec<TmuxWindow>> {
    let format = "#{window_id}:#{window_name}:#{?window_active,1,0}";
    let output = execute_tmux(&format!(
        "list-windows -t '{}' -F '{}'",
        escape_target(session_id),
        format
    ))?;
    Ok(parser::parse_windows(&output, session_id))
}

pub fn list_panes(window_id: &str) -> Result<Vec<TmuxPane>> {
    let format = "#{pane_id}:#{pane_title}:#{?pane_active,1,0}";
    let output = execute_tmux(&format!(
        "list-panes -t '{}' -F '{}'",
        escape_target(window_id),
        format
    ))?;
    Ok(parser::parse_panes(&output, window_id))
}

pub fn capture_pane_content(
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
    execute_tmux(cmd.trim())
}

pub fn create_session(name: &str) -> Result<TmuxSession> {
    execute_tmux(&format!("new-session -d -s '{}'", name))?;
    find_session_by_name(name)?
        .ok_or_else(|| TmuxMcpError::TmuxError("Failed to create session".to_string()))
}

pub fn create_window(session_id: &str, name: &str) -> Result<TmuxWindow> {
    execute_tmux(&format!(
        "new-window -t '{}' -n '{}'",
        escape_target(session_id),
        name
    ))?;
    let windows = list_windows(session_id)?;
    windows
        .into_iter()
        .find(|w| w.name == name)
        .ok_or_else(|| TmuxMcpError::TmuxError("Failed to create window".to_string()))
}

pub fn kill_session(session_id: &str) -> Result<()> {
    execute_tmux(&format!("kill-session -t '{}'", escape_target(session_id)))?;
    Ok(())
}

pub fn kill_window(window_id: &str) -> Result<()> {
    execute_tmux(&format!("kill-window -t '{}'", escape_target(window_id)))?;
    Ok(())
}

pub fn kill_pane(pane_id: &str) -> Result<()> {
    execute_tmux(&format!("kill-pane -t '{}'", escape_target(pane_id)))?;
    Ok(())
}

pub fn split_pane(target_pane_id: &str, direction: &str, size: Option<u8>) -> Result<TmuxPane> {
    let dir_flag = if direction == "horizontal" {
        "-h"
    } else {
        "-v"
    };
    let size_flag = size
        .filter(|s| *s > 0 && *s < 100)
        .map(|s| format!(" -p {}", s))
        .unwrap_or_default();

    execute_tmux(&format!(
        "split-window {} -t '{}'{}",
        dir_flag,
        escape_target(target_pane_id),
        size_flag
    ))?;

    let window_info = execute_tmux(&format!(
        "display-message -p -t '{}' '#{{window_id}}'",
        escape_target(target_pane_id)
    ))?;

    let panes = list_panes(&window_info)?;
    panes
        .into_iter()
        .last()
        .ok_or_else(|| TmuxMcpError::TmuxError("Failed to split pane".to_string()))
}

pub fn send_keys(pane_id: &str, keys: &str, is_special: bool) -> Result<()> {
    if is_special {
        execute_tmux(&format!(
            "send-keys -t '{}' {}",
            escape_target(pane_id),
            keys
        ))?;
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
            ))?;
        }
    }
    Ok(())
}

pub fn send_keys_enter(pane_id: &str, command: &str) -> Result<()> {
    let escaped = command.replace('\'', "'\\''");
    execute_tmux(&format!(
        "send-keys -t '{}' '{}' Enter",
        escape_target(pane_id),
        escaped
    ))?;
    Ok(())
}

fn escape_target(target: &str) -> String {
    target.replace('\'', "'\\''")
}

pub fn get_end_marker_text(shell_type: ShellType) -> String {
    format!("TMUX_MCP_DONE_{}", shell_type.exit_code_var())
}
