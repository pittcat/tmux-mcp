# Tmux MCP Server

基于无状态的 Streamable HTTP 协议的 Model Context Protocol (MCP) 服务器，用于与 tmux 会话进行交互。它允许兼容的 MCP 客户端通过共享的本地守护进程来读取、控制和观察终端会话。

## 功能特性

- 列出和搜索 tmux 会话
- 查看和导航 tmux 窗口和窗格
- 捕获并暴露任意窗格的终端内容
- 在 tmux 窗格中执行命令并获取结果（请自行承担风险 ⚠️）
- 创建新的 tmux 会话和窗口
- 水平或垂直分割窗格，可自定义大小
- 终止 tmux 会话、窗口和窗格
- 支持 50+ 并发客户端的共享 HTTP MCP 服务器
- 带 TTL 清理的有界命令状态存储
- 固定日志文件 `server.log`，保留最近 4 小时

## 系统要求

- Rust 工具链 (1.75+)
- tmux 已安装并运行

## 快速安装（推荐）

使用提供的安装脚本一键安装并配置自动启动服务：

```bash
# 克隆仓库
git clone https://github.com/pittcat/tmux-mcp.git
cd tmux-mcp

# 从源码构建并安装（自动配置自动启动）
./install.sh

# 或使用已有的二进制文件
./install.sh --binary /path/to/tmux-mcp-server

# 卸载
./install.sh --uninstall
```

安装脚本支持：
- **macOS**：使用 `launchd` 用户级服务
- **Linux**：使用 `systemd` 用户级服务
- 自动配置环境变量和日志保留
- 无需 root 权限（安装到 `~/.local/bin`）

## 编译构建

```bash
# 克隆仓库
git clone <repository-url>
cd tmux-mcp

# 构建发布版本
cargo build --release

# 二进制文件位于 target/release/tmux-mcp-server
```

## 使用方法

### 启动 MCP 服务器

```bash
# 运行服务器（默认：127.0.0.1:8090）
cargo run --release

# 或使用自定义绑定地址
TMUX_MCP_BIND_ADDR=127.0.0.1:3000 cargo run --release

# 配置命令注册表限制
TMUX_MCP_MAX_COMMANDS=500 TMUX_MCP_COMMAND_TTL=300 cargo run --release
```

### 安装脚本选项

```bash
# 从源码构建并安装（默认）
./install.sh

# 使用已有的二进制文件
./install.sh --binary ./target/release/tmux-mcp-server

# 二进制文件已复制到 ~/.local/bin，仅配置服务
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
| `-b, --binary PATH` | 使用现有二进制文件路径，跳过构建 |
| `-s, --skip-build` | 跳过构建，假设二进制文件已在安装目录中 |
| `-i, --install-dir DIR` | 安装目录（默认：`~/.local/bin`） |
| `--bind ADDR` | 绑定地址（默认：`127.0.0.1:8090`） |
| `--max-cmd N` | 最大命令数（默认：`1000`） |
| `--ttl SECONDS` | 命令 TTL 秒数（默认：`600`） |
| `-u, --uninstall` | 卸载服务并删除二进制文件 |

### 服务管理

**macOS (launchd):**
```bash
# 检查状态
launchctl list | grep tmux-mcp-server

# 查看当前日志
tail -f "$HOME/Library/Application Support/tmux-mcp/logs/server.log"

# 查看日志目录
ls -la "$HOME/Library/Application Support/tmux-mcp/logs/"

# 重启服务
launchctl stop com.pittcat.tmux-mcp-server
launchctl start com.pittcat.tmux-mcp-server

# 停止服务
launchctl stop com.pittcat.tmux-mcp-server
```

**Linux (systemd):**
```bash
# 检查状态
systemctl --user status tmux-mcp-server

# 查看当前日志
tail -f ~/.local/share/tmux-mcp/logs/server.log

# 查看日志目录
ls -la ~/.local/share/tmux-mcp/logs/

# 重启服务
systemctl --user restart tmux-mcp-server

