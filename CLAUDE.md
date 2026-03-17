# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Model Context Protocol (MCP) server for tmux sessions over stateless Streamable HTTP. It exposes tmux functionality through a shared local HTTP daemon so multiple MCP clients can reuse the same process and state.

The server is implemented in Rust as a shared daemon process supporting 50+ concurrent clients with bounded state management.

## Build and Development Commands

```bash
# Build the release binary
cargo build --release

# Run the server (default: 127.0.0.1:8090)
cargo run --release

# Run tests
cargo test --workspace

# Format check
cargo fmt --all --check

# Lint check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run benchmarks
cargo bench --bench memory_profile

# Run specific test suites
cargo test --test protocol_parity
cargo test --test streamable_http
cargo test --test multi_client_http
cargo test --test command_registry_limits
cargo test --test tmux_integration
```

## Architecture

### Four-Layer Architecture

**src/transport/** - HTTP Transport Layer
- `mod.rs` - Module exports
- HTTP server using axum, listening on 127.0.0.1:8090 by default
- Primary MCP endpoint: `GET/POST /mcp`
- Legacy debugging routes: `/mcp/tools`, `/mcp/resources`

**src/mcp/** - Protocol Layer
- `mod.rs` - Module exports
- `protocol.rs` - Streamable HTTP MCP handling for `/mcp`
- `tools.rs` - MCP tools registration and handling (list-sessions, capture-pane, execute-command, etc.)
- `resources.rs` - MCP resources registration and handling (tmux://sessions, tmux://pane/{paneId}, etc.)
- Parameter validation, protocol version negotiation, notification handling, and structured responses

**src/tmux/** - Tmux Execution Layer
- `mod.rs` - Module exports
- `service.rs` - Tmux read/write business logic
- `parser.rs` - Tmux output parsing
- `command.rs` - Command execution and result tracking
- Executes tmux commands via subprocess
- Parses tmux output using format strings (e.g., `#{session_id}:#{session_name}`)
- Handles shell-specific exit code syntax ($? vs $status for fish)

**src/state/** - State Layer
- `mod.rs` - Module exports
- `command_registry.rs` - Bounded command state storage with TTL cleanup
- Default: 1000 max commands, 600 seconds TTL

### Command Execution Flow

The `execute-command` tool has three modes:

1. **Standard mode** (default): Wraps commands with markers to capture output and exit code
2. **rawMode**: Sends commands without markers - for REPL/interactive use; disables status tracking
3. **noEnter**: Sends keystrokes character-by-character without pressing Enter; for TUI navigation

Command status is tracked via the `CommandRegistry` and retrieved via `get-command-result` tool or `tmux://command/{id}/result` resource.

### Key Implementation Details

- Shell type (bash/zsh/fish) affects exit code marker generation via `get_end_marker_text()`
- Pane content capture defaults to last 200 lines; colors can be included with `-e` flag
- Command execution markers: `TMUX_MCP_START` and `TMUX_MCP_DONE_${exitCode}`
- Split pane direction: 'horizontal' = side-by-side (-h), 'vertical' = top/bottom (-v)
- Command registry cleanup runs every 60 seconds in background task

## File Structure

```
src/
  main.rs              # Server entry point
  lib.rs               # Library exports
  config.rs            # Server configuration (bind_addr, max_commands, command_ttl)
  error.rs             # Unified error types
  transport/
    mod.rs             # Transport layer module
    http.rs            # HTTP MCP transport
  mcp/
    mod.rs             # MCP module
    tools.rs           # Tools registration and handling
    resources.rs       # Resources registration and handling
  tmux/
    mod.rs             # Tmux module
    service.rs         # Tmux service implementation
    parser.rs          # Output parsing
    command.rs         # Command execution
  state/
    mod.rs             # State module
    command_registry.rs # Bounded command registry with TTL
tests/
  protocol_parity.rs   # Protocol alignment tests
  streamable_http.rs   # Streamable HTTP interoperability tests
  multi_client_http.rs # 50+ client concurrency tests
  command_registry_limits.rs # TTL/capacity tests
  tmux_integration.rs  # Real tmux integration tests
benches/
  memory_profile.rs    # Memory and throughput benchmarks
```

## Configuration

Environment variables:

- `TMUX_MCP_BIND_ADDR` - HTTP server bind address (default: 127.0.0.1:8090)
- `TMUX_MCP_MAX_COMMANDS` - Maximum commands in registry (default: 1000)
- `TMUX_MCP_COMMAND_TTL` - Command TTL in seconds (default: 600)
- `TMUX_MCP_SHELL` - Default shell type (default: bash)

## Dependencies

- `tokio` - Async runtime
- `axum` - HTTP framework
- `serde` - Serialization
- `thiserror` / `anyhow` - Error handling
- `uuid` - Command tracking IDs
- `chrono` - Time handling
- `tracing` - Logging
