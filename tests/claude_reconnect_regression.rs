//! Tests for Claude reconnection and auth fallback behavior
//!
//! These tests verify that the server properly handles:
//! - Reconnection after server restart
//! - Auth discovery requests returning proper JSON responses instead of 404

use axum::Router;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;
use std::sync::Arc;
use tmux_mcp_server::state::command_registry::CommandRegistry;
use tokio::net::TcpListener;

/// Spawn a test server using the transport router
async fn spawn_server() -> (String, tokio::task::JoinHandle<()>) {
    let registry = Arc::new(CommandRegistry::new(100, 600));
    let app: Router = tmux_mcp_server::transport::create_transport_router(registry);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), handle)
}

#[tokio::test]
async fn test_auth_discovery_returns_json_response() {
    let (base_url, handle) = spawn_server().await;
    let client = reqwest::Client::new();

    // Test /mcp/auth endpoint
    let response = client
        .get(format!("{}/mcp/auth", base_url))
        .send()
        .await
        .unwrap();

    // Should return 200 OK with JSON body (not 404 with empty body)
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Verify protocol version header is present
    let protocol_version = response
        .headers()
        .get("MCP-Protocol-Version")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(protocol_version, "2025-03-26");

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert!(body["error"].is_object());
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("OAuth authentication is not supported"));

    handle.abort();
}

#[tokio::test]
async fn test_auth_discovery_post_returns_json_response() {
    let (base_url, handle) = spawn_server().await;
    let client = reqwest::Client::new();

    // Test POST /mcp/auth endpoint
    let response = client
        .post(format!("{}/mcp/auth", base_url))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "authenticate",
            "params": {}
        }))
        .send()
        .await
        .unwrap();

    // Should return 200 OK with JSON body
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert!(body["error"].is_object());

    handle.abort();
}

#[tokio::test]
async fn test_oauth_endpoint_returns_json_response() {
    let (base_url, handle) = spawn_server().await;
    let client = reqwest::Client::new();

    // Test /oauth endpoint
    let response = client
        .get(format!("{}/oauth", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert!(body["error"].is_object());

    handle.abort();
}

#[tokio::test]
async fn test_authorize_endpoint_returns_json_response() {
    let (base_url, handle) = spawn_server().await;
    let client = reqwest::Client::new();

    // Test /authorize endpoint
    let response = client
        .get(format!("{}/authorize", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert!(body["error"].is_object());

    handle.abort();
}

#[tokio::test]
async fn test_reconnect_after_initialize() {
    let (base_url, handle) = spawn_server().await;
    let client = reqwest::Client::new();

    // Initial initialize request
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
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["protocolVersion"], "2025-03-26");

    // Simulate reconnection by sending another initialize request
    // This should work because the server maintains no persistent state
    let response2 = client
        .post(format!("{}/mcp", base_url))
        .header(CONTENT_TYPE, "application/json")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 2,
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

    assert_eq!(response2.status(), reqwest::StatusCode::OK);
    let body2: serde_json::Value = response2.json().await.unwrap();
    assert_eq!(body2["result"]["protocolVersion"], "2025-03-26");

    handle.abort();
}

#[tokio::test]
async fn test_get_mcp_sse_works_after_post_mcp() {
    let (base_url, handle) = spawn_server().await;
    let client = reqwest::Client::new();

    // First make a POST request
    let response = client
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

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Then get the SSE stream - should work
    let response2 = client
        .get(format!("{}/mcp", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(response2.status(), reqwest::StatusCode::OK);
    let content_type = response2
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
