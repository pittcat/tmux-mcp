//! Tests for tool contract validation

use serde_json::json;

#[test]
fn test_tool_input_schema_structure() {
    // Test that tools have required inputSchema structure
    let tools = get_tool_definitions();

    for tool in &tools {
        assert!(tool["inputSchema"].is_object());
        assert!(tool["inputSchema"]["type"].is_string());
        assert_eq!(tool["inputSchema"]["type"], "object");
    }
}

#[test]
fn test_list_sessions_tool_has_no_required_params() {
    let tools = get_tool_definitions();
    let tool = tools.iter().find(|t| t["name"] == "list-sessions").unwrap();

    if let Some(required) = tool["inputSchema"]["required"].as_array() {
        assert!(required.is_empty());
    } else {
        // If required field doesn't exist or isn't an array, that's also acceptable for no params
    }
}

#[test]
fn test_capture_pane_requires_pane_id() {
    let tools = get_tool_definitions();
    let tool = tools.iter().find(|t| t["name"] == "capture-pane").unwrap();

    let required = tool["inputSchema"]["required"].as_array().unwrap();
    assert!(required.contains(&json!("paneId")));
}

#[test]
fn test_create_session_requires_name() {
    let tools = get_tool_definitions();
    let tool = tools
        .iter()
        .find(|t| t["name"] == "create-session")
        .unwrap();

    let required = tool["inputSchema"]["required"].as_array().unwrap();
    assert!(required.contains(&json!("name")));
}

#[test]
fn test_execute_command_requires_pane_id_and_command() {
    let tools = get_tool_definitions();
    let tool = tools
        .iter()
        .find(|t| t["name"] == "execute-command")
        .unwrap();

    let required = tool["inputSchema"]["required"].as_array().unwrap();
    assert!(required.contains(&json!("paneId")));
    assert!(required.contains(&json!("command")));
}

#[test]
fn test_execute_command_optional_raw_mode_and_no_enter() {
    let tools = get_tool_definitions();
    let tool = tools
        .iter()
        .find(|t| t["name"] == "execute-command")
        .unwrap();

    let props = tool["inputSchema"]["properties"].as_object().unwrap();
    assert!(props.contains_key("rawMode"));
    assert!(props.contains_key("noEnter"));

    let raw_mode = &props["rawMode"];
    assert_eq!(raw_mode["type"], "boolean");

    let no_enter = &props["noEnter"];
    assert_eq!(no_enter["type"], "boolean");
}

#[test]
fn test_split_pane_requires_pane_id() {
    let tools = get_tool_definitions();
    let tool = tools.iter().find(|t| t["name"] == "split-pane").unwrap();

    let required = tool["inputSchema"]["required"].as_array().unwrap();
    assert!(required.contains(&json!("paneId")));
}

#[test]
fn test_split_pane_direction_is_optional() {
    let tools = get_tool_definitions();
    let tool = tools.iter().find(|t| t["name"] == "split-pane").unwrap();

    let required = tool["inputSchema"]["required"].as_array().unwrap();
    assert!(!required.contains(&json!("direction")));
}

#[test]
fn test_get_command_result_requires_command_id() {
    let tools = get_tool_definitions();
    let tool = tools
        .iter()
        .find(|t| t["name"] == "get-command-result")
        .unwrap();

    let required = tool["inputSchema"]["required"].as_array().unwrap();
    assert!(required.contains(&json!("commandId")));
}

#[test]
fn test_tool_names_are_unique() {
    let tools = get_tool_definitions();
    let names: Vec<_> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    let mut sorted_names = names.clone();
    sorted_names.sort();
    sorted_names.dedup();
    assert_eq!(names.len(), sorted_names.len());
}

#[test]
fn test_all_tools_have_descriptions() {
    let tools = get_tool_definitions();

    for tool in &tools {
        assert!(tool["description"].is_string());
        assert!(!tool["description"].as_str().unwrap().is_empty());
    }
}

/// Helper to get tool definitions - mirrors the protocol handler
fn get_tool_definitions() -> Vec<serde_json::Value> {
    vec![
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
    ]
}
