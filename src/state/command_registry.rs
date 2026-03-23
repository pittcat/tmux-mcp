use std::collections::HashMap;
use std::sync::RwLock;

use chrono::{DateTime, Utc};

use crate::tmux::models::{CommandExecution, CommandStatus};

pub struct CommandRegistry {
    commands: RwLock<HashMap<String, CommandEntry>>,
    max_commands: usize,
    ttl_seconds: u64,
}

struct CommandEntry {
    execution: CommandExecution,
    created_at: DateTime<Utc>,
}

impl CommandRegistry {
    pub fn new(max_commands: usize, ttl_seconds: u64) -> Self {
        Self {
            commands: RwLock::new(HashMap::new()),
            max_commands,
            ttl_seconds,
        }
    }

    pub fn insert(&self, command_id: String, execution: CommandExecution) {
        let mut commands = self.commands.write().unwrap();

        // Enforce capacity limit by removing oldest entries if needed
        if commands.len() >= self.max_commands {
            let to_remove: Vec<String> = commands
                .iter()
                .filter(|(_, entry)| entry.execution.status != CommandStatus::Pending)
                .min_by_key(|(_, entry)| entry.created_at)
                .map(|(id, _)| vec![id.clone()])
                .unwrap_or_default();

            for id in to_remove {
                commands.remove(&id);
            }
        }

        commands.insert(
            command_id,
            CommandEntry {
                execution,
                created_at: Utc::now(),
            },
        );
    }

    pub fn get(&self, command_id: &str) -> Option<CommandExecution> {
        let commands = self.commands.read().unwrap();
        commands
            .get(command_id)
            .map(|entry| entry.execution.clone())
    }

    pub fn list_active(&self) -> Vec<CommandExecution> {
        let commands = self.commands.read().unwrap();
        commands
            .values()
            .map(|entry| entry.execution.clone())
            .collect()
    }

    pub fn cleanup_expired(&self) {
        let mut commands = self.commands.write().unwrap();
        let now = Utc::now();

        commands.retain(|_, entry| {
            let age = now.signed_duration_since(entry.created_at);
            // Keep pending commands and commands younger than TTL
            entry.execution.status == CommandStatus::Pending
                || age.num_seconds() < self.ttl_seconds as i64
        });
    }

    pub fn len(&self) -> usize {
        let commands = self.commands.read().unwrap();
        commands.len()
    }

    pub fn is_empty(&self) -> bool {
        let commands = self.commands.read().unwrap();
        commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_execution(id: &str) -> CommandExecution {
        CommandExecution {
            id: id.to_string(),
            pane_id: "%0".to_string(),
            command: "echo test".to_string(),
            status: CommandStatus::Pending,
            start_time: Utc::now(),
            result: None,
            exit_code: None,
            raw_mode: false,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let registry = CommandRegistry::new(100, 600);
        let exec = create_test_execution("cmd-1");

        registry.insert("cmd-1".to_string(), exec.clone());
        let retrieved = registry.get("cmd-1").unwrap();

        assert_eq!(retrieved.id, "cmd-1");
    }

    #[test]
    fn test_capacity_limit() {
        let registry = CommandRegistry::new(3, 600);

        for i in 0..5 {
            let mut exec = create_test_execution(&format!("cmd-{}", i));
            exec.status = CommandStatus::Completed;
            registry.insert(format!("cmd-{}", i), exec);
        }

        // Should have removed oldest completed commands
        assert!(registry.len() <= 3);
    }

    #[test]
    fn test_cleanup_expired() {
        let registry = CommandRegistry::new(100, 1);

        let mut exec = create_test_execution("cmd-1");
        exec.status = CommandStatus::Completed;
        registry.insert("cmd-1".to_string(), exec);

        // Wait for TTL to expire
        std::thread::sleep(std::time::Duration::from_secs(2));

        registry.cleanup_expired();

        assert!(registry.get("cmd-1").is_none());
    }
}
