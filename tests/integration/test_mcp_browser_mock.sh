#!/usr/bin/env bash
# Always-runnable: build impetus-ext-browser and probe MCP over stdio (--mock).
# No Chrome, no network, no impetus daemon.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

cd "${REPO_ROOT}"
ensure_python

PKG="impetus-ext-browser"
BIN_NAME="impetus-ext-browser"

info "building ${BIN_NAME} (mock path)"
BIN="$(build_bin "${PKG}" "${BIN_NAME}")"

info "MCP stdio: initialize / tools/list / tools/call"
"${PYTHON}" "${SCRIPT_DIR}/mcp_stdio_probe.py" \
  --expect-tool browser_status \
  --expect-tool browser_negotiate \
  --expect-tool browser_ensure_session \
  --call 'browser_status:{}' \
  --call 'browser_negotiate:{"protocol_version":"0.1"}' \
  --call 'browser_ensure_session:{"client_session_id":"int-test-1"}' \
  -- "${BIN}" --mock

echo "PASS: test_mcp_browser_mock"
