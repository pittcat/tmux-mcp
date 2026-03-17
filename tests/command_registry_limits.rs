//! Command registry limits tests
//! Verifies TTL cleanup and capacity limits

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tmux_mcp_server::state::command_registry::CommandRegistry;
use tmux_mcp_server::tmux::models::{CommandExecution, CommandStatus};

fn create_test_execution(id: &str) -> CommandExecution {
    CommandExecution {
        id: id.to_string(),
        pane_id: "%0".to_string(),
        command: "echo test".to_string(),
        status: CommandStatus::Completed,
        start_time: Utc::now(),
        result: Some("test output".to_string()),
        exit_code: Some(0),
        raw_mode: false,
    }
}

#[test]
fn test_capacity_limit_enforcement() {
    let registry = CommandRegistry::new(10, 600);

    // Insert 15 commands (exceeding capacity)
    for i in 0..15 {
        let exec = create_test_execution(&format!("cmd-{}", i));
        registry.insert(format!("cmd-{}", i), exec);
    }

    // Registry should have removed some oldest completed commands
    // but kept at most 10
    assert!(
        registry.len() <= 10,
        "Registry should enforce capacity limit"
    );
}

#[test]
fn test_ttl_cleanup() {
    let registry = CommandRegistry::new(100, 1); // 1 second TTL

    let exec = create_test_execution("expiring-cmd");
    registry.insert("expiring-cmd".to_string(), exec);

    assert_eq!(registry.len(), 1);

    // Wait for TTL to expire
    std::thread::sleep(Duration::from_secs(2));

    // Run cleanup
    registry.cleanup_expired();

    // Command should be removed
    assert!(
        registry.get("expiring-cmd").is_none(),
        "Expired command should be removed"
    );
}

#[test]
fn test_pending_commands_not_expired() {
    let registry = CommandRegistry::new(100, 1); // 1 second TTL

    let mut exec = create_test_execution("pending-cmd");
    exec.status = CommandStatus::Pending;
    registry.insert("pending-cmd".to_string(), exec);

    // Wait for TTL to expire
    std::thread::sleep(Duration::from_secs(2));

    // Run cleanup
    registry.cleanup_expired();

    // Pending command should still exist
    assert!(
        registry.get("pending-cmd").is_some(),
        "Pending commands should not be expired"
    );
}

#[test]
fn test_registry_thread_safety() {
    use std::thread;

    let registry = Arc::new(CommandRegistry::new(1000, 600));
    let mut handles = vec![];

    // Spawn multiple threads inserting commands
    for i in 0..10 {
        let reg = registry.clone();
        let handle = thread::spawn(move || {
            for j in 0..10 {
                let exec = create_test_execution(&format!("thread-{}-cmd-{}", i, j));
                reg.insert(format!("thread-{}-cmd-{}", i, j), exec);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // All commands should be accessible
    assert_eq!(registry.len(), 100);
}
