# tmux-mcp-server 日志 ANSI 乱码修复 - 交接文档

## 1. 当前任务目标

**问题描述**：
1. 日志文件 `~/.local/share/tmux-mcp/logs/server.log` 中出现 ANSI 转义序列乱码（`ESC[2m`, `ESC[0m`, `ESC[32m` 等）
2. 日志显示日期为 3月17日，与实际日期（3月20日）不符

**预期产出**：修复日志中的 ANSI 乱码问题，使日志可读

**完成标准**：
- 日志文件中不再包含 ANSI 转义序列
- 日志内容清晰可读
- 服务正常运行

---

## 2. 已完成内容

### 分析阶段
- 确认 ANSI 乱码来源：`tracing-subscriber 0.3` 的 `.with_ansi(false)` 配置未能完全移除 ANSI 代码
- 检查 `install.sh`：确认 launchd 服务配置正确，设置了 `RUST_LOG=info`
- 检查 `logging.rs`：确认代码实现了固定日志文件和 4 小时保留机制

### 修复尝试
- 在 `src/logging.rs` 中添加了 `strip_ansi_escapes()` 函数
- 修改了 `FixedFileWriter::write()` 以在写入前剥离 ANSI 序列
- 添加了 2 个单元测试验证 ANSI 剥离功能
- 测试在 `cargo test` 中全部通过（7 tests passed）

### 重构验证
- 代码编译成功：`cargo build --release` 通过
- 服务通过 `./install.sh --skip-build` 重启成功
- 服务能正常响应 HTTP 请求：`curl http://127.0.0.1:8090/mcp/tools` 返回正确 JSON

---

## 3. 尝试过什么

### 方案 A：使用 `strip_ansi_escapes()` 函数（当前实现）

**做法**：
- 添加 `strip_ansi_escapes(buf: &[u8]) -> Vec<u8>` 函数
- 在 `FixedFileWriter::write()` 中调用该函数处理输入

**代码位置**：`src/logging.rs:182-212`

**结果**：
- 单元测试通过
- 但实际运行时 ANSI 仍然出现
- **推测原因**：`tracing-subscriber` 的 `MakeWriter` trait 实现可能绕过了我们的 `Write` wrapper

### 方案 B：使用 `AnsiStrippingWriter` wrapper（已放弃）

**做法**：
- 创建 `AnsiStrippingWriter<W>` 包装器实现 `MakeWriter` trait
- 在 `init_logging()` 中使用包装器

**结果**：编译失败，`MakeWriter` trait 实现有兼容性问题，已回退

### 日志日期问题

**观察**：
- 日志文件权限：`drwxr-xr-x@` 目录，owner `pittcat`
- 日志最后修改：3月19日 00:52
- launchd 服务状态：PID 存在但 status=-9 表示异常

**推测**：服务在 3月19日后可能因其他原因未正常写入日志

---

## 4. 关键决策和理由

| 决策 | 理由 | 备选方案 |
|------|------|----------|
| 在 `FixedFileWriter` 内部剥离 ANSI，而非使用外部 wrapper | `MakeWriter` trait 实现复杂且有兼容性 | 使用外部 writer wrapper |
| 添加 `strip_ansi_escapes()` 而非使用 crate | 避免增加依赖 | 使用 `strip-ansi-regex` crate |
| 保留单元测试 | 验证函数逻辑正确性 | - |

---

## 5. 修改过的文件及改动说明

### `src/logging.rs`

**修改内容**：
1. 新增 `strip_ansi_escapes()` 函数（第 182-212 行）
   - 用途：从字节数组中移除 ANSI CSI 序列
   - 验证：单元测试 `test_strip_ansi_escapes_removes_color_codes` 通过

2. 修改 `FixedFileWriter::write()` 实现（第 159-165 行）
   - 在缓冲前调用 `strip_ansi_escapes()`
   - 验证：单元测试 `test_fixed_file_writer_strips_ansi_in_output` 通过

3. 新增单元测试（第 423-452 行）
   - `test_strip_ansi_escapes_removes_color_codes`
   - `test_fixed_file_writer_strips_ansi_in_output`

**状态**：代码编译通过，单元测试通过，但**未在生产环境验证有效**

### `README_zh.md`（新建）

- 位置：`/Users/pittcat/Dev/Rust/tmux-mcp/README_zh.md`
- 内容：完整的中文使用文档

---

## 6. 障碍 / 待决问题

### 未解决：ANSI 乱码仍出现在实际日志中

**现象**：直接运行 `~/.local/bin/tmux-mcp-server` 时，日志仍然包含：
```
[2m2026-03-20T03:52:30.693Z[0m [0m [32m INFO[0m Starting tmux-mcp-server...
```

**可能原因**：
1. `tracing-subscriber` 的 formatter 可能直接在内部 writer 上写入，未经过我们的 `Write` 实现
2. `MakeWriter` trait 的 `make_writer()` 方法创建的新 writer 可能旁路了我们的封装

**验证方法**：
```bash
# 清空日志后直接运行
: > ~/.local/share/tmux-mcp/logs/server.log
~/.local/bin/tmux-mcp-server
# 然后检查日志内容
cat ~/.local/share/tmux-mcp/logs/server.log
```

