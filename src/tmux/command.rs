use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::error::Result;
use crate::state::command_registry::CommandRegistry;
use crate::tmux::models::{CommandExecution, CommandStatus, ShellType};
use crate::tmux::service;

pub async fn execute_command(
    registry: Arc<CommandRegistry>,
    pane_id: String,
    command: String,
    raw_mode: bool,
    no_enter: bool,
    shell_type: ShellType,
) -> Result<String> {
    let command_id = Uuid::new_v4().to_string();

    let execution = CommandExecution {
        id: command_id.clone(),
        pane_id: pane_id.clone(),
        command: command.clone(),
        status: CommandStatus::Pending,
        start_time: Utc::now(),
        result: None,
        exit_code: None,
        raw_mode: raw_mode || no_enter,
    };

    registry.insert(command_id.clone(), execution);

    if no_enter {
        let special_keys = [
            "Up", "Down", "Left", "Right", "Escape", "Tab", "Enter", "Space", "BSpace", "Delete",
            "Home", "End", "PageUp", "PageDown", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8",
            "F9", "F10", "F11", "F12",
        ];
        let is_special = special_keys.contains(&command.as_str());
        service::send_keys(&pane_id, &command, is_special)?;
    } else if raw_mode {
        service::send_keys_enter(&pane_id, &command)?;
    } else {
        let start_marker = "TMUX_MCP_START";
        let end_marker = service::get_end_marker_text(shell_type);
        let full_command = format!(
            "echo \"{}\"; {}; echo \"{}\"",
            start_marker, command, end_marker
        );
        service::send_keys_enter(&pane_id, &full_command)?;
    }

    Ok(command_id)
}

pub async fn check_command_status(
    registry: Arc<CommandRegistry>,
    command_id: String,
) -> Result<Option<CommandExecution>> {
    let mut execution = match registry.get(&command_id) {
        Some(exec) => exec,
        None => return Ok(None),
    };

    if execution.status != CommandStatus::Pending {
        return Ok(Some(execution));
    }

    if execution.raw_mode {
        execution.result = Some(
            "Status tracking unavailable for rawMode commands. Use capture-pane to monitor interactive apps instead."
                .to_string(),
        );
        return Ok(Some(execution));
    }

    let content = service::capture_pane_content(&execution.pane_id, Some(1000), false)?;

    let start_marker = "TMUX_MCP_START";
    let end_marker_prefix = "TMUX_MCP_DONE_";

    let start_index = match content.rfind(start_marker) {
        Some(idx) => idx,
        None => {
            execution.result = Some("Command output could not be captured properly".to_string());
            return Ok(Some(execution));
        }
    };

    let end_index = match content.rfind(end_marker_prefix) {
        Some(idx) => idx,
        None => {
            execution.result = Some("Command output could not be captured properly".to_string());
            return Ok(Some(execution));
        }
    };

    if end_index <= start_index {
        execution.result = Some("Command output could not be captured properly".to_string());
        return Ok(Some(execution));
    }

    let end_line = &content[end_index..];
    let end_line_end = end_line.find('\n').unwrap_or(end_line.len());
    let end_marker_line = &end_line[..end_line_end];

    let exit_code = end_marker_line
        .strip_prefix(end_marker_prefix)
        .and_then(|s| s.trim().parse().ok());

    if let Some(code) = exit_code {
        execution.status = if code == 0 {
            CommandStatus::Completed
        } else {
            CommandStatus::Error
        };
        execution.exit_code = Some(code);

        let output_start = start_index + start_marker.len();
        let output_content = &content[output_start..end_index];
        let result = output_content
            .find('\n')
            .map(|i| output_content[i + 1..].trim().to_string())
            .unwrap_or_else(|| output_content.trim().to_string());

        execution.result = Some(result);

        registry.insert(command_id, execution.clone());
    }

    Ok(Some(execution))
}
