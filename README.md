# Tmux MCP Server

Model Context Protocol server for interacting with tmux sessions over stateless Streamable HTTP. It lets compatible MCP clients read from, control, and observe terminal sessions through one shared local daemon.

## Features

- List and search tmux sessions
- View and navigate tmux windows and panes
- Capture and expose terminal content from any pane
- Execute commands in tmux panes and retrieve results (use it at your own risk ⚠️)
- Create new tmux sessions and windows
- Split panes horizontally or vertically with customizable sizes
- Kill tmux sessions, windows, and panes
- Shared HTTP MCP server supporting 50+ concurrent clients
- Bounded command state storage with TTL cleanup

## Prerequisites

- Rust toolchain (1.75+)
- tmux installed and running

## Building

```bash
# Clone the repository
git clone <repository-url>
cd tmux-mcp

# Build the release binary
cargo build --release

# The binary will be at target/release/tmux-mcp-server
```

## Usage

### Start the MCP Server

```bash
# Run the server (default: 127.0.0.1:8090)
cargo run --release

# Or with custom bind address
TMUX_MCP_BIND_ADDR=127.0.0.1:3000 cargo run --release

# Configure command registry limits
TMUX_MCP_MAX_COMMANDS=500 TMUX_MCP_COMMAND_TTL=300 cargo run --release
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TMUX_MCP_BIND_ADDR` | `127.0.0.1:8090` | HTTP server bind address |
| `TMUX_MCP_MAX_COMMANDS` | `1000` | Maximum commands stored in registry |
| `TMUX_MCP_COMMAND_TTL` | `600` | Command TTL in seconds |
| `TMUX_MCP_SHELL` | `bash` | Default shell type (bash/zsh/fish) |

### Connect a Streamable HTTP MCP Client

Use the shared MCP endpoint directly in clients that support Streamable HTTP:

- **MCP Endpoint**: `http://127.0.0.1:8090/mcp`
- **POST /mcp**: JSON-RPC requests and notifications
- **GET /mcp**: SSE stream

This server is intentionally HTTP-only. It does not provide a `stdio` entrypoint.

### Legacy Debugging Endpoints

These routes are still exposed for compatibility and manual debugging, but they are not the primary MCP transport:

- `GET /mcp/tools`
- `POST /mcp/tools/:name`
- `GET /mcp/resources`
- `GET /mcp/resources/:uri`

## Available Resources

- `tmux://sessions` - List all tmux sessions
- `tmux://pane/{paneId}` - View content of a specific tmux pane
- `tmux://command/{commandId}/result` - Results from executed commands

## Available Tools

- `list-sessions` - List all active tmux sessions
- `find-session` - Find a tmux session by name
- `list-windows` - List windows in a tmux session
- `list-panes` - List panes in a tmux window
- `capture-pane` - Capture content from a tmux pane
- `create-session` - Create a new tmux session
- `create-window` - Create a new window in a tmux session
- `split-pane` - Split a tmux pane horizontally or vertically with optional size
- `kill-session` - Kill a tmux session by ID
- `kill-window` - Kill a tmux window by ID
- `kill-pane` - Kill a tmux pane by ID
- `execute-command` - Execute a command in a tmux pane
- `get-command-result` - Get the result of an executed command

## Architecture

This is a Rust-based HTTP MCP server with the following characteristics:

- **Transport Layer**: Stateless Streamable HTTP using axum (default: 127.0.0.1:8090)
- **Protocol Layer**: MCP tools and resources routing
- **Tmux Layer**: Command execution, output parsing, error mapping
- **State Layer**: Bounded command registry with background TTL cleanup

The server supports multiple concurrent clients sharing the same process state.

## Development

```bash
# Run tests
cargo test --workspace

# Run with formatting and linting checks
cargo fmt --all --check
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

## License

MIT
