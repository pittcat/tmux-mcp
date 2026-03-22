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
- Fixed `server.log` file with 4-hour retention

## Prerequisites

- Rust toolchain (1.75+)
- tmux installed and running

## Quick Install (Recommended)

One-click installation with auto-start service configuration using the provided install script:

```bash
# Clone the repository
git clone https://github.com/pittcat/tmux-mcp.git
cd tmux-mcp

# Build from source and install (auto-configures auto-start)
./install.sh

# Or use an existing binary
./install.sh --binary /path/to/tmux-mcp-server

# Uninstall
./install.sh --uninstall
```

The install script supports:
- **macOS**: Uses `launchd` user-level service
- **Linux**: Uses `systemd` user-level service
- Auto-configures environment variables and log retention
- No root required (installs to `~/.local/bin`)

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

### Install Script Options

```bash
# Build from source and install (default)
./install.sh

# Use an existing binary
./install.sh --binary ./target/release/tmux-mcp-server

# Binary already copied to ~/.local/bin, configure service only
./install.sh --skip-build

# Custom install directory
./install.sh --install-dir /usr/local/bin

# Custom bind address and configuration
./install.sh --bind 127.0.0.1:3000 --max-cmd 500 --ttl 300

# Show help
./install.sh --help
```

| Option | Description |
|------|------|
| `-b, --binary PATH` | Use existing binary path, skip build |
| `-s, --skip-build` | Skip build, assume binary is already in install directory |
| `-i, --install-dir DIR` | Install directory (default: `~/.local/bin`) |
| `--bind ADDR` | Bind address (default: `127.0.0.1:8090`) |
| `--max-cmd N` | Maximum commands (default: `1000`) |
| `--ttl SECONDS` | Command TTL in seconds (default: `600`) |
| `-u, --uninstall` | Uninstall service and binary |

### Service Management

**macOS (launchd):**
```bash
# Check status
launchctl list | grep tmux-mcp-server

# View current logs
tail -f "$HOME/Library/Application Support/tmux-mcp/logs/server.log"

# View log directory
ls -la "$HOME/Library/Application Support/tmux-mcp/logs/"

# Restart service
launchctl stop com.pittcat.tmux-mcp-server
launchctl start com.pittcat.tmux-mcp-server

# Stop service
launchctl stop com.pittcat.tmux-mcp-server
```

**Linux (systemd):**
```bash
# Check status
systemctl --user status tmux-mcp-server

# View current logs
tail -f ~/.local/share/tmux-mcp/logs/server.log

# View log directory
ls -la ~/.local/share/tmux-mcp/logs/

# Restart service
systemctl --user restart tmux-mcp-server

# Stop service
systemctl --user stop tmux-mcp-server
```

### Log Retention

Logs are written to a fixed file and pruned hourly to keep only the last 4 hours of log entries:
- Log directory (macOS): `~/Library/Application Support/tmux-mcp/logs/`
- Log directory (Linux): `~/.local/share/tmux-mcp/logs/`
- Filename: `server.log`
- Auto cleanup: Checks every hour and removes log entries older than 4 hours

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
