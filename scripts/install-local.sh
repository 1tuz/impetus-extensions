#!/usr/bin/env bash
# Wrap impetus-ext install-local for a local Impetus project root.
#
# Usage:
#   ./scripts/install-local.sh <extension-dir> --root <impetus-project-root> [--daemon-mcp]
#
# Examples:
#   ./scripts/install-local.sh extensions/hello-extension --root ~/code/my-app
#   ./scripts/install-local.sh dist/browser --root . --daemon-mcp
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <extension> --root <project-root> [--daemon-mcp]" >&2
  exit 2
fi

exec cargo run -p impetus-ext-support -- install-local "$@"
