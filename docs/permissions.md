# Permissions

Each extension declares the **minimum** permissions it needs in `extension.toml`.
Vocabulary is the closed SDK set (`impetus-extension-sdk::ExtensionPermission`).

There is **one** vocabulary — snake_case tokens. Do not use dotted aliases
(`filesystem.read`, `process.spawn`) or invent new strings.

| Extension | Permissions | Rationale |
|-----------|-------------|-----------|
| hello-extension | _(none)_ | Instruction text only |
| browser | `mcp`, `network`, `process_spawn`, `browser` | MCP server + http(s) + optional Chromium launch |
| lsp | `mcp`, `process_spawn`, `filesystem_read`, `lsp` | LS binary + workspace reads |
| git-tools | `mcp`, `process_spawn`, `git` | git argv allowlist only |
| code-search | `mcp`, `process_spawn`, `filesystem_read` | workspace sandbox |
| http-fetch | `mcp`, `process_spawn`, `network` | http(s); localhost blocked by default |

Browser must **not** auto-receive `filesystem_write` / `secrets_provider`.

MCP `env` must never store secret **values** — keys only when required.

Extensions request; Core decides (Policy → Approval → Sandbox). Declaring a
permission does not bypass daemon gates.
