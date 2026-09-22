#!/usr/bin/env bash
# Run all integration tests. MCP mock stdio tests always run.
# Impetus-dependent tests skip gracefully (exit 0) when CLI is missing.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

PASS=0
FAIL=0
SKIP=0

run_one() {
  local name="$1"
  echo
  echo "======== ${name} ========"
  local out rc
  set +e
  out="$(bash "${SCRIPT_DIR}/${name}" 2>&1)"
  rc=$?
  set -e
  echo "${out}"
  if [[ ${rc} -ne 0 ]]; then
    echo "→ FAIL (${rc})"
    FAIL=$((FAIL + 1))
    return 0
  fi
  if echo "${out}" | grep -q '^SKIP:'; then
    echo "→ SKIP"
    SKIP=$((SKIP + 1))
  else
    echo "→ PASS"
    PASS=$((PASS + 1))
  fi
}

# Always-runnable (no impetus / Chrome / rust-analyzer)
run_one test_mcp_browser_mock.sh
run_one test_mcp_lsp_mock.sh

# Packaging always; install skipped without impetus
run_one test_hello_skill.sh

# Needs impetus CLI
run_one test_lifecycle_enable_disable.sh

echo
echo "======== summary ========"
echo "pass=${PASS} skip=${SKIP} fail=${FAIL}"
if [[ ${FAIL} -ne 0 ]]; then
  exit 1
fi
exit 0
