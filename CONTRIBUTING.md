# Contributing

This repo ships first-party Impetus extensions. Keep changes narrow.

## Rules

1. Canonical manifest is **`extension.toml`** (`impetus.extension_package.v1`).
2. Only three entrypoints: `instruction_pack` | `mcp_bridge` | `host_process`.
3. Pin `impetus-extension-sdk` via immutable git `rev` (`compatibility.json` → `sdk.rev`).
4. Do not depend on Impetus private modules or copy core internals here.
5. Do not add `package.toml` under `extensions/` (legacy envelopes are generated into `dist/` only).
6. No public `native_module` / in-process native ABI for third-party packs.
7. Least privilege: declare only needed SDK permission tokens in `extension.toml`.
8. Do **not** add a Memory extension that duplicates daemon session storage.
9. Every gap that needs core wiring → [CORE_API_BLOCKERS.md](CORE_API_BLOCKERS.md).

## Local checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p impetus-ext-support -- validate-manifests --root .
cargo run -p impetus-ext-support -- validate-catalog --root .
cargo run -p impetus-ext-support -- check-compat --root .
```

## Pull requests

- Explain user-visible behavior and which entrypoint it uses.
- Note any new core API dependency or blocker update.
- Keep unrelated formatting out of the diff.

## Security

Report privately — [SECURITY.md](SECURITY.md).
