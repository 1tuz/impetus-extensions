#!/usr/bin/env bash
# Always-runnable: build impetus-ext-lsp and probe MCP over stdio (--mock).
# No rust-analyzer binary, no network, no impetus daemon.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

cd "${REPO_ROOT}"
ensure_python

PKG="impetus-ext-lsp"
BIN_NAME="impetus-ext-lsp"

info "building ${BIN_NAME} (mock path)"
BIN="$(build_bin "${PKG}" "${BIN_NAME}")"

info "MCP stdio: initialize / tools/list / tools/call (mock LS)"
"${PYTHON}" "${SCRIPT_DIR}/mcp_stdio_probe.py" \
  --expect-tool lsp_start \
  --expect-tool lsp_diagnostics \
  --expect-tool lsp_stop \
  --call 'lsp_start:{"command":"mock-ls","workspace":"."}' \
  --call 'lsp_diagnostics:{}' \
  --call 'lsp_stop:{}' \
  -- "${BIN}" --mock

echo "PASS: test_mcp_lsp_mock"
