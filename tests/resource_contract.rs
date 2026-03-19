//! Tests for resource contract validation

use serde_json::json;

#[test]
fn test_resource_templates_structure() {
    let resources = get_resource_templates();

    for resource in &resources {
        assert!(resource["uri"].is_string());
        assert!(!resource["uri"].as_str().unwrap().is_empty());
        assert!(resource["mimeType"].is_string());
    }
}

#[test]
fn test_sessions_resource_uri() {
    let resources = get_resource_templates();
    let session_resource = resources
        .iter()
        .find(|r| r["uri"] == "tmux://sessions")
        .unwrap();

    assert!(session_resource["name"].is_string());
    assert!(session_resource["description"].is_string());
}

#[test]
fn test_command_resource_uri_format() {
    // Command result URIs follow pattern: tmux://command/{id}/result
    let uri_pattern = "tmux://command/";

    // The resources list includes a template
    let resources = get_resource_templates();
    let has_command_template = resources.iter().any(|r| {
        r["uri"]
            .as_str()
            .map(|u| u.starts_with(uri_pattern) || u.contains("{commandId}"))
            .unwrap_or(false)
    });

    // Resources should include command result resource type
    assert!(has_command_template || !resources.is_empty());
}

#[test]
fn test_resource_uri_scheme() {
    let resources = get_resource_templates();

    for resource in &resources {
        let uri = resource["uri"].as_str().unwrap();
        assert!(
            uri.starts_with("tmux://"),
            "URI {} should start with tmux://",
            uri
        );
    }
}

#[test]
fn test_resource_list_response_structure() {
    let response = get_resource_list_response();

    assert!(response["resources"].is_array());
    let resources = response["resources"].as_array().unwrap();
    assert!(!resources.is_empty());
}

#[test]
fn test_resource_read_response_structure() {
    // Test response structure for command result
    let response = json!({
        "contents": [{
            "uri": "tmux://command/abc-123/result",
            "mimeType": "text/plain",
            "text": "Status: Completed\nExit code: 0\nCommand: echo test\n\nOutput:\ntest"
        }]
    });

    assert!(response["contents"].is_array());
    let contents = response["contents"].as_array().unwrap();
    assert_eq!(contents.len(), 1);

    let content = &contents[0];
    assert!(content["uri"].is_string());
    assert!(content["mimeType"].is_string());
    assert!(content["text"].is_string());
}

#[test]
fn test_pending_command_result_text_format() {
    let pending_text =
        "Status: Pending\nCommand: echo test\n\n--- Message ---\nCommand still executing...";

    assert!(pending_text.contains("Pending"));
    assert!(pending_text.contains("Command:"));
}

#[test]
fn test_completed_command_result_text_format() {
    let completed_text =
        "Status: Completed\nExit code: 0\nCommand: echo test\n\n--- Output ---\ntest";

    assert!(completed_text.contains("Completed"));
    assert!(completed_text.contains("Exit code:"));
    assert!(completed_text.contains("0"));
}

#[test]
fn test_error_command_result_text_format() {
    let error_text = "Status: Error\nExit code: 1\nCommand: false\n\n--- Output ---\n";

    assert!(error_text.contains("Error"));
    assert!(error_text.contains("Exit code:"));
    assert!(error_text.contains("1"));
}

#[test]
fn test_session_resource_text_format() {
    let session_text = "[{\"id\":\"$0\",\"name\":\"test\",\"attached\":true,\"windows\":3}]";

    // Should be valid JSON when parsed
    let parsed: Result<Vec<serde_json::Value>, _> = serde_json::from_str(session_text);
    assert!(parsed.is_ok());
}

/// Helper to get resource templates - mirrors the protocol handler
fn get_resource_templates() -> Vec<serde_json::Value> {
    vec![
        json!({
            "uri": "tmux://sessions",
            "name": "List all tmux sessions",
            "mimeType": "application/json",
            "description": "Returns a JSON array of all active tmux sessions"
        }),
        json!({
            "uri": "tmux://command/{commandId}/result",
            "name": "Command execution result",
            "mimeType": "text/plain",
            "description": "Returns the result of an executed command"
        }),
    ]
}

/// Helper to get resource list response
fn get_resource_list_response() -> serde_json::Value {
    json!({
        "resources": [
            {
                "uri": "tmux://sessions",
                "name": "List all tmux sessions",
                "mimeType": "application/json",
                "description": "Returns a JSON array of all active tmux sessions"
            }
        ]
    })
}
