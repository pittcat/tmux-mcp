#!/usr/bin/env bash
#
# Claude MCP Reconnection Recovery Regression Script
#
# This script tests the tmux-mcp server's ability to handle reconnection
# after server restart and proper auth fallback behavior.
#
# BEFORE FIX (buggy behavior):
#   - After server restart, Claude shows "failed" state and clicking reconnect doesn't work
#   - Clicking "Authenticate" leads to "SDK auth failed: HTTP 404: Invalid OAuth error response"
#
# AFTER FIX (expected behavior):
#   - Server returns proper JSON responses for auth discovery endpoints (/mcp/auth, /oauth, /authorize)
#   - No more empty-body 404 responses that trigger OAuth errors
#   - Server supports stateless reconnection (no persistent state required)
#
# NOTE: This script requires Claude CLI to be installed and configured.
# For automated testing, see:
#   - cargo test --test claude_reconnect_regression (auth fallback tests)
#   - cargo test --test streamable_http (protocol tests)
#
# The automated tests verify the core recovery behavior without requiring Claude CLI.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${TMUX_MCP_REPRO_PORT:-8090}"
RESTART_DELAY="${TMUX_MCP_RESTART_DELAY:-12}"
POST_RESTART_WAIT="${TMUX_MCP_POST_RESTART_WAIT:-15}"
DEBUG_LOG="${TMUX_MCP_DEBUG_LOG:-/tmp/tmux-mcp-claude-repro-debug.txt}"
SERVER_LOG="${TMUX_MCP_SERVER_LOG:-/tmp/tmux-mcp-claude-repro-server.log}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

need_cmd claude
need_cmd expect
need_cmd cargo
need_cmd lsof

cleanup() {
  if [[ -n "${EXPECT_PID:-}" ]]; then
    kill "$EXPECT_PID" >/dev/null 2>&1 || true
  fi
}

trap cleanup EXIT

echo "[repro] stopping listeners on :$PORT"
if pids="$(lsof -tiTCP:"$PORT" -sTCP:LISTEN 2>/dev/null)"; then
  if [[ -n "$pids" ]]; then
    kill $pids >/dev/null 2>&1 || true
  fi
fi

echo "[repro] scheduling tmux-mcp restart in ${RESTART_DELAY}s"
(
  sleep "$RESTART_DELAY"
  cd "$ROOT_DIR"
  cargo run --release >"$SERVER_LOG" 2>&1 &
) &

echo "[repro] writing Claude debug log to $DEBUG_LOG"
echo "[repro] writing server log to $SERVER_LOG"

TMUX_MCP_DEBUG_LOG="$DEBUG_LOG" \
TMUX_MCP_POST_RESTART_WAIT="$POST_RESTART_WAIT" \
expect <<'EOF'
log_user 1
set timeout 120
set debug_log $env(TMUX_MCP_DEBUG_LOG)
set post_restart_wait $env(TMUX_MCP_POST_RESTART_WAIT)

spawn claude --debug-file $debug_log
sleep 3

send "/mcp\r"
expect -re {tmux-mcp.*failed}
send "\r"

expect -re {Tmux-mcp MCP Server}
expect -re {Status: .*failed}
puts "REPRO_STEP initial_failed_detail"

# Let the tmux-mcp server come back up, but stay inside the same Claude session.
sleep $post_restart_wait

send "\033"
expect -re {Manage MCP servers}
expect -re {tmux-mcp.*failed}
puts "REPRO_STEP stale_failed_after_server_restart"

send "\r"
expect -re {Tmux-mcp MCP Server}
send "1"
expect -re {SDK auth failed: HTTP 404: Invalid OAuth error response}
puts "REPRO_STEP auth_misroute_404"

sleep 2
send "\003"
expect eof
EOF

echo
echo "[repro] complete"
echo "[repro] Claude debug log: $DEBUG_LOG"
echo "[repro] Server log: $SERVER_LOG"
