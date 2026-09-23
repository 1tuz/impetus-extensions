#!/usr/bin/env bash
# Full public lifecycle: install → list → disable → enable (needs impetus CLI).
#
# Optional MCP daemon SoT: if IMPETUS_DATA_DIR writable, copy browser mcp.json.
# Probe with impetusd only noted when binary present (no stable public probe API).
#
# Skips (exit 0) when impetus is not installed — CI without Impetus stays green.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

cd "${REPO_ROOT}"
ensure_python
assert_compat_tag
require_impetus_or_skip

EXT="${REPO_ROOT}/extensions/hello-extension"
[[ -f "${EXT}/extension.toml" ]] || fail "missing ${EXT}/extension.toml"

OUT="$(mktemp -d "${TMPDIR:-/tmp}/life-pkg.XXXXXX")"
PROJECT="$(make_temp_project)"
DATA_DIR="$(mktemp -d "${TMPDIR:-/tmp}/impetus-data.XXXXXX")"
trap 'rm -rf "${OUT}" "${PROJECT}" "${DATA_DIR}"' EXIT

info "package + install hello-extension"
LAYOUT="$(package_extension "${EXT}" "${OUT}")"
INSTALL_JSON="$("${IMPETUS}" extension install --kind skill "${LAYOUT}" --root "${PROJECT}" --json)"
INSTALL_ID="$("${PYTHON}" -c 'import json,sys; print(json.load(sys.stdin)["installation_id"])' <<<"${INSTALL_JSON}")"
[[ -n "${INSTALL_ID}" ]] || fail "empty installation_id"

status_of() {
  local id="$1"
  local list_json
  list_json="$("${IMPETUS}" extension list --root "${PROJECT}" --json)"
  "${PYTHON}" -c '
import json,sys
want=sys.argv[1]
rows=json.load(sys.stdin)
for r in rows:
    if r.get("installation_id")==want:
        print(r.get("status"))
        raise SystemExit(0)
raise SystemExit("installation not listed: "+want)
' "${id}" <<<"${list_json}"
}

info "list after install"
ST="$(status_of "${INSTALL_ID}")"
info "status=${ST}"

info "disable"
"${IMPETUS}" extension disable "${INSTALL_ID}" --root "${PROJECT}" --json >/dev/null
ST="$(status_of "${INSTALL_ID}")"
info "status after disable=${ST}"
echo "${ST}" | grep -qi 'disabled' || fail "expected Disabled status, got ${ST}"

info "enable"
"${IMPETUS}" extension enable "${INSTALL_ID}" --root "${PROJECT}" --json >/dev/null
ST="$(status_of "${INSTALL_ID}")"
info "status after enable=${ST}"
echo "${ST}" | grep -qi 'enabled' || fail "expected Enabled status, got ${ST}"

# --- optional MCP install + daemon SoT ---
BROWSER_EXT="${REPO_ROOT}/extensions/browser"
if [[ -f "${BROWSER_EXT}/mcp.json" && -f "${BROWSER_EXT}/extension.toml" ]]; then
  info "optional: install browser MCP config via public CLI"
  B_LAYOUT="$(package_extension "${BROWSER_EXT}" "${OUT}")"
  MCP_SRC="${B_LAYOUT}/mcp.json"
  if [[ ! -f "${MCP_SRC}" ]]; then
    MCP_SRC="${BROWSER_EXT}/mcp.json"
  fi
  if MCP_INSTALL="$("${IMPETUS}" extension install --kind mcp "${MCP_SRC}" --root "${PROJECT}" --json)"; then
    info "MCP install OK (project .impetus/mcp)"
  else
    info "MCP install returned non-zero — continuing (config/binary PATH may differ)"
  fi

  export IMPETUS_DATA_DIR="${DATA_DIR}"
  mkdir -p "${IMPETUS_DATA_DIR}/mcp"
  cp "${BROWSER_EXT}/mcp.json" "${IMPETUS_DATA_DIR}/mcp/browser.json"
  info "copied daemon MCP SoT → ${IMPETUS_DATA_DIR}/mcp/browser.json"

  if DAEMON_BIN="$(resolve_impetusd_bin)"; then
    info "impetusd available at ${DAEMON_BIN}; light probe not automated (no stable public probe flag). File SoT present."
  else
    info "impetusd absent — daemon reload not probed (SoT file still written)"
  fi
else
  info "browser extension packaging incomplete — MCP daemon branch skipped"
fi

echo "PASS: test_lifecycle_enable_disable"
info "Covered: install/list/disable/enable for skill. AgentLoop inject: NOT covered."
