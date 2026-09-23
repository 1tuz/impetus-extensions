# hello-extension

Canonical **`instruction_pack`** tutorial for Impetus extensions. No Rust binary.

## Layout

| Path | Role |
|------|------|
| `extension.toml` | Canonical package manifest |
| `skills/SKILL.md` | Instruction pack root |
| `tests/smoke.sh` | Frontmatter + manifest checks (no `impetus` CLI) |

## Install (Extension Host)

Copy this directory under `$IMPETUS_DATA_DIR/extensions/packages/` and reload packages via Core IPC.

## Legacy Skill CLI (compatibility)

```bash
cargo run -p impetus-ext-support -- package extensions/hello-extension
cargo run -p impetus-ext-support -- install-local dist/hello-extension-0.1.0 --root /path/to/project
```

## Compatibility

- `extension_api_version = 1`
- SDK pin: repo `compatibility.json` → `sdk.rev`

## Tests

```bash
bash extensions/hello-extension/tests/smoke.sh
cargo test -p impetus-ext-support package_hello_extension_from_repo
```

Examples: [`examples/hello-instruction-pack/`](../../examples/hello-instruction-pack/).
