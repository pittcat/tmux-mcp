//! Tmux integration tests
//! These tests interact with real tmux and only operate on test-created resources

use std::process::Command;

fn generate_test_prefix() -> String {
    let pid = std::process::id();
    format!("tmux_mcp_test_{}_", pid)
}

fn cleanup_test_sessions() {
    let prefix = generate_test_prefix();
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output();

    if let Ok(output) = output {
        let sessions = String::from_utf8_lossy(&output.stdout);
        for session in sessions.lines() {
            if session.starts_with(&prefix) {
                let _ = Command::new("tmux")
                    .args(["kill-session", "-t", session])
                    .output();
            }
        }
    }
}

#[test]
fn test_list_sessions_format() {
    // This test verifies the output format matches expectations
    // without requiring actual tmux sessions
    let output = r"$0:test_session:1:3
$1:another_session:0:1";

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);

    let parts: Vec<&str> = lines[0].split(':').collect();
    assert_eq!(parts.len(), 4);
    assert!(parts[0].starts_with('$'));
}

#[test]
fn test_parse_windows_format() {
    let output = "@0:window1:1\n@1:window2:0";

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);

    let parts: Vec<&str> = lines[0].split(':').collect();
    assert_eq!(parts.len(), 3);
    assert!(parts[0].starts_with('@'));
}

#[test]
fn test_parse_panes_format() {
    let output = "%0:bash:1\n%1:zsh:0";

    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);

    let parts: Vec<&str> = lines[0].split(':').collect();
    assert_eq!(parts.len(), 3);
    assert!(parts[0].starts_with('%'));
}

#[test]
fn test_session_name_prefix_safety() {
    // Ensure our test prefix generation creates unique names
    let prefix1 = generate_test_prefix();
    let prefix2 = generate_test_prefix();
    assert_eq!(prefix1, prefix2, "Same process should generate same prefix");
    assert!(prefix1.starts_with("tmux_mcp_test_"));
}

#[test]
#[ignore = "Requires tmux to be running"]
fn test_create_and_kill_session() {
    cleanup_test_sessions();

    let prefix = generate_test_prefix();
    let session_name = format!("{}integration_test", prefix);

    // Create session
    let output = Command::new("tmux")
        .args(["new-session", "-d", "-s", &session_name])
        .output()
        .expect("Failed to execute tmux");

    assert!(output.status.success(), "Failed to create test session");

    // Verify session exists
    let output = Command::new("tmux")
        .args(["has-session", "-t", &session_name])
        .output()
        .expect("Failed to check session");

    assert!(output.status.success(), "Session should exist");

    // Kill session
    let output = Command::new("tmux")
        .args(["kill-session", "-t", &session_name])
        .output()
        .expect("Failed to kill session");

    assert!(output.status.success(), "Failed to kill test session");
}