# 停止服务
systemctl --user stop tmux-mcp-server
```

### 日志保留

日志写入固定文件，每小时修剪以仅保留最近 4 小时的日志条目：
- 日志目录（macOS）：`~/Library/Application Support/tmux-mcp/logs/`
- 日志目录（Linux）：`~/.local/share/tmux-mcp/logs/`
- 文件名：`server.log`
- 自动清理：每小时检查一次，删除超过 4 小时的日志条目

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `TMUX_MCP_BIND_ADDR` | `127.0.0.1:8090` | HTTP 服务器绑定地址 |
| `TMUX_MCP_MAX_COMMANDS` | `1000` | 注册表中存储的最大命令数 |
| `TMUX_MCP_COMMAND_TTL` | `600` | 命令 TTL 秒数 |
| `TMUX_MCP_SHELL` | `bash` | 默认 shell 类型 (bash/zsh/fish) |

### 连接 Streamable HTTP MCP 客户端

在使用 Streamable HTTP 的 MCP 客户端中直接使用共享 MCP 端点：

- **MCP 端点**：`http://127.0.0.1:8090/mcp`
- **POST /mcp**：JSON-RPC 请求和通知
- **GET /mcp**：SSE 流

此服务器仅提供 HTTP 入口，不提供 `stdio` 入口点。

### 遗留调试端点

这些路由仍为兼容性和手动调试而暴露，但它们不是主要的 MCP 传输方式：

- `GET /mcp/tools`
- `POST /mcp/tools/:name`
- `GET /mcp/resources`
- `GET /mcp/resources/:uri`

## 可用资源

- `tmux://sessions` - 列出所有 tmux 会话
- `tmux://pane/{paneId}` - 查看特定 tmux 窗格的内容
- `tmux://command/{commandId}/result` - 执行命令的结果

## 可用工具

- `list-sessions` - 列出所有活动的 tmux 会话
- `find-session` - 按名称查找 tmux 会话
- `list-windows` - 列出 tmux 会话中的窗口
- `list-panes` - 列出 tmux 窗口中的窗格
- `capture-pane` - 捕获 tmux 窗格的内容
- `create-session` - 创建新的 tmux 会话
- `create-window` - 在 tmux 会话中创建新窗口
- `split-pane` - 水平或垂直分割 tmux 窗格，可设置大小
- `kill-session` - 终止 tmux 会话
- `kill-window` - 终止 tmux 窗口
- `kill-pane` - 终止 tmux 窗格
- `execute-command` - 在 tmux 窗格中执行命令
- `get-command-result` - 获取执行命令的结果

## 断连恢复

服务器为 MCP 客户端实现了断连恢复支持：

- **无状态设计**：服务器不维护持久连接状态，允许多次自由重连
- **SSE 保活**：GET /mcp 端点每 30 秒发送一次保活注释
- **认证降级**：当 Claude 遇到失败状态并点击"认证"时，服务器返回正确的 JSON 响应说明不支持 OAuth（而不是返回 404 空体）

### 恢复端点

以下端点优雅地处理恢复场景：

- `/mcp/auth` - 返回 JSON 说明不支持 OAuth
- `/oauth` - 返回 JSON 说明不支持 OAuth
- `/authorize` - 返回 JSON 说明不支持 OAuth

## 架构设计

这是一个基于 Rust 的 HTTP MCP 服务器，具有以下特点：

- **传输层**：使用 axum 的无状态 Streamable HTTP（默认：127.0.0.1:8090）
- **协议层**：MCP 工具和资源路由
- **Tmux 层**：命令执行、输出解析、错误映射
- **状态层**：带后台 TTL 清理的有界命令注册表

服务器支持多个并发客户端共享同一进程状态。

## 开发指南

```bash
# 运行测试
cargo test --workspace

# 运行格式和 lint 检查
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 运行基准测试
cargo bench --bench memory_profile

# 运行特定测试套件
cargo test --test protocol_parity
cargo test --test streamable_http
cargo test --test multi_client_http
cargo test --test command_registry_limits
cargo test --test tmux_integration
cargo test --test claude_reconnect_regression
```

### 回归测试

`claude_reconnect_regression` 测试套件验证重连和认证降级行为：

```bash
# 运行重连恢复测试
cargo test --test claude_reconnect_regression

# 运行所有测试包括重连恢复
cargo test --workspace
```

如需使用 Claude CLI 进行手动端到端测试，请参见 `scripts/repro_claude_mcp_failed_state.sh`。

## 许可证

MIT
