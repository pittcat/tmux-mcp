use std::sync::Arc;

use axum::extract::{Extension, Json, Path};
use axum::response::Json as AxumJson;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{Result, TmuxMcpError};
use crate::state::command_registry::CommandRegistry;
use crate::tmux::command;
use crate::tmux::models::ShellType;
use crate::tmux::service;

#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CallToolRequest {
    #[serde(flatten)]
    params: serde_json::Value,
}

pub async fn list_tools() -> AxumJson<serde_json::Value> {
    let tools = vec![
        Tool {
            name: "list-sessions".to_string(),
            description: "List all active tmux sessions".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "find-session".to_string(),
            description: "Find a tmux session by name".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the tmux session to find"
                    }
                },
                "required": ["name"]
            }),
        },
        Tool {
            name: "list-windows".to_string(),
            description: "List windows in a tmux session".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "sessionId": {
                        "type": "string",
                        "description": "ID of the tmux session"
                    }
                },
                "required": ["sessionId"]
            }),
        },
        Tool {
            name: "list-panes".to_string(),
            description: "List panes in a tmux window".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "windowId": {
                        "type": "string",
                        "description": "ID of the tmux window"
                    }
                },
                "required": ["windowId"]
            }),
        },
        Tool {
            name: "capture-pane".to_string(),
            description: "Capture content from a tmux pane".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paneId": {
                        "type": "string",
                        "description": "ID of the tmux pane"
                    },
                    "lines": {
                        "type": "string",
                        "description": "Number of lines to capture"
                    },
                    "colors": {
                        "type": "boolean",
                        "description": "Include color/escape sequences"
                    }
                },
                "required": ["paneId"]
            }),
        },
        Tool {
            name: "create-session".to_string(),
            description: "Create a new tmux session".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the new tmux session"
                    }
                },
                "required": ["name"]
            }),
        },
        Tool {
            name: "create-window".to_string(),
            description: "Create a new window in a tmux session".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "sessionId": {
                        "type": "string",
                        "description": "ID of the tmux session"
                    },
                    "name": {
                        "type": "string",
                        "description": "Name for the new window"
                    }
                },
                "required": ["sessionId", "name"]
            }),
        },
        Tool {
            name: "kill-session".to_string(),
            description: "Kill a tmux session by ID".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "sessionId": {
                        "type": "string",
                        "description": "ID of the tmux session to kill"
                    }
                },
                "required": ["sessionId"]
            }),
        },
        Tool {
            name: "kill-window".to_string(),
            description: "Kill a tmux window by ID".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "windowId": {
                        "type": "string",
                        "description": "ID of the tmux window to kill"
                    }
                },
                "required": ["windowId"]
            }),
        },
        Tool {
            name: "kill-pane".to_string(),
            description: "Kill a tmux pane by ID".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paneId": {
                        "type": "string",
                        "description": "ID of the tmux pane to kill"
                    }
                },
                "required": ["paneId"]
            }),
        },
        Tool {
            name: "split-pane".to_string(),
            description: "Split a tmux pane horizontally or vertically".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paneId": {
                        "type": "string",
                        "description": "ID of the tmux pane to split"
                    },
                    "direction": {
                        "type": "string",
                        "description": "Split direction: 'horizontal' or 'vertical'"
                    },
                    "size": {
                        "type": "number",
                        "description": "Size of new pane as percentage (1-99)"
                    }
                },
                "required": ["paneId"]
            }),
        },
        Tool {
            name: "execute-command".to_string(),
            description: "Execute a command in a tmux pane".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "paneId": {
                        "type": "string",
                        "description": "ID of the tmux pane"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    },
                    "rawMode": {
                        "type": "boolean",
                        "description": "Execute without wrapper markers"
                    },
                    "noEnter": {
                        "type": "boolean",
                        "description": "Send keystrokes without pressing Enter"
                    }
                },
                "required": ["paneId", "command"]
            }),
        },
        Tool {
            name: "get-command-result".to_string(),
            description: "Get the result of an executed command".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "commandId": {
                        "type": "string",
                        "description": "ID of the executed command"
                    }
                },
                "required": ["commandId"]
            }),
        },
    ];

    AxumJson(json!({ "tools": tools }))
}

pub async fn call_tool(
    Path(name): Path<String>,
    Extension(registry): Extension<Arc<CommandRegistry>>,
    Json(req): Json<CallToolRequest>,
) -> Result<AxumJson<serde_json::Value>> {
    let result = match name.as_str() {
        "list-sessions" => handle_list_sessions().await?,
        "find-session" => handle_find_session(req.params).await?,
        "list-windows" => handle_list_windows(req.params).await?,
        "list-panes" => handle_list_panes(req.params).await?,
        "capture-pane" => handle_capture_pane(req.params).await?,
        "create-session" => handle_create_session(req.params).await?,
        "create-window" => handle_create_window(req.params).await?,
        "kill-session" => handle_kill_session(req.params).await?,
        "kill-window" => handle_kill_window(req.params).await?,
        "kill-pane" => handle_kill_pane(req.params).await?,
        "split-pane" => handle_split_pane(req.params).await?,
        "execute-command" => handle_execute_command(registry, req.params).await?,
        "get-command-result" => handle_get_command_result(registry, req.params).await?,
        _ => {
            return Err(TmuxMcpError::InvalidParameter(format!(
                "Unknown tool: {}",
                name
            )))
        }
    };

    Ok(AxumJson(result))
}

async fn handle_list_sessions() -> Result<serde_json::Value> {
    let sessions = service::list_sessions().await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&sessions)?
        }]
    }))
}

