# impetus-extensions

Official first-party extension registry and reference packages for [Impetus](https://github.com/1tuz/impetus).

This repository is **not** a Rust-only plugin system. Rust is used where an extension needs a native executable; the contract is language-agnostic.

| Repository | Responsibility |
|---|---|
| `impetus` | Runtime, daemon, policy, Extension Host/SDK, IPC, install authority |
| `impetus-extensions` | First-party packages, catalog metadata, release artifacts, examples |

## Package model

| Entrypoint | Use for | Implementation |
|---|---|---|
| `instruction_pack` | Skills, instructions, reusable agent guidance | Declarative files such as `SKILL.md`; no Rust required |
| `mcp_bridge` | Tools already exposed through MCP | Any MCP server: Rust, Python, Node, Go, etc. |
| `host_process` | Isolated native/heavy integrations | Any executable implementing the Impetus host JSON-RPC protocol |

**Canonical manifest:** `extension.toml` (`impetus.extension_package.v1`, extension API major `1`).

There is no second plugin API and no public in-process `native_module` ABI. Heavy work runs out-of-process so a crashing extension cannot take down `impetusd`.

Authoring flow: [docs/creating-extension.md](docs/creating-extension.md).

```text
1. mkdir extensions/my-ext
2. write extension.toml
3. implement the chosen entrypoint
4. cargo run -p impetus-ext-support -- validate-manifests --root .
5. sync / add catalog.json entry
```

## Current first-party packages

| Id | Entrypoint | Implementation | Notes |
|---|---|---|---|
| `hello-extension` | `instruction_pack` | declarative | Tutorial / reference skill |
| `browser` | `mcp_bridge` | Rust MCP server | Optional CDP backend; may move to `host_process` later |
| `lsp` | `mcp_bridge` | Rust MCP server | Language servers remain external |
| `git-tools` | `mcp_bridge` | Rust MCP server | Read-only git tools |
| `code-search` | `mcp_bridge` | Rust MCP server | Workspace-scoped search |
| `http-fetch` | `mcp_bridge` | Rust MCP server | Constrained HTTP access |

## Catalog

[`catalog.json`](catalog.json) is the discovery list for a future Core Extension Manager. Core does **not yet** fetch or install from it automatically — see [CORE_API_BLOCKERS.md](CORE_API_BLOCKERS.md).

CI validates catalog fields against each `extension.toml` (no drift).

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo run -p impetus-ext-support -- validate-manifests --root .
cargo run -p impetus-ext-support -- check-compat --root .
```

Declarative packs need only `extension.toml` + skills. Rust is required for MCP/host binaries and helper crates.

SDK: `impetus-extension-sdk` pinned in [compatibility.json](compatibility.json) (`sdk.rev`). Not on crates.io yet.

Legacy Skill/MCP CLI packaging (`impetus-ext package` / `install-local`) **generates** envelopes under `dist/` from `extension.toml`. Do not commit `package.toml` under `extensions/`.

## Docs

- [Architecture](docs/architecture.md)
- [Creating an extension](docs/creating-extension.md)
- [Manifest](docs/manifest.md)
- [Catalog](docs/catalog.md)
- [Packaging](docs/packaging.md)
- [Permissions](docs/permissions.md)
- [Versioning](docs/versioning.md)
- [Testing](docs/testing.md)
- [Core API blockers](CORE_API_BLOCKERS.md)
- [TODO](TODO.md)
- [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## Compatibility

- schema: `impetus.extension_package.v1`
- extension API major: `1`
- entrypoints: `instruction_pack`, `mcp_bridge`, `host_process`
- SDK: git pin in `compatibility.json`

## License

Apache License 2.0 — same as Impetus. See [LICENSE](LICENSE).
