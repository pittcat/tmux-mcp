//! MCP JSON-RPC 2.0 Protocol Implementation
//!
//! This module implements the standard MCP protocol over HTTP using JSON-RPC 2.0

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Json, Response,
    },
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_stream::{wrappers::IntervalStream, StreamExt};

use crate::state::command_registry::CommandRegistry;

const SUPPORTED_PROTOCOL_VERSION: &str = "2025-03-26";
const JSONRPC_VERSION: &str = "2.0";
const PROTOCOL_VERSION_HEADER: &str = "MCP-Protocol-Version";

/// Standard JSON-RPC 2.0 message
#[derive(Debug, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Standard JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Application state for MCP protocol
#[derive(Clone)]
pub struct ProtocolState {
    pub command_registry: Arc<CommandRegistry>,
}

/// Handle incoming JSON-RPC messages over Streamable HTTP.
pub async fn handle_json_rpc(
    State(state): State<ProtocolState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let req = match serde_json::from_slice::<JsonRpcMessage>(&body) {
        Ok(req) => req,
        Err(error) => {
            return protocol_error_response(
                StatusCode::BAD_REQUEST,
                None,
                JsonRpcError {
                    code: -32700,
                    message: format!("Invalid JSON body: {}", error),
                    data: None,
                },
            );
        }
    };

    let id = req.id.clone();

    // Validate JSON-RPC version
    if req.jsonrpc != JSONRPC_VERSION {
        return protocol_error_response(
            StatusCode::BAD_REQUEST,
            id,
            JsonRpcError {
                code: -32600,
                message: "Invalid JSON-RPC version".to_string(),
                data: None,
            },
        );
    }

    if req.method != "initialize" {
        if let Err(message) = validate_protocol_version_header(&headers) {
            return protocol_error_response(
                StatusCode::BAD_REQUEST,
                id,
                JsonRpcError {
                    code: -32600,
                    message,
                    data: None,
                },
            );
        }
    }

    let result = match req.method.as_str() {
        "initialize" => handle_initialize(&req.params).await,
        "notifications/initialized" => Ok(json!({})),
        "tools/list" => handle_tools_list().await,
        "tools/call" => handle_tools_call(&state, req.params).await,
        "resources/list" => handle_resources_list().await,
        "resources/read" => handle_resources_read(&state, req.params).await,
        "ping" => handle_ping().await,
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Method not found: {}", req.method),
            data: None,
        }),
    };

    if let Some(id) = id {
        match result {
            Ok(result) => protocol_json_response(
                StatusCode::OK,
                JsonRpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id,
                    result: Some(result),
                    error: None,
                },
            ),
            Err(error) => protocol_json_response(
                StatusCode::OK,
                JsonRpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id,
                    result: None,
                    error: Some(error),
                },
            ),
        }
    } else {
        match result {
            Ok(_) => with_protocol_version_header((StatusCode::ACCEPTED, "").into_response()),
            Err(error) => protocol_error_response(StatusCode::BAD_REQUEST, None, error),
        }
    }
}

pub async fn handle_sse_stream() -> impl IntoResponse {
    let stream = IntervalStream::new(tokio::time::interval(Duration::from_secs(30)))
        .map(|_| Ok::<Event, Infallible>(Event::default().comment("keep-alive")));

    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    );

    with_protocol_version_header(sse.into_response())
}

async fn handle_initialize(params: &serde_json::Value) -> Result<serde_json::Value, JsonRpcError> {
    let negotiated_protocol_version = negotiate_protocol_version(params)?;

    Ok(json!({
        "protocolVersion": negotiated_protocol_version,
        "capabilities": {
            "tools": {},
            "resources": {}
        },
        "serverInfo": {
            "name": "tmux-mcp-server",
            "version": "1.0.0"
        }
    }))
}

fn protocol_json_response(status: StatusCode, body: JsonRpcResponse) -> Response {
    with_protocol_version_header((status, Json(body)).into_response())
}

fn protocol_error_response(
    status: StatusCode,
    id: Option<serde_json::Value>,
    error: JsonRpcError,
) -> Response {
    if let Some(id) = id {
        protocol_json_response(
            status,
            JsonRpcResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                id,
                result: None,
                error: Some(error),
            },
        )
    } else {
        with_protocol_version_header(
            (
                status,
                Json(json!({
                    "jsonrpc": JSONRPC_VERSION,
                    "error": error,
                })),
            )
                .into_response(),
        )
    }
}

