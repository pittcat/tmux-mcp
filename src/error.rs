use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TmuxMcpError {
    #[error("tmux command failed: {0}")]
    TmuxError(String),

    #[error("tmux not found or not running")]
    TmuxNotAvailable,

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("window not found: {0}")]
    WindowNotFound(String),

    #[error("pane not found: {0}")]
    PaneNotFound(String),

    #[allow(dead_code)]
    #[error("command not found: {0}")]
    CommandNotFound(String),

    #[error("invalid parameter: {0}")]
    InvalidParameter(String),

    #[allow(dead_code)]
    #[error("command execution failed: {0}")]
    CommandExecutionError(String),

    #[allow(dead_code)]
    #[error("internal error: {0}")]
    InternalError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),
}

impl IntoResponse for TmuxMcpError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            TmuxMcpError::TmuxNotAvailable => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            TmuxMcpError::SessionNotFound(_)
            | TmuxMcpError::WindowNotFound(_)
            | TmuxMcpError::PaneNotFound(_)
            | TmuxMcpError::CommandNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            TmuxMcpError::InvalidParameter(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(json!({
            "error": error_message,
            "code": status.as_u16()
        }));

        (status, body).into_response()
    }
}

impl From<serde_json::Error> for TmuxMcpError {
    fn from(e: serde_json::Error) -> Self {
        TmuxMcpError::SerializationError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, TmuxMcpError>;
