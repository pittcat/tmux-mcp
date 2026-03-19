//! Tests for TmuxMcpError to HTTP response mapping

use axum::http::StatusCode;
use axum::response::IntoResponse;
use tmux_mcp_server::error::TmuxMcpError;

fn get_error_status(error: TmuxMcpError) -> StatusCode {
    let response: axum::response::Response = error.into_response();
    response.status()
}

#[test]
fn test_tmux_not_available_returns_service_unavailable() {
    let error = TmuxMcpError::TmuxNotAvailable;
    assert_eq!(get_error_status(error), StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn test_session_not_found_returns_not_found() {
    let error = TmuxMcpError::SessionNotFound("$0".to_string());
    assert_eq!(get_error_status(error), StatusCode::NOT_FOUND);
}

#[test]
fn test_window_not_found_returns_not_found() {
    let error = TmuxMcpError::WindowNotFound("@0".to_string());
    assert_eq!(get_error_status(error), StatusCode::NOT_FOUND);
}

#[test]
fn test_pane_not_found_returns_not_found() {
    let error = TmuxMcpError::PaneNotFound("%0".to_string());
    assert_eq!(get_error_status(error), StatusCode::NOT_FOUND);
}

#[test]
fn test_command_not_found_returns_not_found() {
    let error = TmuxMcpError::CommandNotFound("cmd-123".to_string());
    assert_eq!(get_error_status(error), StatusCode::NOT_FOUND);
}

#[test]
fn test_invalid_parameter_returns_bad_request() {
    let error = TmuxMcpError::InvalidParameter("paneId is required".to_string());
    assert_eq!(get_error_status(error), StatusCode::BAD_REQUEST);
}

#[test]
fn test_tmux_timeout_returns_bad_request() {
    let error = TmuxMcpError::TmuxTimeout(10);
    assert_eq!(get_error_status(error), StatusCode::BAD_REQUEST);
}

#[test]
fn test_tmux_error_returns_internal_server_error() {
    let error = TmuxMcpError::TmuxError("something went wrong".to_string());
    assert_eq!(get_error_status(error), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_command_execution_error_returns_internal_server_error() {
    let error = TmuxMcpError::CommandExecutionError("execution failed".to_string());
    assert_eq!(get_error_status(error), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_internal_error_returns_internal_server_error() {
    let error = TmuxMcpError::InternalError("unexpected error".to_string());
    assert_eq!(get_error_status(error), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_serialization_error_returns_internal_server_error() {
    let error = TmuxMcpError::SerializationError("json error".to_string());
    assert_eq!(get_error_status(error), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_error_response_includes_status_code() {
    let error = TmuxMcpError::PaneNotFound("%1".to_string());
    let response: axum::response::Response = error.into_response();

    // Verify the status code is NOT_FOUND
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
