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

## Quick Install (Recommended)

使用提供的安装脚本一键安装并配置开机自启：

```bash
# 克隆仓库
git clone https://github.com/pittcat/tmux-mcp.git
cd tmux-mcp

# 从源码构建并安装（自动配置开机自启）
./install.sh

# 或使用已有的二进制文件
./install.sh --binary /path/to/tmux-mcp-server

# 卸载
./install.sh --uninstall
```

安装脚本支持：
- **macOS**: 使用 `launchd` 用户级服务
- **Linux**: 使用 `systemd` 用户级服务
- 自动配置环境变量和日志轮转
- 无需 root 权限（安装到 `~/.local/bin`）

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
# 从源码构建并安装（默认）
./install.sh

# 使用已有的二进制文件
./install.sh --binary ./target/release/tmux-mcp-server

# 二进制已手动复制到 ~/.local/bin，只配置服务
./install.sh --skip-build

# 自定义安装目录
./install.sh --install-dir /usr/local/bin

# 自定义绑定地址和配置
./install.sh --bind 127.0.0.1:3000 --max-cmd 500 --ttl 300

# 显示帮助
./install.sh --help
```

| 选项 | 说明 |
|------|------|
| `-b, --binary PATH` | 使用已有的二进制文件路径，跳过构建 |
| `-s, --skip-build` | 跳过构建，假设二进制已在安装目录 |
| `-i, --install-dir DIR` | 安装目录 (默认: `~/.local/bin`) |
| `--bind ADDR` | 绑定地址 (默认: `127.0.0.1:8090`) |
| `--max-cmd N` | 最大命令数 (默认: `1000`) |
| `--ttl SECONDS` | 命令TTL秒数 (默认: `600`) |
| `-u, --uninstall` | 卸载服务和二进制 |

### Service Management

**macOS (launchd):**
```bash
# 查看状态
launchctl list | grep tmux-mcp-server

# 查看日志
tail -f ~/.local/share/tmux-mcp/logs/server.log

# 重启服务
launchctl stop com.pittcat.tmux-mcp-server
launchctl start com.pittcat.tmux-mcp-server

# 停止服务
launchctl stop com.pittcat.tmux-mcp-server
```

**Linux (systemd):**
```bash
# 查看状态
systemctl --user status tmux-mcp-server

# 查看日志
journalctl --user -u tmux-mcp-server -f

# 重启服务
systemctl --user restart tmux-mcp-server

# 停止服务
systemctl --user stop tmux-mcp-server
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
