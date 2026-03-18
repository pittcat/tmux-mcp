use std::sync::Arc;

use axum::extract::{Extension, Path};
use axum::response::Json as AxumJson;
use serde::Serialize;
use serde_json::json;

use crate::error::Result;
use crate::state::command_registry::CommandRegistry;
use crate::tmux::service;

#[derive(Debug, Serialize)]
struct Resource {
    name: String,
    uri: String,
    description: String,
}

pub async fn list_resources(
    Extension(registry): Extension<Arc<CommandRegistry>>,
) -> AxumJson<serde_json::Value> {
    let mut resources = vec![];

    // Add sessions resource
    resources.push(Resource {
        name: "Tmux Sessions".to_string(),
        uri: "tmux://sessions".to_string(),
        description: "List of all tmux sessions".to_string(),
    });

    // Add pane resources
    if let Ok(sessions) = service::list_sessions().await {
        for session in sessions {
            if let Ok(windows) = service::list_windows(&session.id).await {
                for window in windows {
                    if let Ok(panes) = service::list_panes(&window.id).await {
                        for pane in panes {
                            resources.push(Resource {
                                name: format!(
                                    "Pane: {} - {} - {} {}",
                                    session.name,
                                    pane.id,
                                    pane.title,
                                    if pane.active { "(active)" } else { "" }
                                ),
                                uri: format!("tmux://pane/{}", pane.id),
                                description: format!(
                                    "Content from pane {} - {} in session {}",
                                    pane.id, pane.title, session.name
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    // Add command resources
    registry.cleanup_expired();
    for cmd in registry.list_active() {
        resources.push(Resource {
            name: format!(
                "Command: {}{}",
                &cmd.command[..cmd.command.len().min(30)],
                if cmd.command.len() > 30 { "..." } else { "" }
            ),
            uri: format!("tmux://command/{}/result", cmd.id),
            description: format!("Execution status: {:?}", cmd.status),
        });
    }

    AxumJson(json!({ "resources": resources }))
}

pub async fn read_resource(
    Path(uri): Path<String>,
    Extension(registry): Extension<Arc<CommandRegistry>>,
) -> Result<AxumJson<serde_json::Value>> {
    let result = if uri == "tmux://sessions" {
        handle_read_sessions().await?
    } else if uri.starts_with("tmux://pane/") {
        let pane_id = uri.strip_prefix("tmux://pane/").unwrap_or("");
        handle_read_pane(pane_id).await?
    } else if uri.starts_with("tmux://command/") && uri.ends_with("/result") {
        let command_id = uri
            .strip_prefix("tmux://command/")
            .and_then(|s| s.strip_suffix("/result"))
            .unwrap_or("");
        handle_read_command_result(registry, command_id).await?
    } else {
        return Ok(AxumJson(json!({
            "contents": [{
                "uri": uri,
                "text": format!("Unknown resource URI: {}", uri)
            }]
        })));
    };

    Ok(AxumJson(result))
}

async fn handle_read_sessions() -> Result<serde_json::Value> {
    let sessions = service::list_sessions().await?;
    let session_data: Vec<_> = sessions
        .into_iter()
        .map(|s| {
            json!({
                "id": s.id,
                "name": s.name,
                "attached": s.attached,
                "windows": s.windows
            })
        })
        .collect();

    Ok(json!({
        "contents": [{
            "uri": "tmux://sessions",
            "text": serde_json::to_string_pretty(&session_data)?
        }]
    }))
}

async fn handle_read_pane(pane_id: &str) -> Result<serde_json::Value> {
    let content = service::capture_pane_content(pane_id, Some(200), false).await?;

    Ok(json!({
        "contents": [{
            "uri": format!("tmux://pane/{}", pane_id),
            "text": content
        }]
    }))
}

async fn handle_read_command_result(
    registry: Arc<CommandRegistry>,
    command_id: &str,
) -> Result<serde_json::Value> {
    use crate::tmux::command;

    let command = command::check_command_status(registry, command_id.to_string()).await?;

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
        "contents": [{
            "uri": format!("tmux://command/{}/result", command_id),
            "text": text
        }]
    }))
}
