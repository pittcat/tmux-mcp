//! Tests for parser boundary conditions

use tmux_mcp_server::error::TmuxMcpError;
use tmux_mcp_server::tmux::parser;

#[test]
fn test_parse_sessions_empty_output() {
    let output = "";
    let sessions = parser::parse_sessions(output);
    assert!(sessions.is_empty());
}

#[test]
fn test_parse_sessions_whitespace_only() {
    let output = "   \n\t\n  ";
    let sessions = parser::parse_sessions(output);
    assert!(sessions.is_empty());
}

#[test]
fn test_parse_sessions_malformed_line() {
    // Line with insufficient parts
    let output = "$0:session1:1";
    let sessions = parser::parse_sessions(output);
    assert!(sessions.is_empty());
}

#[test]
fn test_parse_sessions_valid() {
    let output = "$0:session1:1:3\n$1:session2:0:1";
    let sessions = parser::parse_sessions(output);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].id, "$0");
    assert_eq!(sessions[0].name, "session1");
    assert!(sessions[0].attached);
    assert_eq!(sessions[0].windows, 3);
}

#[test]
fn test_parse_windows_empty_output() {
    let output = "";
    let windows = parser::parse_windows(output, "$0");
    assert!(windows.is_empty());
}

#[test]
fn test_parse_windows_malformed() {
    let output = "@0:window1"; // Missing active flag
    let windows = parser::parse_windows(output, "$0");
    assert!(windows.is_empty());
}

#[test]
fn test_parse_panes_empty_output() {
    let output = "";
    let panes = parser::parse_panes(output, "@0");
    assert!(panes.is_empty());
}

#[test]
fn test_parse_pane_valid_control_mode() {
    let output = "%begin 1 2 1\n%4:title4:1:@2\n%end 1 2 1";
    let pane = parser::parse_pane(output).unwrap();
    assert_eq!(pane.id, "%4");
    assert_eq!(pane.title, "title4");
    assert!(pane.active);
    assert_eq!(pane.window_id, "@2");
}

#[test]
fn test_parse_pane_empty_output() {
    let output = "";
    let result = parser::parse_pane(output);
    assert!(result.is_err());
}

#[test]
fn test_parse_pane_malformed_output() {
    let output = "%begin 1 2 1\n%end 1 2 1"; // No pane line
    let result = parser::parse_pane(output);
    assert!(result.is_err());
}

#[test]
fn test_parse_command_output_valid() {
    let content = "TMUX_MCP_START\necho hello\nhello\nTMUX_MCP_DONE_0\n";
    let result = parser::parse_command_output(content, "TMUX_MCP_START", "TMUX_MCP_DONE_");
    assert!(result.is_ok());
    let (output, exit_code) = result.unwrap();
    assert_eq!(exit_code, 0);
    assert!(output.contains("hello"));
}

#[test]
fn test_parse_command_output_missing_start_marker() {
    let content = "some text\nTMUX_MCP_DONE_0\n";
    let result = parser::parse_command_output(content, "TMUX_MCP_START", "TMUX_MCP_DONE_");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, TmuxMcpError::CommandExecutionError(_)));
}

#[test]
fn test_parse_command_output_missing_end_marker() {
    let content = "TMUX_MCP_START\noutput\n";
    let result = parser::parse_command_output(content, "TMUX_MCP_START", "TMUX_MCP_DONE_");
    assert!(result.is_err());
}

#[test]
fn test_parse_command_output_invalid_marker_order() {
    // End marker comes before start marker
    let content = "TMUX_MCP_DONE_0\nTMUX_MCP_START\noutput\n";
    let result = parser::parse_command_output(content, "TMUX_MCP_START", "TMUX_MCP_DONE_");
    assert!(result.is_err());
}

#[test]
fn test_parse_command_output_invalid_exit_code() {
    let content = "TMUX_MCP_START\noutput\nTMUX_MCP_DONE_not_a_number\n";
    let result = parser::parse_command_output(content, "TMUX_MCP_START", "TMUX_MCP_DONE_");
    assert!(result.is_err());
}

#[test]
fn test_parse_command_output_non_zero_exit_code() {
    let content = "TMUX_MCP_START\necho error\nerror\nTMUX_MCP_DONE_1\n";
    let result = parser::parse_command_output(content, "TMUX_MCP_START", "TMUX_MCP_DONE_");
    assert!(result.is_ok());
    let (_, exit_code) = result.unwrap();
    assert_eq!(exit_code, 1);
}
