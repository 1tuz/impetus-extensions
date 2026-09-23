# Code Search (`impetus-ext-code-search`)

Workspace-sandboxed text search over MCP. Tool: `search_text`.

## Security

- Requires `IMPETUS_WORKSPACE_ROOT` (absolute).
- All paths are resolved with `canonicalize` and must `starts_with` the workspace root (blocks `..` and symlink escape).
- Prefers `rg` when available; otherwise walks files with a Rust `regex` fallback.
- Never invokes a shell with user strings.

## Permissions

`filesystem_read` — read-only access under the workspace root (also declares `mcp`, `process_spawn` in `extension.toml`).

## Build / run

```bash
cargo build -p impetus-ext-code-search --release
IMPETUS_WORKSPACE_ROOT=/path/to/repo ./target/release/impetus-ext-code-search
```

## Install

```bash
impetus extension install --kind mcp extensions/code-search/mcp.json --root /path/to/project
```

Env values in `mcp.json` are empty labels only — no secrets.
