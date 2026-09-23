#!/usr/bin/env bash
# Package hello-extension skill; optionally install via impetus CLI into a temp project.
#
# Always covers: packaging → valid impetus.extension.v1 manifest.
# With impetus: install + list (public API only).
#
# Honest limit: Impetus AgentLoop skill inject may be incomplete — this test does
# NOT assert skills are injected into a live agent session.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${SCRIPT_DIR}/lib.sh"

cd "${REPO_ROOT}"
ensure_python
assert_compat_tag

EXT="${REPO_ROOT}/extensions/hello-extension"
[[ -f "${EXT}/extension.toml" && -f "${EXT}/skills/SKILL.md" ]] \
  || fail "missing hello-extension sources under ${EXT}"

OUT="$(mktemp -d "${TMPDIR:-/tmp}/hello-pkg.XXXXXX")"
PROJECT=""
trap 'rm -rf "${OUT}" ${PROJECT:+"${PROJECT}"}' EXIT

info "packaging hello-extension → ${OUT}"
LAYOUT="$(package_extension "${EXT}" "${OUT}")"
[[ -d "${LAYOUT}" ]] || fail "package did not print layout dir"
[[ -f "${LAYOUT}/manifest.json" ]] || fail "missing manifest.json in ${LAYOUT}"
[[ -f "${LAYOUT}/SKILL.md" ]] || fail "missing SKILL.md in ${LAYOUT}"

"${PYTHON}" - "${LAYOUT}/manifest.json" <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))
assert m.get("schema_version") == 1, m
assert m.get("id") == "hello-extension", m
assert m.get("kind") == "skill", m
assert m.get("digest", "").startswith("sha256:"), m
caps = set(m.get("capabilities") or [])
assert "instructions" in caps and "triggers" in caps, caps
print("OK: packaged manifest validates documented impetus.extension.v1 fields")
PY

if ! resolve_impetus_bin >/dev/null; then
  info "impetus CLI absent — install/list not run (AgentLoop inject also untested)"
  echo "PASS: test_hello_skill (packaging only)"
  exit 0
fi
require_impetus_or_skip

PROJECT="$(make_temp_project)"
info "installing skill into ${PROJECT}"
INSTALL_JSON="$("${IMPETUS}" extension install --kind skill "${LAYOUT}" --root "${PROJECT}" --json)"
echo "${INSTALL_JSON}" | "${PYTHON}" -c 'import json,sys; s=json.load(sys.stdin); assert s.get("installation_id"), s; print("installation_id=", s["installation_id"])'

SKILL_DEST="${PROJECT}/.impetus/skills/hello-extension/SKILL.md"
[[ -f "${SKILL_DEST}" ]] || fail "expected installed skill at ${SKILL_DEST}"

LIST_JSON="$("${IMPETUS}" extension list --root "${PROJECT}" --json)"
echo "${LIST_JSON}" | "${PYTHON}" - <<'PY'
import json, sys
rows = json.load(sys.stdin)
assert isinstance(rows, list) and rows, rows
ids = {r.get("module_id") or r.get("module_name") for r in rows}
assert any("hello" in (i or "") for i in ids), rows
print("OK: extension list includes hello-extension install")
PY

info "NOTE: skill AgentLoop inject not asserted (public API stop at install/list)."
echo "PASS: test_hello_skill"
