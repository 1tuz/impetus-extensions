#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cargo build -p impetus-ext-lsp -q
"$ROOT/tests/integration/mcp_stdio_smoke.sh" impetus-ext-lsp --mock
