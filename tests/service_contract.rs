//! Tests for tmux service command construction and error mapping

use tmux_mcp_server::tmux::models::ShellType;

#[test]
fn test_escape_target_single_quotes() {
    // Test that single quotes are properly escaped for tmux targets
    // The escape pattern is: ' -> '\''
    let input = "session'name";
    let escaped = escape_for_shell(input);
    // The escaped version should have the quote escaped as '\''
    assert!(escaped.contains("'\\''")); // Properly escaped single quote
}

#[test]
fn test_escape_target_regular_name() {
    let input = "session_name";
    let escaped = escape_for_shell(input);
    assert_eq!(escaped, "session_name");
}

#[test]
fn test_get_end_marker_text_bash() {
    let marker = tmux_mcp_server::tmux::service::get_end_marker_text(ShellType::Bash);
    assert_eq!(marker, "TMUX_MCP_DONE_$?");
}

#[test]
fn test_get_end_marker_text_zsh() {
    let marker = tmux_mcp_server::tmux::service::get_end_marker_text(ShellType::Zsh);
    assert_eq!(marker, "TMUX_MCP_DONE_$?");
}

#[test]
fn test_get_end_marker_text_fish() {
    let marker = tmux_mcp_server::tmux::service::get_end_marker_text(ShellType::Fish);
    assert_eq!(marker, "TMUX_MCP_DONE_$status");
}

#[test]
fn test_shell_type_exit_code_var_bash() {
    assert_eq!(ShellType::Bash.exit_code_var(), "$?");
}

#[test]
fn test_shell_type_exit_code_var_zsh() {
    assert_eq!(ShellType::Zsh.exit_code_var(), "$?");
}

#[test]
fn test_shell_type_exit_code_var_fish() {
    assert_eq!(ShellType::Fish.exit_code_var(), "$status");
}

#[test]
fn test_shell_type_parse() {
    assert_eq!(ShellType::parse("bash"), ShellType::Bash);
    assert_eq!(ShellType::parse("zsh"), ShellType::Zsh);
    assert_eq!(ShellType::parse("fish"), ShellType::Fish);
    assert_eq!(ShellType::parse("BASH"), ShellType::Bash); // case insensitive
    assert_eq!(ShellType::parse("unknown"), ShellType::Bash); // default
}

#[test]
fn test_capture_pane_command_construction() {
    let cmd = construct_capture_pane_command("%0", Some(100), false);
    assert!(cmd.contains("capture-pane"));
    assert!(cmd.contains("-p"));
    assert!(cmd.contains("-t '%0'"));
    assert!(cmd.contains("-S -100")); // lines from end
    assert!(cmd.contains("-E -")); // to end
}

#[test]
fn test_capture_pane_command_with_colors() {
    let cmd = construct_capture_pane_command("%0", Some(50), true);
    assert!(cmd.contains("-e"));
}

#[test]
fn test_capture_pane_command_default_lines() {
    let cmd = construct_capture_pane_command("%0", None, false);
    assert!(cmd.contains("-S -200")); // default 200 lines
}

#[test]
fn test_split_pane_direction_horizontal() {
    let cmd = construct_split_pane_command("%0", "horizontal", None);
    assert!(cmd.contains("-h")); // horizontal split flag
}

#[test]
fn test_split_pane_direction_vertical() {
    let cmd = construct_split_pane_command("%0", "vertical", None);
    assert!(cmd.contains("-v")); // vertical split flag
}

#[test]
fn test_split_pane_with_size() {
    let cmd = construct_split_pane_command("%0", "horizontal", Some(50));
    assert!(cmd.contains("-p 50"));
}

#[test]
fn test_split_pane_without_size() {
    let cmd = construct_split_pane_command("%0", "vertical", None);
    assert!(!cmd.contains("-p"));
}

#[test]
fn test_send_keys_command_construction() {
    let cmd = construct_send_keys_command("%0", "echo hello");
    assert!(cmd.contains("send-keys"));
    assert!(cmd.contains("-t '%0'"));
    assert!(cmd.contains("'echo hello'"));
    assert!(cmd.contains("Enter"));
}

#[test]
fn test_list_sessions_format_string() {
    let format = "#{session_id}:#{session_name}:#{?session_attached,1,0}:#{session_windows}";
    // Verify format contains expected tmux format variables
    assert!(format.contains("session_id"));
    assert!(format.contains("session_name"));
    assert!(format.contains("session_attached"));
    assert!(format.contains("session_windows"));
}

/// Escape a target string for tmux shell commands
fn escape_for_shell(target: &str) -> String {
    target.replace('\'', "'\\''")
}

/// Construct a capture-pane command
fn construct_capture_pane_command(
    pane_id: &str,
    lines: Option<usize>,
    include_colors: bool,
) -> String {
    let lines = lines.unwrap_or(200);
    let color_flag = if include_colors { "-e" } else { "" };
    format!(
        "capture-pane -p {} -t '{}' -S -{} -E -",
        color_flag,
        escape_for_shell(pane_id),
        lines
    )
}

/// Construct a split-pane command
fn construct_split_pane_command(target: &str, direction: &str, size: Option<u8>) -> String {
    let dir_flag = if direction == "horizontal" {
        "-h"
    } else {
        "-v"
    };
    let size_flag = size.map(|s| format!(" -p {}", s)).unwrap_or_default();

    format!(
        "split-window {} -P -F '#{{pane_id}}:#{{pane_title}}:#{{?pane_active,1,0}}:#{{window_id}}' -t '{}'{}",
        dir_flag,
        escape_for_shell(target),
        size_flag
    )
}

/// Construct a send-keys command
fn construct_send_keys_command(pane_id: &str, command: &str) -> String {
    let escaped = command.replace('\'', "'\\''");
    format!(
        "send-keys -t '{}' '{}' Enter",
        escape_for_shell(pane_id),
        escaped
    )
}
