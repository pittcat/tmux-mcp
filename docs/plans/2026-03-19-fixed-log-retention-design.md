# Fixed Log Retention Design

**Goal:** Keep logs in a single fixed file at a fixed path and delete log entries older than 4 hours by rewriting the file contents during a periodic cleanup pass.

## Decision

Replace hourly file rotation with a single append-only `server.log` file under `~/.local/share/tmux-mcp/logs/`.

Each log line will start with a machine-parseable RFC3339 timestamp. A background task will scan the file every hour, keep only lines whose timestamps are within the last 4 hours, and rewrite the file in place. This preserves a fixed path while bounding storage growth.

## Implementation Notes

- Introduce a fixed-file tracing writer instead of `tracing-appender` rotation.
- Synchronize log appends and file pruning so cleanup cannot corrupt concurrent writes.
- Keep console logging separate from file logging and continue disabling ANSI in the file output.
- Update tests to assert fixed filename behavior and content-based 4-hour pruning.

## Risks

- Rewriting the same file while logs are being written can lose data unless writes and cleanup share the same lock.
- Content pruning requires a stable timestamp prefix format; the formatter must be explicit and tested.
