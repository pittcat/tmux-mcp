# 设计概述

将当前"协议处理 + HTTP 路由"混合实现，拆分为 `mcp protocol` 与 `transport` 两层。`src/mcp/protocol.rs` 保留 JSON-RPC 语义处理，新的 transport 层负责 Claude 恢复路径、错误 auth 路径和无认证兼容响应。

# 现有系统上下文

- 入口：`src/main.rs`
- 协议处理：`src/mcp/protocol.rs`
- transport 目录：`src/transport/mod.rs`（当前为占位）
- 导出层：`src/lib.rs`
- 现有测试：`tests/streamable_http.rs`、`tests/multi_client_http.rs`
- 回归脚本：`scripts/repro_claude_mcp_failed_state.sh`
- 文档：`README.md`、`README_zh.md`
- 依赖：Rust 2021、`tokio`、`axum`、`reqwest`、本地 `tmux`、Claude Code

# 方案设计

- 在 `src/transport/` 下新增实际 HTTP transport 模块，承载恢复相关的路由行为与错误响应策略。
- `protocol.rs` 只处理 JSON-RPC 请求、方法分发与协议语义，不再混入恢复策略。
- 对 Claude 的失败恢复链路提供明确、可测试的 transport 行为。
- 对误入的 auth discovery 路径提供可解析的无认证兼容响应，避免空体 `404`。
- 将现有复现脚本从"复现故障"调整为"验证修复通过"的门禁脚本。

# 数据流 / 控制流

1. Claude 通过 `POST /mcp` 发送 `initialize` 与后续 JSON-RPC 请求。
2. Claude 通过 `GET /mcp` 建立 SSE 或流式连接。
3. 服务短暂不可用后恢复时，Claude 的 `Reconnect` 请求进入 transport 层，由其完成恢复兼容处理。
4. Claude 在失败态点击 `Authenticate` 时，请求进入 transport 层，由其返回无认证兼容响应，而不是空体 `404`。
5. 自动化测试与回归脚本验证初始化、恢复、auth 误路径三类行为。

# 文件 / 模块改动

| 类型 | 路径 | 说明 |
|------|------|------|
| 新增 | `src/transport/http.rs` | 承载 HTTP transport、恢复逻辑、错误响应策略 |
| 新增 | `tests/claude_reconnect_regression.rs` | 覆盖恢复链路与 auth 误路径的回归测试 |
| 修改 | `src/main.rs` | 使用新的 transport 组装路由 |
| 修改 | `src/lib.rs` | 暴露 transport 模块 |
| 修改 | `src/mcp/protocol.rs` | 收缩为协议语义处理 |
| 修改 | `src/transport/mod.rs` | 从占位模块改为真实导出 |
| 修改 | `tests/streamable_http.rs` | 增补恢复与兼容性断言 |
| 修改 | `scripts/repro_claude_mcp_failed_state.sh` | 从诊断脚本升级为回归门禁 |
| 修改 | `README.md` | 更新恢复行为与验证方式说明 |
| 修改 | `README_zh.md` | 更新中文文档说明 |

# 边界情况

- 服务在 `GET /mcp` 建流前不可达，随后恢复。
- 服务在 SSE 建立后中断，再次恢复。
- 客户端在失败态点击 `Reconnect`。
- 客户端在失败态点击 `Authenticate`。
- 请求缺少或携带错误的 `MCP-Protocol-Version`。
- 客户端重复初始化或携带旧状态。
- Claude 本地保留过期 token，但服务本身不支持 OAuth。

# 风险与权衡

- Claude 的失败态缓存可能仍主导部分 UI 表现，服务端修复可能只能做到恢复成功但界面刷新滞后。
- 新增恢复语义时必须保证现有 stateless 无认证路径不回归。
- 若误判 Claude 对 auth discovery 的期望，可能引入新的兼容性问题。
- 将 transport 从占位目录升级为真实模块会增加结构变化，但能隔离恢复逻辑并降低后续维护成本。

# 测试策略

- 修改 `tests/streamable_http.rs`，覆盖恢复和错误 auth 响应断言。
- 新增 `tests/claude_reconnect_regression.rs`，覆盖初始化、断连恢复、错误 auth 路径。
- 保留并更新 `scripts/repro_claude_mcp_failed_state.sh` 作为端到端回归标准。
- 执行 `cargo fmt --all`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`、必要时 `cargo test --workspace`。
- 在 Claude 交互式 `/mcp` 中手工验证恢复路径，不允许依赖 kill Claude 或 kill server。