### 待验证：修复后的二进制是否真正解决 ANSI 问题

**当前状态**：
- 单元测试通过，但实际运行无效
- 需要进一步诊断 `tracing-subscriber` 的 writer 机制

### launchd 服务状态异常

**现象**：`launchctl list | grep tmux-mcp` 显示 `status=-9`
- -9 通常表示被 `SIGKILL` 终止
- 但服务仍能响应 HTTP 请求，说明有多个进程或服务已重启

---

## 7. 下一步计划

### 立即执行

1. **诊断 ANSI 问题根因**
   ```bash
   # 先杀掉所有 tmux-mcp-server 进程
   pkill -f tmux-mcp-server
   sleep 1
   # 清空日志
   : > ~/.local/share/tmux-mcp/logs/server.log
   # 直接运行（前台）观察输出
   ~/.local/bin/tmux-mcp-server
   # 在另一终端检查日志
   cat ~/.local/share/tmux-mcp/logs/server.log
   ```

2. **检查 `tracing-subscriber` writer 机制**
   - 参考 `src/logging.rs` 中 `FixedFileMakeWriter` 和 `FixedFileWriter` 的实现
   - 考虑是否需要实现自定义的 `MakeWriter` 而非依赖默认实现

3. **如果当前修复无效，考虑备选方案**：
   - 使用 `tracing-appender` crate 的非旋转写入器
   - 或者在后处理阶段清理日志文件中的 ANSI 序列

### 后续步骤

4. **验证日志日期问题**
   - 确认服务正常运行后，是否正常写入当前日期的日志

5. **重新启动 launchd 服务**
   ```bash
   launchctl stop com.pittcat.tmux-mcp-server
   launchctl unload ~/Library/LaunchAgents/com.pittcat.tmux-mcp-server.plist
   launchctl load ~/Library/LaunchAgents/com.pittcat.tmux-mcp-server.plist
   launchctl start com.pittcat.tmux-mcp-server
   ```

---

## 8. 重要注意事项

### 环境信息
- **平台**：macOS (Darwin)
- **tmux 版本**：tmux 3.6a
- **Rust 版本**：1.75+（项目要求）
- **tracing-subscriber 版本**：0.3

### 日志路径
- **日志目录**：`~/.local/share/tmux-mcp/logs/`
- **日志文件**：`~/.local/share/tmux-mcp/logs/server.log`
- **服务配置**：`~/Library/LaunchAgents/com.pittcat.tmux-mcp-server.plist`

### launchd 服务信息
- **服务名称**：`com.pittcat.tmux-mcp-server`
- **二进制路径**：`~/.local/bin/tmux-mcp-server`
- **端口**：127.0.0.1:8090

### 相关文件
- `src/logging.rs` - 日志子系统实现
- `install.sh` - 安装脚本

### 关键代码位置
- ANSI 剥离函数：`src/logging.rs:182-212` (`strip_ansi_escapes`)
- FixedFileWriter：`src/logging.rs:144-177`
- 日志初始化：`src/logging.rs:199-240` (`init_logging`)

---

## 9. 总结 Bullet Points

- **问题**：日志包含 ANSI 转义序列乱码 + 日志日期与实际不符
- **根因**：tracing-subscriber 0.3 的 `.with_ansi(false)` 配置未能完全移除 ANSI 代码
- **修复尝试**：在 `FixedFileWriter::write()` 中添加 `strip_ansi_escapes()` 函数
- **验证状态**：单元测试通过（7/7），但生产环境**未验证有效**
- **关键文件**：`src/logging.rs`（新增 ANSI 剥离逻辑和测试）
- **新增文件**：`README_zh.md`（中文文档，与英文版内容一致）
- **launchd 服务**：配置正确但 status=-9 异常，可能被 SIGKILL
- **HTTP 服务**：正常运行在 127.0.0.1:8090，可响应请求
- **待办**：需要实际运行验证 ANSI 修复是否生效，可能需要更深层的 tracing-subscriber writer 定制
- **日志保留**：代码实现正确（4小时保留，每小时清理一次）

---

## 10. 下一位 Agent 的第一步建议

**第一步**：验证 ANSI 修复是否真正有效

```bash
# 1. 杀掉所有相关进程
pkill -f tmux-mcp-server
sleep 1

# 2. 清空日志
: > ~/.local/share/tmux-mcp/logs/server.log

# 3. 直接运行二进制（前台模式，便于观察）
~/.local/bin/tmux-mcp-server
```

然后检查日志内容：
```bash
cat ~/.local/share/tmux-mcp/logs/server.log
hexdump -C ~/.local/share/tmux-mcp/logs/server.log | head -20
```

**为什么先做这一步**：
- 之前的测试都在单元测试层面通过，但实际运行无效
- 需要确认是 `tracing-subscriber` writer 机制问题，还是我们的代码逻辑问题
- 如果 ANSI 仍然存在，需要重新考虑使用 `tracing-appender` 或其他方案

**后续分支**：
- 如果 ANSI 已消除 → 验证 launchd 服务稳定性
- 如果 ANSI 仍存在 → 需要深入研究 `tracing-subscriber` 的 `MakeWriter` trait，可能需要实现自定义 formatter 或使用不同日志方案
