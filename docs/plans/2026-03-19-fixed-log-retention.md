# Fixed Log Retention Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace hourly rotated log files with a single `server.log` file that is scanned every hour and pruned to only keep the last 4 hours of log lines.

**Architecture:** The logging subsystem will switch to a fixed-file writer guarded by shared synchronization so append operations and cleanup rewrites cannot race. File logs will use an explicit RFC3339 timestamp prefix so the cleanup task can parse each line and retain only recent entries.

**Tech Stack:** Rust, `tracing`, `tracing-subscriber`, `tokio`, `chrono`

---

### Task 1: Lock the New Behavior with Tests

**Files:**
- Modify: `tests/log_rotation.rs`
- Test: `tests/log_rotation.rs`

**Step 1: Write the failing test**

Add tests that require:
- logs are written to a fixed `server.log` file
- cleanup removes lines older than 4 hours from that file

**Step 2: Run test to verify it fails**

Run: `cargo test --test log_rotation -- --nocapture`
Expected: FAIL because the current implementation rotates files and does not prune file contents.

### Task 2: Replace Rotation with a Fixed-File Writer

**Files:**
- Modify: `src/logging.rs`
- Modify: `src/main.rs`

**Step 1: Write minimal implementation**

- remove hourly file rotation
- write file logs to a fixed `server.log`
- use a parseable timestamp prefix
- share synchronization between append and prune paths

**Step 2: Run targeted tests**

Run: `cargo test --test log_rotation -- --nocapture`
Expected: PASS

### Task 3: Verify the Whole Logging Surface

**Files:**
- Modify: `README.md`

**Step 1: Update docs**

- document fixed log filename
- document hourly content pruning

**Step 2: Run broader verification**

Run: `cargo test`
Expected: PASS