fn with_protocol_version_header(mut response: Response) -> Response {
    response.headers_mut().insert(
        PROTOCOL_VERSION_HEADER,
        HeaderValue::from_static(SUPPORTED_PROTOCOL_VERSION),
    );
    response
}

fn validate_protocol_version_header(headers: &HeaderMap) -> Result<(), String> {
    let Some(value) = headers.get(PROTOCOL_VERSION_HEADER) else {
        return Ok(());
    };

    let value = value
        .to_str()
        .map_err(|_| format!("Invalid {} header encoding", PROTOCOL_VERSION_HEADER))?;

    if !is_valid_protocol_version(value) {
        return Err(format!("Invalid {} header value", PROTOCOL_VERSION_HEADER));
    }

    if value != SUPPORTED_PROTOCOL_VERSION {
        return Err(format!(
            "Unsupported {} header value: {}",
            PROTOCOL_VERSION_HEADER, value
        ));
    }

    Ok(())
}

fn negotiate_protocol_version(params: &serde_json::Value) -> Result<&'static str, JsonRpcError> {
    let requested = params
        .get("protocolVersion")
        .and_then(|value| value.as_str())
        .unwrap_or(SUPPORTED_PROTOCOL_VERSION);

    if !is_valid_protocol_version(requested) {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid protocolVersion".to_string(),
            data: None,
        });
    }

    if requested < SUPPORTED_PROTOCOL_VERSION {
        return Err(JsonRpcError {
            code: -32602,
            message: format!("Unsupported protocolVersion: {}", requested),
            data: None,
        });
    }

    Ok(SUPPORTED_PROTOCOL_VERSION)
}

fn is_valid_protocol_version(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(idx, byte)| matches!(idx, 4 | 7) || byte.is_ascii_digit())
}

async fn handle_tools_list() -> Result<serde_json::Value, JsonRpcError> {
    // Return same tools as the REST API
    let tools = vec![
        json!({
            "name": "list-sessions",
            "description": "List all active tmux sessions",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "find-session",
            "description": "Find a tmux session by name",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the tmux session to find"
                    }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "list-windows",
            "description": "List windows in a tmux session",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sessionId": {
                        "type": "string",
                        "description": "ID of the tmux session"
                    }
                },
                "required": ["sessionId"]
            }
        }),
        json!({
            "name": "list-panes",
            "description": "List panes in a tmux window",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "windowId": {
                        "type": "string",
                        "description": "ID of the tmux window"
                    }
                },
                "required": ["windowId"]
            }
        }),
        json!({
            "name": "capture-pane",
            "description": "Capture content from a tmux pane",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paneId": {
                        "type": "string",
                        "description": "ID of the tmux pane"
                    },
                    "lines": {
                        "type": "number",
                        "description": "Number of lines to capture"
                    },
                    "colors": {
                        "type": "boolean",
                        "description": "Include color/escape sequences"
                    }
                },
                "required": ["paneId"]
            }
        }),
        json!({
            "name": "create-session",
            "description": "Create a new tmux session",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the new tmux session"
                    }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "create-window",
            "description": "Create a new window in a tmux session",
            "inputSchema": {
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
            }
        }),
        json!({
            "name": "kill-session",
            "description": "Kill a tmux session by ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sessionId": {
                        "type": "string",
                        "description": "ID of the tmux session to kill"
                    }
                },
                "required": ["sessionId"]
            }
        }),
        json!({
            "name": "kill-window",
            "description": "Kill a tmux window by ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "windowId": {
                        "type": "string",
                        "description": "ID of the tmux window to kill"
                    }
                },
                "required": ["windowId"]
            }
        }),
        json!({
            "name": "kill-pane",
            "description": "Kill a tmux pane by ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paneId": {
                        "type": "string",
                        "description": "ID of the tmux pane to kill"
                    }
                },
                "required": ["paneId"]
            }
        }),
        json!({
            "name": "split-pane",
            "description": "Split a tmux pane horizontally or vertically",
            "inputSchema": {
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
            }
        }),
        json!({
            "name": "execute-command",
            "description": "Execute a command in a tmux pane",
            "inputSchema": {
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
            }
        }),
        json!({
            "name": "get-command-result",
            "description": "Get the result of an executed command",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "commandId": {
                        "type": "string",
                        "description": "ID of the executed command"
                    }
                },
                "required": ["commandId"]
            }
        }),
    ];

    Ok(json!({ "tools": tools }))
}

