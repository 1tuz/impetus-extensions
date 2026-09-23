#!/usr/bin/env bash
# Smoke: skills/SKILL.md frontmatter + extension.toml shape. No impetus binary required.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SKILL="$ROOT/skills/SKILL.md"
MANIFEST="$ROOT/extension.toml"
fail=0

need() {
  local file=$1 key=$2
  if ! grep -qE "$key" "$file"; then
    echo "missing in $(basename "$file"): $key" >&2
    fail=1
  fi
}

[[ -f "$SKILL" ]] || { echo "missing skills/SKILL.md"; exit 1; }
[[ -f "$MANIFEST" ]] || { echo "missing extension.toml"; exit 1; }
[[ ! -f "$ROOT/package.toml" ]] || { echo "legacy package.toml must not exist in sources"; exit 1; }

# YAML frontmatter delimiters
first="$(head -n1 "$SKILL")"
[[ "$first" == '---' ]] || { echo "SKILL.md: missing leading ---"; fail=1; }
awk 'NR>1 && /^---$/{found=1; exit} END{exit !found}' "$SKILL" \
  || { echo "SKILL.md: missing closing ---"; fail=1; }

need "$SKILL" '^id:[[:space:]]*hello-extension'
need "$SKILL" '^description:'
need "$SKILL" '^version:[[:space:]]*"?0\.1\.0"?'

need "$MANIFEST" '^id = "hello-extension"'
need "$MANIFEST" '^version = "0.1.0"'
need "$MANIFEST" 'kind = "instruction_pack"'
need "$MANIFEST" '^permissions = \[\]'
need "$MANIFEST" 'extension_api_version = 1'

if [[ "$fail" -ne 0 ]]; then
  echo "smoke: FAIL"
  exit 1
fi

# Packaging + validation (cargo / support crate; no impetus CLI)
REPO="$(cd "$ROOT/../.." && pwd)"
cd "$REPO"
cargo test -p impetus-ext-support package_hello_extension_from_repo

echo "smoke: OK"
