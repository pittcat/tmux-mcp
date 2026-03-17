use std::sync::Arc;

use axum::Router;
use reqwest::header::CONTENT_TYPE;
use serde_json::json;
use tmux_mcp_server::mcp::protocol;
use tmux_mcp_server::state::command_registry::CommandRegistry;
use tokio::net::TcpListener;

async fn spawn_protocol_server() -> (String, tokio::task::JoinHandle<()>) {
    let registry = Arc::new(CommandRegistry::new(100, 600));
    let app: Router = protocol::create_protocol_router(registry);

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
            .get(CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap(),
        "application/json"
    );

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["result"]["protocolVersion"], "2025-03-26");

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
    assert!(response
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));

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

    handle.abort();
}
