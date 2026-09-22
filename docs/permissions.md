# Permissions

Each extension declares the **minimum** permissions it needs in `package.toml`.

| Extension | Permissions | Rationale |
|-----------|-------------|-----------|
| hello-extension | none | Skill text only |
| browser | network | http(s) navigation/fetch; no FS/secrets/shell in mock |
| lsp | process.spawn, filesystem.read | LS binary + workspace; no secrets/network grant |
| git-tools | process | git argv allowlist only |
| code-search | filesystem.read | workspace sandbox |
| http-fetch | network | http(s); localhost blocked by default |

Browser must **not** auto-receive filesystem/process/secrets.

MCP `env` must never store secret **values** — keys only when required.
