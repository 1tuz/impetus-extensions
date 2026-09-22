#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
./tests/integration/test_mcp_browser_mock.sh
./tests/integration/test_mcp_lsp_mock.sh
./tests/integration/test_hello_skill.sh
./tests/integration/test_lifecycle_enable_disable.sh
echo "all integration scripts finished"
