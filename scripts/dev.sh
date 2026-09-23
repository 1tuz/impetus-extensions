#!/usr/bin/env bash
# Developer one-shot: build, test, validate manifests/catalog, package all extensions.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> cargo build --workspace"
cargo build --workspace --all-targets

echo "==> cargo test --workspace"
cargo test --workspace --all-targets

echo "==> validate-manifests"
cargo run -p impetus-ext-support -- validate-manifests --root "$ROOT"

echo "==> validate-catalog"
cargo run -p impetus-ext-support -- validate-catalog --root "$ROOT"

echo "==> check-compat"
cargo run -p impetus-ext-support -- check-compat --root "$ROOT"

echo "==> package-all -> dist/"
cargo run -p impetus-ext-support -- package-all --root "$ROOT" --out "$ROOT/dist"

echo "OK: build, test, validate, package complete."
