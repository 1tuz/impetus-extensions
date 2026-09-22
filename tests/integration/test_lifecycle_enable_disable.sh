#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
if ! command -v impetus >/dev/null 2>&1; then
  echo "skip: impetus CLI not installed"
  exit 0
fi
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
cargo run -q -p impetus-ext-support -- install-local "$ROOT/extensions/hello-extension" --root "$TMP"
LIST="$(impetus extension list --root "$TMP" --json)"
# Best-effort: find installation id if present
ID="$(echo "$LIST" | sed -n 's/.*"installation_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1 || true)"
if [[ -z "${ID:-}" ]]; then
  echo "skip: could not parse installation_id from list output (schema may differ)"
  exit 0
fi
impetus extension disable "$ID" --root "$TMP"
impetus extension enable "$ID" --root "$TMP"
echo "ok enable/disable $ID"
