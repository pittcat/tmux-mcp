//! Tests for configuration parsing from environment variables

use std::env;

/// Test default configuration values when no env vars are set
#[test]
fn test_default_bind_address() {
    // Clear any existing env vars
    env::remove_var("TMUX_MCP_BIND_ADDR");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.bind_addr, "127.0.0.1:8090");
}

/// Test custom bind address from env var
#[test]
fn test_custom_bind_address() {
    env::remove_var("TMUX_MCP_BIND_ADDR");
    env::set_var("TMUX_MCP_BIND_ADDR", "0.0.0.0:3000");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.bind_addr, "0.0.0.0:3000");

    env::remove_var("TMUX_MCP_BIND_ADDR");
}

/// Test default max commands
#[test]
fn test_default_max_commands() {
    env::remove_var("TMUX_MCP_MAX_COMMANDS");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.max_commands, 1000);
}

/// Test custom max commands from env var
#[test]
fn test_custom_max_commands() {
    env::remove_var("TMUX_MCP_MAX_COMMANDS");
    env::set_var("TMUX_MCP_MAX_COMMANDS", "500");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.max_commands, 500);

    env::remove_var("TMUX_MCP_MAX_COMMANDS");
}

/// Test default command TTL
#[test]
fn test_default_command_ttl() {
    env::remove_var("TMUX_MCP_COMMAND_TTL");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.command_ttl_seconds, 600);
}

/// Test custom command TTL from env var
#[test]
fn test_custom_command_ttl() {
    env::remove_var("TMUX_MCP_COMMAND_TTL");
    env::set_var("TMUX_MCP_COMMAND_TTL", "300");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.command_ttl_seconds, 300);

    env::remove_var("TMUX_MCP_COMMAND_TTL");
}

/// Test invalid max commands falls back to default
#[test]
fn test_invalid_max_commands_falls_back() {
    env::remove_var("TMUX_MCP_MAX_COMMANDS");
    env::set_var("TMUX_MCP_MAX_COMMANDS", "not_a_number");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.max_commands, 1000); // Falls back to default

    env::remove_var("TMUX_MCP_MAX_COMMANDS");
}

/// Test invalid command TTL falls back to default
#[test]
fn test_invalid_command_ttl_falls_back() {
    env::remove_var("TMUX_MCP_COMMAND_TTL");
    env::set_var("TMUX_MCP_COMMAND_TTL", "-100");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.command_ttl_seconds, 600); // Falls back to default

    env::remove_var("TMUX_MCP_COMMAND_TTL");
}

/// Test all env vars set together
#[test]
fn test_all_env_vars_together() {
    // First clear all env vars
    env::remove_var("TMUX_MCP_BIND_ADDR");
    env::remove_var("TMUX_MCP_MAX_COMMANDS");
    env::remove_var("TMUX_MCP_COMMAND_TTL");

    // Now set them
    env::set_var("TMUX_MCP_BIND_ADDR", "192.168.1.1:9000");
    env::set_var("TMUX_MCP_MAX_COMMANDS", "2000");
    env::set_var("TMUX_MCP_COMMAND_TTL", "1200");

    let config = tmux_mcp_server::config::Config::from_env().unwrap();
    assert_eq!(config.bind_addr, "192.168.1.1:9000");
    assert_eq!(config.max_commands, 2000);
    assert_eq!(config.command_ttl_seconds, 1200);

    env::remove_var("TMUX_MCP_BIND_ADDR");
    env::remove_var("TMUX_MCP_MAX_COMMANDS");
    env::remove_var("TMUX_MCP_COMMAND_TTL");
}
