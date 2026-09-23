# impetus-ext-lsp

Generic **language-server** MCP extension for Impetus. Spawns a configurable LS over stdio (Content-Length framed JSON-RPC) and exposes MCP tools. Not Rust-only — `presets/rust-analyzer.json` is a reference config.

**Entrypoint:** `mcp_bridge` via `extension.toml`. **Compat:** extension API major `1` (SDK pin in repo `compatibility.json`).

## Install

```bash
cargo build -p impetus-ext-lsp --release
# put target/release/impetus-ext-lsp on PATH, then:
impetus extension install --kind mcp extensions/lsp/mcp.json --root /path/to/project
```

## Permissions

| Permission | Why |
| --- | --- |
| `process_spawn` | Spawn **only** the configured language server binary |
| `filesystem_read` | Workspace read for LS / diagnostics |
| `mcp` / `lsp` | MCP install + LSP tool surface |

**No** `network`, **no** `secrets_provider`. If a language server itself needs network (rare), that is outside this package’s declared permissions.

## Configuration

| Source | Keys |
| --- | --- |
| Env | `IMPETUS_LSP_COMMAND`, `IMPETUS_LSP_ARGS` (JSON array or whitespace), `IMPETUS_LSP_WORKSPACE` |
| CLI | `--command`, `--args`, `--workspace`, `--mock` |
| Tool | `lsp_start { command, args, workspace, mock? }` |

## Tools

- `lsp_start` / `lsp_stop` / `lsp_restart`
- `lsp_diagnostics` `{ uri? }`
- `lsp_request` `{ method, params }` — allowlisted methods only
- `lsp_hover` / `lsp_definition` / `lsp_references` / `lsp_symbols`
- `lsp_cancel` `{ id }`
- `lsp_status`

## Mock mode (CI)

No rust-analyzer required:

```bash
impetus-ext-lsp --mock
# then lsp_start with mock=true (or rely on --mock default)
```

`--mock` fakes `initialize` + sample `publishDiagnostics`.

## Preset: rust-analyzer

See [`presets/rust-analyzer.json`](presets/rust-analyzer.json).

```bash
export IMPETUS_LSP_COMMAND=rust-analyzer
export IMPETUS_LSP_WORKSPACE=/path/to/crate
impetus-ext-lsp
```

## Tests

```bash
cargo test -p impetus-ext-lsp
```
