Improve code quality in the tmux-mcp-server codebase by reducing clippy warnings and dead code.

Benchmark command: cargo clippy --all-targets 2>&1 | grep -c 'warning\[' || echo 0
Metric: clippy warning count (lower is better, 0 is perfect)
Parse metric: the single integer printed to stdout

Correctness gate: cargo test --workspace 2>&1 must end with "test result: ok" for all test suites. If any test fails, discard the experiment — no exceptions.

Files in scope: src/
Off limits: any test files (tests/), benches/, public API signatures in lib.rs

Constraints:
- cargo test --workspace must pass after every change
- Do not add new dependencies
- Do not change public-facing function signatures or struct fields
- One small focused change per experiment

Known issues to address (in priority order):
1. src/state/command_registry.rs uses std::sync::RwLock in async context — replace with tokio::sync::RwLock and make methods async
2. Capacity eviction bug in CommandRegistry::insert — when all commands are Pending, nothing gets evicted and max_commands is exceeded
3. Dead code: len(), is_empty() in command_registry.rs and is_tmux_running() in service.rs marked #[allow(dead_code)] — either use them or remove
4. ShellType hardcoded as Bash in tools.rs and protocol.rs — should read from config
5. capture-pane lines parameter schema mismatch between tools.rs (string) and protocol.rs (number)
6. Duplicate tool definitions between tools.rs and protocol.rs (~600 lines of duplication)

When done with all improvements, output: LOOP_COMPLETE
