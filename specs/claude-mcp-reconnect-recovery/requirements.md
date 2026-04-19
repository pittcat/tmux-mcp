# 功能概述

修复 `tmux-mcp` 在 Claude 交互式 `/mcp` 中的失败态恢复链路，使服务在短暂不可用后恢复时可以重新连接，并避免误入错误的 OAuth 认证路径。

# 范围内（In Scope）

- 修复 `GET /mcp` 与 `POST /mcp` 的恢复兼容行为。
- 修复 Claude 在失败态下点击 `Reconnect` 时的服务端恢复路径。
- 修复 Claude 在失败态下点击 `Authenticate` 时的无认证兼容响应，消除 `404 + 空体`。
- 将 `scripts/repro_claude_mcp_failed_state.sh` 升级为回归标准。
- 新增 Rust 自动化测试，覆盖恢复链路与错误 auth 路径。
- 保持现有基础 MCP JSON-RPC 行为不回归。

# 范围外（Out of Scope）

- 修改 Claude Code 客户端源码或本地 token 存储实现。
- 变更 tmux 工具语义、命令执行逻辑或 command registry TTL 策略。
- 引入真实 OAuth 登录能力。

# 功能性需求（Functional Requirements）

1. 服务在启动初期不可达、随后恢复时，Claude 的 `Reconnect` 路径必须能够重新建立 MCP 连接。
2. 服务在失败态下必须对误入的认证探测路径返回可解析的无认证响应，不能再返回空体 `404`。
3. `scripts/repro_claude_mcp_failed_state.sh` 必须能作为回归门禁区分"问题仍存在"和"修复已生效"。
4. 必须新增至少一组 Rust 自动化测试，覆盖初始化、断连后恢复、错误 auth 路径。
5. 现有 `initialize`、`tools/list`、`resources/list` 等基础 MCP 功能必须保持可用。

# 非功能性需求（Non-Functional Requirements）

- 兼容性：不能破坏现有无认证 Streamable HTTP 用法，也不能破坏其他基于 `reqwest` 的现有测试行为。
- 可验证性：每个修复目标都必须映射到脚本、自动化测试或人工检查点。
- 可回归性：复现脚本必须可重复执行，适合作为长期回归标准。
- 约束：修复后不能依赖"手工 kill server 再重进 Claude"才能恢复连接。
- 工程质量：`cargo fmt --all` 与 `cargo clippy --workspace --all-targets --all-features -- -D warnings` 必须通过。
- 范围控制：不得通过弱化断言、跳过校验、硬编码返回值来伪造修复结果。

# 验收标准（Acceptance Criteria）

- [ ] 运行修复后的 `bash scripts/repro_claude_mcp_failed_state.sh` 时，不再停留在 `stale_failed_after_server_restart`，而是进入可恢复或成功状态。
- [ ] 修复后的认证误路径不再触发 `SDK auth failed: HTTP 404: Invalid OAuth error response`。
- [ ] `cargo test --test streamable_http` 通过，且包含恢复相关断言。
- [ ] 新增的恢复回归测试通过，例如 `cargo test --test claude_reconnect_regression`。
- [ ] `cargo fmt --all` 与 `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。
- [ ] 在 Claude 交互式 `/mcp` 中手工验证时，服务恢复后无需 kill Claude 或 kill server 即可重新使用 `tmux-mcp`。