async fn handle_tools_call(
    state: &ProtocolState,
    params: serde_json::Value,
) -> Result<serde_json::Value, JsonRpcError> {
    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Missing tool name".to_string(),
            data: None,
        })?;

    let tool_args = params.get("arguments").cloned().unwrap_or(json!({}));

    // Delegate to existing tools implementation
    match tool_name {
        "list-sessions" => match crate::tmux::service::list_sessions().await {
            Ok(sessions) => Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&sessions).unwrap_or_default()
                }]
            })),
            Err(e) => Err(JsonRpcError {
                code: -32000,
                message: e.to_string(),
                data: None,
            }),
        },
        "find-session" => {
            let name = tool_args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing name parameter".to_string(),
                    data: None,
                })?;
            match crate::tmux::service::find_session_by_name(name).await {
                Ok(session) => {
                    let text = match session {
                        Some(s) => serde_json::to_string_pretty(&s).unwrap_or_default(),
                        None => format!("Session not found: {}", name),
                    };
                    Ok(json!({ "content": [{ "type": "text", "text": text }] }))
                }
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "list-windows" => {
            let session_id = tool_args
                .get("sessionId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing sessionId parameter".to_string(),
                    data: None,
                })?;
            match crate::tmux::service::list_windows(session_id).await {
                Ok(windows) => Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&windows).unwrap_or_default()
                    }]
                })),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "list-panes" => {
            let window_id = tool_args
                .get("windowId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing windowId parameter".to_string(),
                    data: None,
                })?;
            match crate::tmux::service::list_panes(window_id).await {
                Ok(panes) => Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&panes).unwrap_or_default()
                    }]
                })),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "capture-pane" => {
            let pane_id = tool_args
                .get("paneId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing paneId parameter".to_string(),
                    data: None,
                })?;
            let lines = tool_args
                .get("lines")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let colors = tool_args
                .get("colors")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            match crate::tmux::service::capture_pane_content(pane_id, lines, colors).await {
                Ok(content) => Ok(json!({
                    "content": [{ "type": "text", "text": content }]
                })),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "create-session" => {
            let name = tool_args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing name parameter".to_string(),
                    data: None,
                })?;
            match crate::tmux::service::create_session(name).await {
                Ok(session) => Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Session created: {}", serde_json::to_string_pretty(&session).unwrap_or_default())
                    }]
                })),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "create-window" => {
            let session_id = tool_args
                .get("sessionId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing sessionId parameter".to_string(),
                    data: None,
                })?;
            let name = tool_args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing name parameter".to_string(),
                    data: None,
                })?;
            match crate::tmux::service::create_window(session_id, name).await {
                Ok(window) => Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Window created: {}", serde_json::to_string_pretty(&window).unwrap_or_default())
                    }]
                })),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "kill-session" => {
            let session_id = tool_args
                .get("sessionId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing sessionId parameter".to_string(),
                    data: None,
                })?;
            match crate::tmux::service::kill_session(session_id).await {
                Ok(_) => Ok(json!({
                    "content": [{ "type": "text", "text": format!("Session {} has been killed", session_id) }]
                })),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "kill-window" => {
            let window_id = tool_args
                .get("windowId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing windowId parameter".to_string(),
                    data: None,
                })?;
            match crate::tmux::service::kill_window(window_id).await {
                Ok(_) => Ok(json!({
                    "content": [{ "type": "text", "text": format!("Window {} has been killed", window_id) }]
                })),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "kill-pane" => {
            let pane_id = tool_args
                .get("paneId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing paneId parameter".to_string(),
                    data: None,
                })?;
            match crate::tmux::service::kill_pane(pane_id).await {
                Ok(_) => Ok(json!({
                    "content": [{ "type": "text", "text": format!("Pane {} has been killed", pane_id) }]
                })),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "split-pane" => {
            let pane_id = tool_args
                .get("paneId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing paneId parameter".to_string(),
                    data: None,
                })?;
            let direction = tool_args
                .get("direction")
                .and_then(|v| v.as_str())
                .unwrap_or("vertical");
            let size = tool_args
                .get("size")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8);
            match crate::tmux::service::split_pane(pane_id, direction, size).await {
                Ok(pane) => Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Pane split successfully. New pane: {}", serde_json::to_string_pretty(&pane).unwrap_or_default())
                    }]
                })),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "execute-command" => {
            let pane_id = tool_args
                .get("paneId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing paneId parameter".to_string(),
                    data: None,
                })?;
            let command = tool_args
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing command parameter".to_string(),
                    data: None,
                })?;
            let raw_mode = tool_args
                .get("rawMode")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let no_enter = tool_args
                .get("noEnter")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let shell_type = crate::tmux::models::ShellType::Bash;

            match crate::tmux::command::execute_command(
                state.command_registry.clone(),
                pane_id.to_string(),
                command.to_string(),
                raw_mode,
                no_enter,
                shell_type,
            )
            .await
            {
                Ok(command_id) => {
                    if raw_mode || no_enter {
                        let mode_text = if no_enter {
                            "Keys sent without Enter"
                        } else {
                            "Interactive command started (rawMode)"
                        };
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("{}\n\nStatus tracking is disabled.\nUse 'capture-pane' with paneId '{}' to verify the command outcome.\n\nCommand ID: {}", mode_text, pane_id, command_id)
                            }]
                        }))
                    } else {
                        let resource_uri = format!("tmux://command/{}/result", command_id);
                        Ok(json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Command execution started.\n\nTo get results, subscribe to and read resource: {}\n\nStatus will change from 'pending' to 'completed' or 'error' when finished.", resource_uri)
                            }]
                        }))
                    }
                }
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        "get-command-result" => {
            let command_id = tool_args
                .get("commandId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Missing commandId parameter".to_string(),
                    data: None,
                })?;

            match crate::tmux::command::check_command_status(
                state.command_registry.clone(),
                command_id.to_string(),
            )
            .await
            {
                Ok(Some(cmd)) => {
                    let text = if cmd.status == crate::tmux::models::CommandStatus::Pending {
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
                    };
                    Ok(json!({ "content": [{ "type": "text", "text": text }] }))
                }
                Ok(None) => Err(JsonRpcError {
                    code: -32000,
                    message: format!("Command not found: {}", command_id),
                    data: None,
                }),
                Err(e) => Err(JsonRpcError {
                    code: -32000,
                    message: e.to_string(),
                    data: None,
                }),
            }
        }
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Unknown tool: {}", tool_name),
            data: None,
        }),
    }
}

