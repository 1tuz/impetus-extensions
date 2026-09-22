#!/usr/bin/env bash
# Shared helpers for impetus-extensions integration tests.
# shellcheck shell=bash

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${INTEGRATION_DIR}/../.." && pwd)"
EXPECTED_IMPETUS_TAG="${IMPETUS_TAG:-v0.1.2}"

skip() {
  echo "SKIP: $*"
  exit 0
}

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

info() {
  echo "INFO: $*"
}

resolve_impetus_bin() {
  if [[ -n "${IMPETUS_BIN:-}" ]]; then
    if [[ -x "${IMPETUS_BIN}" ]]; then
      echo "${IMPETUS_BIN}"
      return 0
    fi
    fail "IMPETUS_BIN=${IMPETUS_BIN} is not executable"
  fi
  if command -v impetus >/dev/null 2>&1; then
    command -v impetus
    return 0
  fi
  return 1
}

require_impetus_or_skip() {
  local bin
  if ! bin="$(resolve_impetus_bin)"; then
    skip "impetus CLI not found (set IMPETUS_BIN or install on PATH). Public install/list/enable/disable path not exercised."
  fi
  IMPETUS="${bin}"
  export IMPETUS
  info "using impetus: ${IMPETUS}"
}

resolve_impetusd_bin() {
  if [[ -n "${IMPETUSD_BIN:-}" ]]; then
    if [[ -x "${IMPETUSD_BIN}" ]]; then
      echo "${IMPETUSD_BIN}"
      return 0
    fi
    fail "IMPETUSD_BIN=${IMPETUSD_BIN} is not executable"
  fi
  if command -v impetusd >/dev/null 2>&1; then
    command -v impetusd
    return 0
  fi
  return 1
}

ensure_python() {
  if command -v python3 >/dev/null 2>&1; then
    PYTHON=python3
  elif command -v python >/dev/null 2>&1; then
    PYTHON=python
  else
    fail "python3 required for MCP stdio JSON-RPC probe"
  fi
  export PYTHON
}

package_extension() {
  local ext_dir="$1"
  local out_dir="${2:-${REPO_ROOT}/dist}"
  mkdir -p "${out_dir}"
  cargo run -q -p impetus-ext-support -- package "${ext_dir}" --out "${out_dir}"
}

build_bin() {
  local package="$1"
  local bin="$2"
  cargo build -q -p "${package}" --bin "${bin}"
  local candidate="${REPO_ROOT}/target/debug/${bin}"
  if [[ ! -x "${candidate}" ]]; then
    fail "expected binary missing after build: ${candidate}"
  fi
  echo "${candidate}"
}

assert_compat_tag() {
  local file="${REPO_ROOT}/compatibility.json"
  [[ -f "${file}" ]] || fail "missing ${file}"
  local tag
  tag="$("${PYTHON:-python3}" -c 'import json,sys; print(json.load(open(sys.argv[1]))["impetus_tag"])' "${file}")"
  [[ "${tag}" == "${EXPECTED_IMPETUS_TAG}" ]] \
    || fail "compatibility.json impetus_tag=${tag}, expected ${EXPECTED_IMPETUS_TAG}"
}

make_temp_project() {
  local dir
  dir="$(mktemp -d "${TMPDIR:-/tmp}/impetus-ext-int.XXXXXX")"
  mkdir -p "${dir}"
  echo "${dir}"
}
