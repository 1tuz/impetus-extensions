# impetus-extensions

Official first-party extensions for [Impetus](https://github.com/1tuz/impetus).

| Repo | Role |
|------|------|
| **impetus** | Core / runtime / daemon / CLI |
| **impetus-extensions** | First-party Skill + MCP packs that install through public Impetus APIs |

Pin: Impetus git tag **`v0.1.2`** (see `compatibility.json`). License: **Apache-2.0**.

Public install kinds today: **`skill`** and **`mcp_config`** only (`impetus.extension.v1`). There is no published Extension SDK / `extension_api_version` yet — see [CORE_API_BLOCKERS.md](CORE_API_BLOCKERS.md).

---

## Quick start

```bash
git clone https://github.com/1tuz/impetus-extensions.git
cd impetus-extensions

# Rust 1.98 (rust-toolchain.toml)
cargo build --workspace
cargo test --workspace

# Dev helper CLI
cargo run -p impetus-ext-support -- validate-manifests
cargo run -p impetus-ext-support -- check-compat
cargo run -p impetus-ext-support -- package-all

# Install into a local Impetus project (requires `impetus` on PATH, built from tag v0.1.2)
cargo run -p impetus-ext-support -- install-local extensions/hello-extension --root /path/to/project

# MCP packs: also copy into daemon SoT when you use impetusd tools
cargo run -p impetus-ext-support -- install-local extensions/git-tools --root /path/to/project --daemon-mcp
```

Impetus CLI (same project root):

```bash
impetus extension list --root /path/to/project
impetus extension doctor --root /path/to/project
```

---

## Extensions

| Id | Kind | Status | Notes |
|----|------|--------|-------|
| `hello-extension` | skill | tutorial | Canonical Skill pack |
| `browser` | mcp_config | MCP stand-in | Browser automation over MCP; **not** in-process `BrowserProvider` |
| `lsp` | mcp_config | MCP stand-in | Generic LSP proxy; **not** `CodingToolsProvider` |
| `git-tools` | mcp_config | MCP | Read-only git tools |
| `code-search` | mcp_config | MCP | Workspace-sandboxed search |
| `http-fetch` | mcp_config | MCP | Constrained HTTP fetch |

**Memory:** blocked as a provider API. This repo does **not** ship a fake memory extension that duplicates session storage. See [CORE_API_BLOCKERS.md](CORE_API_BLOCKERS.md).

---

## Docs

- [Creating an extension](docs/creating-extension.md)
- [Manifest](docs/manifest.md) (`impetus.extension.v1` vs `package.toml`)
- [Packaging](docs/packaging.md)
- [Permissions](docs/permissions.md)
- [Versioning](docs/versioning.md)
- [Testing](docs/testing.md)
- [Development loop](docs/development-loop.md) (8-step UX)
- [Core API blockers](CORE_API_BLOCKERS.md)
- [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)
- Suggested core README blurb: [docs/LINK_FROM_IMPETUS.md](docs/LINK_FROM_IMPETUS.md)

---

## Compatibility

```json
"impetus_tag": "v0.1.2",
"extension_api_version": "0.1.0-skill-mcp"   // documented stand-in until core ships a real field
```

Schemas: `impetus.extension.v1@1`, `impetus.mcp.v1@1`, browser protocol `0.1`.

---

## License

Apache License 2.0 — same as Impetus. See [LICENSE](LICENSE).
