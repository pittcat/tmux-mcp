# 实施计划

## 阶段 1：准备

- [ ] 运行 `bash scripts/repro_claude_mcp_failed_state.sh`，确认当前失败链路仍可稳定复现。
- [ ] 运行 `cargo test --test streamable_http`，确认现有协议基线。
- [ ] 对照当前失败现象，整理需要由服务端承担的恢复行为与 auth 误路径兼容行为。
- [ ] 明确门禁：当前阶段未形成稳定基线前，不进入 transport 重构。

## 阶段 2：核心实现

- [ ] 在 `src/transport/` 下新增实际 HTTP transport 模块。
- [ ] 调整 `src/main.rs` 路由装配，使恢复逻辑进入 transport 层。
- [ ] 调整 `src/mcp/protocol.rs`，将其职责收缩为 JSON-RPC 协议语义处理。
- [ ] 修改 `src/lib.rs` 与 `src/transport/mod.rs`，暴露新的 transport 结构。
- [ ] 为 Claude 的 `Reconnect` 恢复路径实现明确且可测试的服务端兼容行为。
- [ ] 为误入的 auth discovery 路径实现无认证兼容响应，消除空体 `404`。
- [ ] 保持 `initialize`、`tools/list`、`resources/list` 等基础 MCP 行为不回归。
- [ ] 当前阶段每次改动后先验证局部行为，未通过前不得进入下一项。

## 阶段 3：验证与测试

- [ ] 新增 `tests/claude_reconnect_regression.rs`，覆盖初始化、断连恢复、错误 auth 路径。
- [ ] 修改 `tests/streamable_http.rs`，补充恢复与兼容性断言。
- [ ] 将 `scripts/repro_claude_mcp_failed_state.sh` 从诊断脚本改为通过型回归门禁。
- [ ] 运行 `cargo test --test streamable_http`。
- [ ] 运行 `cargo test --test claude_reconnect_regression`。
- [ ] 运行修复后的 `bash scripts/repro_claude_mcp_failed_state.sh`。
- [ ] 运行 `cargo fmt --all`。
- [ ] 运行 `cargo clippy --workspace --all-targets --all-features -- -D warnings`。
- [ ] 必要时运行 `cargo test --workspace`，确认其他行为未回归。
- [ ] 在 Claude 交互式 `/mcp` 中手工验证恢复路径，确认无需 kill Claude 或 kill server。

## 阶段 4：完成条件

- [ ] README 与 README_zh 中关于恢复行为、验证方式、已知限制的说明已同步更新。
- [ ] 回归脚本不再停留在 `stale_failed_after_server_restart`。
- [ ] 不再出现 `SDK auth failed: HTTP 404: Invalid OAuth error response`。
- [ ] 自动化测试、格式化和 lint 全部通过。
- [ ] 所有计划内改动都在声明范围内，没有通过弱化断言、硬编码返回值或跳过校验来伪造完成。
- [ ] 没有未记录的 blocker，且 requirements.md 中的验收标准全部满足。
