# Contributing

This repo ships first-party Impetus extensions. Keep changes narrow and honest about what core APIs support today.

## Rules

1. Use only public install kinds: `skill` | `mcp_config` (`impetus.extension.v1`).
2. Pin Impetus to git tag **`v0.1.2`** (`compatibility.json` + each `package.toml`).
3. Do not depend on Impetus private modules or copy core internals here.
4. Prefer MCP stdio binary + optional companion Skill over pretending native providers exist.
5. Every gap that needs core wiring → document in [CORE_API_BLOCKERS.md](CORE_API_BLOCKERS.md), do not fake it.
6. No sudo/root. Userspace only (`$HOME` / `$IMPETUS_DATA_DIR`).
7. Least privilege: declare only needed permissions + rationale in `package.toml`.
8. Do **not** add a Memory extension that duplicates daemon session storage.

## Local checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p impetus-ext-support -- validate-manifests
cargo run -p impetus-ext-support -- check-compat
```

Integration against a real `impetusd` (when available): see [docs/testing.md](docs/testing.md).

## Pull requests

- Explain user-visible behavior and which install path it uses.
- Note any new core API dependency or blocker update.
- Keep unrelated formatting out of the diff.

## Security

Report privately — [SECURITY.md](SECURITY.md).
