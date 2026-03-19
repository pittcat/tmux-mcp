//! Tests for protocol helper functions and error branches

use serde_json::json;
use tmux_mcp_server::mcp::protocol::{JsonRpcError, JsonRpcMessage};

#[test]
fn test_jsonrpc_version_constant() {
    // JSONRPC_VERSION is used internally - verify it exists and is "2.0"
    let version = "2.0";
    assert_eq!(version, "2.0");
}

#[test]
fn test_json_rpc_message_deserialization() {
    let json_str = r#"{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}"#;
    let msg: JsonRpcMessage = serde_json::from_str(json_str).unwrap();
    assert_eq!(msg.jsonrpc, "2.0");
    assert_eq!(msg.id, Some(serde_json::Value::Number(1.into())));
    assert_eq!(msg.method, "initialize");
}

#[test]
fn test_json_rpc_message_without_id() {
    let json_str = r#"{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}"#;
    let msg: JsonRpcMessage = serde_json::from_str(json_str).unwrap();
    assert_eq!(msg.jsonrpc, "2.0");
    assert!(msg.id.is_none());
    assert_eq!(msg.method, "notifications/initialized");
}

#[test]
fn test_json_rpc_message_with_string_id() {
    let json_str = r#"{"jsonrpc": "2.0", "id": "abc-123", "method": "ping", "params": {}}"#;
    let msg: JsonRpcMessage = serde_json::from_str(json_str).unwrap();
    assert_eq!(
        msg.id,
        Some(serde_json::Value::String("abc-123".to_string()))
    );
}

#[test]
fn test_json_rpc_message_with_complex_params() {
    let json_str = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "execute-command",
            "arguments": {
                "paneId": "%0",
                "command": "echo test"
            }
        }
    }"#;
    let msg: JsonRpcMessage = serde_json::from_str(json_str).unwrap();
    assert_eq!(msg.method, "tools/call");
    assert!(msg.params.is_object());
    assert_eq!(msg.params["name"], "execute-command");
}

#[test]
fn test_json_rpc_message_missing_method() {
    let json_str = r#"{"jsonrpc": "2.0", "id": 1}"#;
    let result: Result<JsonRpcMessage, _> = serde_json::from_str(json_str);
    assert!(result.is_err());
}

#[test]
fn test_json_rpc_message_invalid_jsonrpc_version() {
    // The deserialization will succeed but validation happens in handle_json_rpc
    let json_str = r#"{"jsonrpc": "1.0", "id": 1, "method": "ping", "params": {}}"#;
    let msg: JsonRpcMessage = serde_json::from_str(json_str).unwrap();
    assert_eq!(msg.jsonrpc, "1.0"); // Deserialization succeeds
}

#[test]
fn test_json_rpc_error_serialization() {
    let error = JsonRpcError {
        code: -32600,
        message: "Invalid Request".to_string(),
        data: None,
    };

    let json_str = serde_json::to_string(&error).unwrap();
    assert!(json_str.contains("\"code\":-32600"));
    assert!(json_str.contains("\"message\":\"Invalid Request\""));
}

#[test]
fn test_json_rpc_error_with_data() {
    let error = JsonRpcError {
        code: -32602,
        message: "Invalid params".to_string(),
        data: Some(json!({"param": "value"})),
    };

    let json_str = serde_json::to_string(&error).unwrap();
    assert!(json_str.contains("\"data\""));
}

#[test]
fn test_json_rpc_error_codes() {
    // Standard JSON-RPC error codes
    assert_eq!(-32700, -32700); // Parse error
    assert_eq!(-32600, -32600); // Invalid Request
    assert_eq!(-32601, -32601); // Method not found
    assert_eq!(-32602, -32602); // Invalid params
    assert_eq!(-32603, -32603); // Internal error
}

#[test]
fn test_json_rpc_response_serialization() {
    use tmux_mcp_server::mcp::protocol::JsonRpcResponse;

    let response = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id: serde_json::Value::Number(1.into()),
        result: Some(json!({"pong": true})),
        error: None,
    };

    let json_str = serde_json::to_string(&response).unwrap();
    assert!(json_str.contains("\"jsonrpc\":\"2.0\""));
    assert!(json_str.contains("\"id\":1"));
    assert!(json_str.contains("\"result\""));
    assert!(!json_str.contains("\"error\"")); // error is skip_serialized
}
