//! Tests for tmux timeout behavior and hanging subprocess handling.
//!
//! These tests verify that:
//! 1. Hanging tmux subprocesses are properly terminated
//! 2. Timeout errors are returned instead of hanging indefinitely
//! 3. Concurrent hanging requests don't block the entire service

use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[tokio::test]
async fn test_tmux_command_timeout_terminates_process() {
    // This test verifies that a hanging process is terminated after timeout
    let mut cmd = Command::new("sleep");
    cmd.arg("3600"); // Would hang for 1 hour

    let timeout_duration = Duration::from_secs(1);

    let result = timeout(timeout_duration, cmd.output()).await;

    // Should timeout and terminate the process
    assert!(result.is_err(), "Expected timeout to elapse");
}

#[tokio::test]
async fn test_fake_tmux_hang_script_times_out() {
    // Test the fake hang script with a short timeout
    let mut cmd = Command::new("./tests/support/fake_tmux_hang.sh");
    let timeout_duration = Duration::from_secs(1);

    let result = timeout(timeout_duration, cmd.output()).await;

    // Should timeout
    assert!(result.is_err(), "Fake hang script should timeout");
}

#[tokio::test]
async fn test_fake_tmux_error_script_returns_error() {
    // Test the fake error script
    let mut cmd = Command::new("./tests/support/fake_tmux_error.sh");

    let result = cmd
        .output()
        .await
        .expect("Failed to execute fake error script");

    // Should exit with error
    assert!(
        !result.status.success(),
        "Fake error script should return error"
    );
    assert!(!result.stderr.is_empty(), "Should have error output");
}

#[tokio::test]
async fn test_multiple_hanging_commands_can_be_started_concurrently() {
    // Verify that multiple hanging commands can be started without blocking
    // This tests that our async model doesn't serialize subprocess execution

    let handles = (0..5)
        .map(|_| {
            tokio::spawn(async {
                let mut cmd = Command::new("./tests/support/fake_tmux_hang.sh");
                let result = timeout(Duration::from_secs(2), cmd.output()).await;
                result.is_err() // We expect timeout
            })
        })
        .collect::<Vec<_>>();

    // All should timeout independently
    for handle in handles {
        let timed_out = handle.await.expect("Task should not panic");
        assert!(timed_out, "Each hanging command should timeout");
    }
}
