use std::sync::Arc;

use axum::Router;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;
use tmux_mcp_server::state::command_registry::CommandRegistry;
use tmux_mcp_server::transport;
use tokio::net::TcpListener;

async fn spawn_protocol_server() -> (String, tokio::task::JoinHandle<()>) {
    let registry = Arc::new(CommandRegistry::new(100, 600));
    let app: Router = transport::create_transport_router(registry);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), handle)
}

#[tokio::test]
async fn initialize_over_post_returns_streamable_http_protocol_version() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "0.1.0"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("MCP-Protocol-Version")
            .unwrap()
            .to_str()
            .unwrap(),
        "2025-03-26"
    );
    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap(),
        "application/json"
    );

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["protocolVersion"], "2025-03-26");
    assert!(body["result"]["capabilities"].is_object());
    assert_eq!(body["result"]["serverInfo"]["name"], "tmux-mcp-server");

    handle.abort();
}

#[tokio::test]
async fn initialized_notification_returns_accepted() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);
    assert_eq!(response.content_length(), Some(0));

    handle.abort();
}

#[tokio::test]
async fn get_mcp_returns_sse_stream() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/mcp", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.starts_with("text/event-stream"),
        "Expected text/event-stream, got {}",
        content_type
    );
    assert_eq!(
        response
            .headers()
            .get("MCP-Protocol-Version")
            .unwrap()
            .to_str()
            .unwrap(),
        "2025-03-26"
    );

    handle.abort();
}

#[tokio::test]
async fn invalid_protocol_version_header_is_rejected_for_non_initialize_requests() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "not-a-version")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32600);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid MCP-Protocol-Version"));

    handle.abort();
}

#[tokio::test]
async fn invalid_json_body_returns_parse_error() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .body("not valid json{{{")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32700);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid JSON"));

    handle.abort();
}

#[tokio::test]
async fn invalid_jsonrpc_version_returns_error() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "jsonrpc": "1.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-05"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["error"]["code"], -32600);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid JSON-RPC version"));

    handle.abort();
}

#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "unknown/method",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["error"].is_object());
    assert_eq!(body["error"]["code"], -32601);
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Method not found"));

    handle.abort();
}

#[tokio::test]
async fn ping_returns_pong() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "ping",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["pong"], true);

    handle.abort();
}

#[tokio::test]
async fn tools_list_via_json_rpc_returns_tool_list() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/list",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["result"]["tools"].is_array());
    let tools = body["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty());

    // Verify tool structure
    let first_tool = &tools[0];
    assert!(first_tool["name"].is_string());
    assert!(first_tool["description"].is_string());
    assert!(first_tool["inputSchema"].is_object());

    handle.abort();
}

#[tokio::test]
async fn resources_list_via_json_rpc_returns_resource_list() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "resources/list",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["result"]["resources"].is_array());

    handle.abort();
}

#[tokio::test]
async fn missing_protocol_version_header_for_initialize_is_accepted() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    // Initialize does not require the protocol version header
    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-05"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["result"]["protocolVersion"].is_string());

    handle.abort();
}

#[tokio::test]
async fn wrong_protocol_version_for_initialize_returns_error() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-01-01"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["error"].is_object());
    assert_eq!(body["error"]["code"], -32602);

    handle.abort();
}

#[tokio::test]
async fn notification_without_id_returns_accepted() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    // Send a notification (no id field)
    let response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::ACCEPTED);

    handle.abort();
}

#[tokio::test]
async fn sse_stream_contains_protocol_version_header() {
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/mcp", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response
            .headers()
            .get("MCP-Protocol-Version")
            .unwrap()
            .to_str()
            .unwrap(),
        "2025-03-26"
    );

    handle.abort();
}

#[tokio::test]
async fn post_then_get_reconnection_path() {
    // Test that POST /mcp followed by GET /mcp works (simulates reconnection)
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    // First POST a request
    let post_response = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .header("MCP-Protocol-Version", "2025-03-26")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(post_response.status(), reqwest::StatusCode::OK);

    // Then GET SSE stream (reconnection path)
    let get_response = client
        .get(format!("{}/mcp", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(get_response.status(), reqwest::StatusCode::OK);
    let content_type = get_response
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.starts_with("text/event-stream"),
        "Expected text/event-stream, got {}",
        content_type
    );

    handle.abort();
}

#[tokio::test]
async fn multiple_initialize_requests_stateless() {
    // Test that multiple initialize requests work (server is stateless)
    let (base_url, handle) = spawn_protocol_server().await;
    let client = reqwest::Client::new();

    for i in 1..=3 {
        let response = client
            .post(format!("{}/mcp", base_url))
            .header(CONTENT_TYPE, "application/json")
            .json(&json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "test-client",
                        "version": "0.1.0"
                    }
                }
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["result"]["protocolVersion"], "2025-03-26");
    }

    handle.abort();
}