async fn handle_resources_list() -> Result<serde_json::Value, JsonRpcError> {
    Ok(json!({
        "resources": [
            {
                "uri": "tmux://sessions",
                "name": "List all tmux sessions",
                "mimeType": "application/json",
                "description": "Returns a JSON array of all active tmux sessions"
            }
        ]
    }))
}

async fn handle_resources_read(
    state: &ProtocolState,
    params: serde_json::Value,
) -> Result<serde_json::Value, JsonRpcError> {
    let uri = params
        .get("uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Missing uri parameter".to_string(),
            data: None,
        })?;

    // Parse resource URI
    if uri.starts_with("tmux://sessions") {
        match crate::tmux::service::list_sessions().await {
            Ok(sessions) => Ok(json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&sessions).unwrap_or_default()
                }]
            })),
            Err(e) => Err(JsonRpcError {
                code: -32000,
                message: e.to_string(),
                data: None,
            }),
        }
    } else if uri.starts_with("tmux://command/") {
        // Parse command result resource
        let command_id = uri
            .trim_start_matches("tmux://command/")
            .trim_end_matches("/result");
        match crate::tmux::command::check_command_status(
            state.command_registry.clone(),
            command_id.to_string(),
        )
        .await
        {
            Ok(Some(cmd)) => {
                let text = format!(
                    "Status: {:?}\nExit code: {}\nCommand: {}\n\nOutput:\n{}",
                    cmd.status,
                    cmd.exit_code.unwrap_or(-1),
                    cmd.command,
                    cmd.result.unwrap_or_default()
                );
                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/plain",
                        "text": text
                    }]
                }))
            }
            Ok(None) => Err(JsonRpcError {
                code: -32000,
                message: format!("Command not found: {}", command_id),
                data: None,
            }),
            Err(e) => Err(JsonRpcError {
                code: -32000,
                message: e.to_string(),
                data: None,
            }),
        }
    } else {
        Err(JsonRpcError {
            code: -32000,
            message: format!("Unknown resource: {}", uri),
            data: None,
        })
    }
}

async fn handle_ping() -> Result<serde_json::Value, JsonRpcError> {
    Ok(json!({ "pong": true }))
}

/// Create the MCP protocol router
pub fn create_protocol_router(command_registry: Arc<CommandRegistry>) -> Router {
    Router::new()
        .route("/mcp", get(handle_sse_stream).post(handle_json_rpc))
        .route("/mcp/", get(handle_sse_stream).post(handle_json_rpc))
        .with_state(ProtocolState { command_registry })
}