async fn handle_find_session(params: serde_json::Value) -> Result<serde_json::Value> {
    let name = params["name"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("name required".to_string()))?;

    let session = service::find_session_by_name(name).await?;
    let text = match session {
        Some(s) => serde_json::to_string_pretty(&s)?,
        None => format!("Session not found: {}", name),
    };

    Ok(json!({
        "content": [{"type": "text", "text": text}]
    }))
}

async fn handle_list_windows(params: serde_json::Value) -> Result<serde_json::Value> {
    let session_id = params["sessionId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("sessionId required".to_string()))?;

    let windows = service::list_windows(session_id).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&windows)?
        }]
    }))
}

async fn handle_list_panes(params: serde_json::Value) -> Result<serde_json::Value> {
    let window_id = params["windowId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("windowId required".to_string()))?;

    let panes = service::list_panes(window_id).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&panes)?
        }]
    }))
}

async fn handle_capture_pane(params: serde_json::Value) -> Result<serde_json::Value> {
    let pane_id = params["paneId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("paneId required".to_string()))?;

    let lines = params["lines"].as_str().and_then(|s| s.parse().ok());

    let colors = params["colors"].as_bool().unwrap_or(false);

    let content = service::capture_pane_content(pane_id, lines, colors).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": content
        }]
    }))
}

async fn handle_create_session(params: serde_json::Value) -> Result<serde_json::Value> {
    let name = params["name"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("name required".to_string()))?;

    let session = service::create_session(name).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Session created: {}", serde_json::to_string_pretty(&session)?)
        }]
    }))
}

async fn handle_create_window(params: serde_json::Value) -> Result<serde_json::Value> {
    let session_id = params["sessionId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("sessionId required".to_string()))?;

    let name = params["name"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("name required".to_string()))?;

    let window = service::create_window(session_id, name).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Window created: {}", serde_json::to_string_pretty(&window)?)
        }]
    }))
}

async fn handle_kill_session(params: serde_json::Value) -> Result<serde_json::Value> {
    let session_id = params["sessionId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("sessionId required".to_string()))?;

    service::kill_session(session_id).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Session {} has been killed", session_id)
        }]
    }))
}

async fn handle_kill_window(params: serde_json::Value) -> Result<serde_json::Value> {
    let window_id = params["windowId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("windowId required".to_string()))?;

    service::kill_window(window_id).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Window {} has been killed", window_id)
        }]
    }))
}

async fn handle_kill_pane(params: serde_json::Value) -> Result<serde_json::Value> {
    let pane_id = params["paneId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("paneId required".to_string()))?;

    service::kill_pane(pane_id).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Pane {} has been killed", pane_id)
        }]
    }))
}

async fn handle_split_pane(params: serde_json::Value) -> Result<serde_json::Value> {
    let pane_id = params["paneId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("paneId required".to_string()))?;

    let direction = params["direction"].as_str().unwrap_or("vertical");

    let size = params["size"].as_u64().map(|s| s as u8);

    let pane = service::split_pane(pane_id, direction, size).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!(
                "Pane split successfully. New pane: {}",
                serde_json::to_string_pretty(&pane)?
            )
        }]
    }))
}

async fn handle_execute_command(
    registry: Arc<CommandRegistry>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let pane_id = params["paneId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("paneId required".to_string()))?;

    let command = params["command"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("command required".to_string()))?;

    let raw_mode = params["rawMode"].as_bool().unwrap_or(false);
    let no_enter = params["noEnter"].as_bool().unwrap_or(false);

    // Shell type from environment or default
    let shell_type = ShellType::Bash; // TODO: make configurable

    let command_id = command::execute_command(
        registry,
        pane_id.to_string(),
        command.to_string(),
        raw_mode,
        no_enter,
        shell_type,
    )
    .await?;

    if raw_mode || no_enter {
        let mode_text = if no_enter {
            "Keys sent without Enter"
        } else {
            "Interactive command started (rawMode)"
        };

        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "{}\n\nStatus tracking is disabled.\nUse 'capture-pane' with paneId '{}' to verify the command outcome.\n\nCommand ID: {}",
                    mode_text, pane_id, command_id
                )
            }]
        }))
    } else {
        let resource_uri = format!("tmux://command/{}/result", command_id);
        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "Command execution started.\n\nTo get results, subscribe to and read resource: {}\n\nStatus will change from 'pending' to 'completed' or 'error' when finished.",
                    resource_uri
                )
            }]
        }))
    }
}

async fn handle_get_command_result(
    registry: Arc<CommandRegistry>,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let command_id = params["commandId"]
        .as_str()
        .ok_or_else(|| TmuxMcpError::InvalidParameter("commandId required".to_string()))?;

    let command = command::check_command_status(registry, command_id.to_string()).await?;

    let is_none = command.is_none();

    let text = match command {
        Some(cmd) => {
            if cmd.status == crate::tmux::models::CommandStatus::Pending {
                if let Some(result) = cmd.result {
                    format!(
                        "Status: pending\nCommand: {}\n\n--- Message ---\n{}",
                        cmd.command, result
                    )
                } else {
                    format!(
                        "Command still executing...\nStarted: {}\nCommand: {}",
                        cmd.start_time.to_rfc3339(),
                        cmd.command
                    )
                }
            } else {
                format!(
                    "Status: {:?}\nExit code: {}\nCommand: {}\n\n--- Output ---\n{}",
                    cmd.status,
                    cmd.exit_code.unwrap_or(-1),
                    cmd.command,
                    cmd.result.unwrap_or_default()
                )
            }
        }
        None => format!("Command not found: {}", command_id),
    };

    Ok(json!({
        "content": [{"type": "text", "text": text}],
        "isError": is_none
    }))
}
