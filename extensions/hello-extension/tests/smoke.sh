#!/usr/bin/env bash
# Smoke: SKILL.md frontmatter + package.toml shape. No impetus binary required.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SKILL="$ROOT/SKILL.md"
PKG="$ROOT/package.toml"
fail=0

need() {
  local file=$1 key=$2
  if ! grep -qE "$key" "$file"; then
    echo "missing in $(basename "$file"): $key" >&2
    fail=1
  fi
}

[[ -f "$SKILL" ]] || { echo "missing SKILL.md"; exit 1; }
[[ -f "$PKG" ]] || { echo "missing package.toml"; exit 1; }

# YAML frontmatter delimiters
first="$(head -n1 "$SKILL")"
[[ "$first" == '---' ]] || { echo "SKILL.md: missing leading ---"; fail=1; }
awk 'NR>1 && /^---$/{found=1; exit} END{exit !found}' "$SKILL" \
  || { echo "SKILL.md: missing closing ---"; fail=1; }

need "$SKILL" '^name:[[:space:]]*hello-extension'
need "$SKILL" '^description:'
need "$SKILL" '^version:[[:space:]]*"?0\.1\.0"?'

need "$PKG" '^id = "hello-extension"'
need "$PKG" '^version = "0.1.0"'
need "$PKG" '^kind = "skill"'
need "$PKG" '^permissions = \[\]'
need "$PKG" 'extension_api_version = "0.1.0-skill-mcp"'
need "$PKG" 'impetus_tag = "v0.1.2"'
need "$PKG" 'extension_schema = "impetus.extension.v1@1"'
need "$PKG" 'mcp_schema = "impetus.mcp.v1@1"'

if [[ "$fail" -ne 0 ]]; then
  echo "smoke: FAIL"
  exit 1
fi

# Packaging + impetus.extension.v1 validation (cargo / support crate; no impetus CLI)
REPO="$(cd "$ROOT/../.." && pwd)"
cd "$REPO"
cargo test -p impetus-ext-support package_hello_extension_from_repo

echo "smoke: OK"
