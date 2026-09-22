#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="$1"
shift || true
export PATH="$ROOT/target/debug:$PATH"
if [[ ! -x "$ROOT/target/debug/$BIN" && ! -x "$(command -v "$BIN" || true)" ]]; then
  cargo build -p "$BIN" -q
fi
CMD="$(command -v "$BIN")"
# Send initialize + tools/list over stdio
{
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
  printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
} | "$CMD" "$@" | head -n 20 | tee /tmp/impetus-ext-mcp-smoke.out
grep -q '"result"' /tmp/impetus-ext-mcp-smoke.out
echo "ok mcp smoke $BIN"
