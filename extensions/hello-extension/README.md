# hello-extension

Canonical **skill** tutorial for [impetus-extensions](https://github.com/1tuz/impetus-extensions).
Public Impetus install kind: `skill`. No Rust binary.

Teaches: core manifest envelope, initialize, capability registration (`instructions` / `triggers`), empty permissions, no configuration, and packaging tests.

## Layout

| File | Role |
|------|------|
| `package.toml` | Ecosystem packaging metadata (`PackageMeta`) |
| `SKILL.md` | Installable Agent Skills artifact |
| `tests/smoke.sh` | Frontmatter + package.toml checks (no `impetus` CLI) |

## Install

### With Impetus CLI

```bash
impetus extension install --kind skill /path/to/impetus-extensions/extensions/hello-extension --root /path/to/project
```

### With impetus-ext (this repo)

```bash
cargo run -p impetus-ext-support --bin impetus-ext -- package extensions/hello-extension
cargo run -p impetus-ext-support --bin impetus-ext -- install-local dist/hello-extension-0.1.0 --root /path/to/project
```

Packaging emits `dist/hello-extension-0.1.0/` with `SKILL.md`, `package.toml`, and validated `manifest.json` (`impetus.extension.v1@1`).

## Compatibility

Pinned in `package.toml`:

- `extension_api_version = "0.1.0-skill-mcp"`
- `impetus_tag = "v0.1.2"`
- `extension_schema = "impetus.extension.v1@1"`
- `mcp_schema = "impetus.mcp.v1@1"`

## Tests (CI-friendly, no impetus binary)

```bash
bash extensions/hello-extension/tests/smoke.sh
cargo test -p impetus-ext-support package_hello_extension_from_repo
```

Mirror / tutorial pointer: [`examples/hello-extension/`](../../examples/hello-extension/).
