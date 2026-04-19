//! Tests for configuration parsing from environment variables
//!
//! These tests modify global process environment variables and use serial_test
//! to ensure they run serially and avoid interference.

use serial_test::serial;
use std::env;

/// Save and restore environment helper
struct EnvGuard {
    vars: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn new() -> Self {
        EnvGuard { vars: Vec::new() }
    }

    fn set(&mut self, key: &str, value: &str) {
        let prev = env::var(key).ok();
        env::set_var(key, value);
        self.vars.push((key.to_string(), prev));
    }

    fn remove(&mut self, key: &str) {
        let prev = env::var(key).ok();
        env::remove_var(key);
        self.vars.push((key.to_string(), prev));
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.vars.drain(..).rev() {
            match value {
                Some(v) => env::set_var(&key, &v),
                None => env::remove_var(&key),
            }
        }
    }
}

/// Test default configuration values when no env vars are set
#[test]
#[serial]
fn test_default_bind_address() {
    let mut guard = EnvGuard::new();
    guard.remove("TMUX_MCP_BIND_ADDR");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.bind_addr, "127.0.0.1:8090");
}

/// Test custom bind address from env var
#[test]
#[serial]
fn test_custom_bind_address() {
    let mut guard = EnvGuard::new();
    guard.set("TMUX_MCP_BIND_ADDR", "0.0.0.0:3000");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.bind_addr, "0.0.0.0:3000");
}

/// Test default max commands
#[test]
#[serial]
fn test_default_max_commands() {
    let mut guard = EnvGuard::new();
    guard.remove("TMUX_MCP_MAX_COMMANDS");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.max_commands, 1000);
}

/// Test custom max commands from env var
#[test]
#[serial]
fn test_custom_max_commands() {
    let mut guard = EnvGuard::new();
    guard.set("TMUX_MCP_MAX_COMMANDS", "500");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.max_commands, 500);
}

/// Test default command TTL
#[test]
#[serial]
fn test_default_command_ttl() {
    let mut guard = EnvGuard::new();
    guard.remove("TMUX_MCP_COMMAND_TTL");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.command_ttl_seconds, 600);
}

/// Test custom command TTL from env var
#[test]
#[serial]
fn test_custom_command_ttl() {
    let mut guard = EnvGuard::new();
    guard.set("TMUX_MCP_COMMAND_TTL", "300");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.command_ttl_seconds, 300);
}

/// Test invalid max commands falls back to default
#[test]
#[serial]
fn test_invalid_max_commands_falls_back() {
    let mut guard = EnvGuard::new();
    guard.set("TMUX_MCP_MAX_COMMANDS", "not_a_number");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.max_commands, 1000); // Falls back to default
}

/// Test invalid command TTL falls back to default
#[test]
#[serial]
fn test_invalid_command_ttl_falls_back() {
    let mut guard = EnvGuard::new();
    guard.set("TMUX_MCP_COMMAND_TTL", "-100");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.command_ttl_seconds, 600); // Falls back to default
}

/// Test all env vars set together
#[test]
#[serial]
fn test_all_env_vars_together() {
    let mut guard = EnvGuard::new();
    guard.set("TMUX_MCP_BIND_ADDR", "192.168.1.1:9000");
    guard.set("TMUX_MCP_MAX_COMMANDS", "2000");
    guard.set("TMUX_MCP_COMMAND_TTL", "1200");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.bind_addr, "192.168.1.1:9000");
    assert_eq!(config.max_commands, 2000);
    assert_eq!(config.command_ttl_seconds, 1200);
}
