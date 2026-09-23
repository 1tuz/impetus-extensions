# impetus-extensions

Official first-party extension registry and reference packages for [Impetus](https://github.com/1tuz/impetus).

This repository is intentionally **not a Rust-only plugin system**. Rust is used where an extension needs a native executable, but the extension contract is language-agnostic and follows the public Impetus package model.

| Repository | Responsibility |
|---|---|
| `impetus` | Runtime, daemon, policy, Extension Host/SDK, IPC, install authority |
| `impetus-extensions` | First-party extension packages, catalog metadata, release artifacts, examples |

----
## Package model

Prefer the most portable extension surface that fits the job:

| Entrypoint | Use for | Implementation |
|---|---|---|
| `instruction_pack` | Skills, instructions, reusable agent guidance | Declarative files such as `SKILL.md`; no Rust required |
| `mcp_bridge` | Tools already exposed through MCP | Any MCP server: Rust, Python, Node, Go, etc. |
| `host_process` | Browser/LSP/native integrations that need a dedicated process | Any executable implementing the Impetus host JSON-RPC protocol |

The canonical package manifest is `extension.toml` (`impetus.extension_package.v1`, extension API major `1`). Legacy `package.toml` / `impetus.extension.v1` files remain only for the old helper/install adapter until that path is retired.

The architectural rule is simple: **extensions plug into Impetus; they do not become part of the trusted kernel.** Policy, permissions, lifecycle authority and durable state remain daemon-owned.

See [docs/architecture.md](docs/architecture.md) and [docs/manifest.md](docs/manifest.md).

## Current first-party packages

| Id | Runtime surface | Implementation | Notes |
|---|---|---|---|
| `hello-extension` | `instruction_pack` | declarative | Portable tutorial/reference skill |
| `browser` | `mcp_bridge` today | Rust MCP server, optional CDP backend | Downloadable external browser capability; may move to `host_process` when its binary speaks the host protocol |
| `lsp` | `mcp_bridge` today | Rust MCP server | Generic language-server bridge; language servers remain external |
| `git-tools` | `mcp_bridge` | Rust MCP server | Read-only git tools |
| `code-search` | `mcp_bridge` | Rust MCP server | Workspace-scoped search |
| `http-fetch` | `mcp_bridge` | Rust MCP server | Constrained HTTP access |

Browser and LSP being written in Rust does **not** make Rust part of the extension ABI. They are ordinary out-of-core packages whose current portable surface is MCP.

## Catalog

[`catalog.json`](catalog.json) is the source-of-truth list of first-party packages for future CLI/Desktop discovery.

Current Impetus package discovery is still local. The Core does **not yet** fetch this catalog or install/update/remove packages from GitHub automatically. That missing Core work is tracked in [CORE_API_BLOCKERS.md](CORE_API_BLOCKERS.md).

The intended flow is:

```text
GitHub repository / release artifacts
              |
          catalog.json
              |
      Impetus Extension Manager
        /                 \
      CLI                Desktop
```

The catalog should update independently from installed package versions. Automatic catalog refresh is safe; package updates should remain explicit when versions or permissions change.

## Development

Rust is required only for the Rust-backed packages and helper crates:

```bash
cargo build --workspace
cargo test --workspace
```

----
----
For declarative `instruction_pack` packages, `extension.toml` plus the referenced skills directory is enough at runtime.

The existing `impetus-ext-support` helper still targets the legacy `package.toml` packaging path. Treat it as a compatibility tool, not the canonical authoring API. New packages should start from `extension.toml` and the public `impetus-extension-sdk` contract in the main Impetus repository.

## Docs

- [Architecture and extension classes](docs/architecture.md)
- [Catalog contract](docs/catalog.md)
- [Creating an extension](docs/creating-extension.md)
- [Manifest](docs/manifest.md)
- [Packaging](docs/packaging.md)
- [Permissions](docs/permissions.md)
- [Versioning](docs/versioning.md)
- [Testing](docs/testing.md)
- [Core API blockers](CORE_API_BLOCKERS.md)
----
- [Contributing](CONTRIBUTING.md) - [Security](SECURITY.md)

## Compatibility

Canonical new packages target:

- manifest schema: `impetus.extension_package.v1`
- extension API major: `1`
- entrypoints: `instruction_pack`, `mcp_bridge`, `host_process`
- SDK: `impetus-extension-sdk` from an immutable Impetus git revision until it is published

----
`compatibility.json` keeps the old `v0.1.2` fields only because the legacy helper still reads them. Do not use those fields as the new package contract.

## License

Apache License 2.0 - same as Impetus. See [LICENSE](LICENSE).
