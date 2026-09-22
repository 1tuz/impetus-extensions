#!/usr/bin/env bash
# Thin wrapper kept for local smoke; prefer test_mcp_*_mock.sh + mcp_stdio_probe.py.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN_NAME="${1:?bin name}"
shift || true
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# Resolve package name: bin names match package names for first-party MCP exts.
cargo build -q -p "${BIN_NAME}" --bin "${BIN_NAME}"
BIN="${ROOT}/target/debug/${BIN_NAME}"
exec python3 "${SCRIPT_DIR}/mcp_stdio_probe.py" -- "${BIN}" "$@"
