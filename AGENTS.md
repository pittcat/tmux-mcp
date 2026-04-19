# Repository Guidelines

## Project Structure & Module Organization

`src/main.rs` boots the shared HTTP daemon; `src/lib.rs` re-exports the crate modules. Keep protocol code in `src/mcp/`, tmux command execution and parsing in `src/tmux/`, bounded runtime state in `src/state/`, and transport wiring in `src/transport/`. Shared configuration, logging, and error types live in `src/config.rs`, `src/logging.rs`, and `src/error.rs`.

Integration and regression tests live in `tests/*.rs`. Behavioral fixtures and fake tmux helpers live under `tests/features/` and `tests/support/`. Benchmarks live in `benches/`, and longer-form implementation notes belong in `docs/plans/` or `docs/handoff/`.

## Build, Test, and Development Commands

Use the standard Cargo workflow:

- `cargo build --release` builds the production binary at `target/release/tmux-mcp-server`.
- `cargo run --release` starts the server on `127.0.0.1:8090` by default.
- `cargo test --workspace` runs the full test suite.
- `cargo test --test streamable_http` runs a focused transport/protocol test.
- `cargo test --test tmux_integration` exercises real tmux behavior; requires `tmux` to be installed.
- `cargo fmt --all` formats the codebase.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` treats lints as errors.
- `cargo bench --bench memory_profile` runs the benchmark harness.

## Coding Style & Naming Conventions

Target Rust 2021 and keep formatting compatible with `rustfmt` defaults (4-space indentation). Use `snake_case` for modules, files, and functions; `PascalCase` for types; and `SCREAMING_SNAKE_CASE` for constants. Prefer small, layer-specific functions over cross-module shortcuts. Route logs through `tracing`, not `println!`, and keep protocol/config errors flowing through the shared error types.

## Testing Guidelines

Add integration tests in `tests/` for public behavior, especially MCP protocol parity, tmux command lifecycle, and registry limits. Name new test files after the behavior under test, such as `tests/session_listing.rs`. Run `cargo test --workspace` before opening a PR; run targeted suites while iterating.

## Commit & Pull Request Guidelines

Recent history favors short, imperative subjects, often with Conventional Commit prefixes such as `refactor:`. Follow that pattern when possible, for example `fix: tighten command registry cleanup`. PRs should summarize the user-visible change, note config or environment impacts, link related issues, and list the validation commands you ran. Include screenshots only when a change affects rendered docs or other visible output.
