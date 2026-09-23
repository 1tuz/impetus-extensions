# Impetus Browser extension

First-party **`mcp_bridge`** package (see `extension.toml`) implementing
**Browser Provider Protocol `0.1`**
(see Impetus `docs/reference/browser-provider-protocol.md`).

## Quick start

```bash
# CI / local without Chrome (default)
cargo run -p impetus-ext-browser -- --mock

# Or after install script rewrites mcp.json command to an absolute binary path
impetus-ext-browser --mock
```

Optional real Chromium path (feature-gated, allowlisted binary only):

```bash
cargo build -p impetus-ext-browser --features cdp --release
BROWSER_BIN=/path/to/chromium impetus-ext-browser --cdp
```

`BROWSER_BIN` is the only spawn allowlist. Declared as `process_spawn` in
`extension.toml`; MCP `env` / `env_keys` stay minimal — do not pass secrets.

## Permissions

| Permission | Why |
| --- | --- |
| `mcp` | MCP install surface |
| `network` | Navigation / rendered fetch |
| `process_spawn` | Optional allowlisted Chromium via `BROWSER_BIN` |
| `browser` | Browser tool surface |

**Not requested:** `filesystem_write`, `secrets_provider`, arbitrary shell.

## MCP tools

| Tool | Role |
| --- | --- |
| `browser_status` | Health: unavailable / degraded / misconfigured / available |
| `browser_negotiate` | Capability intersection (protocol major `0.1`) |
| `browser_ensure_session` | Create/reuse opaque session |
| `browser_close_session` | Close session + cancel token |
| `browser_navigate` | Session-scoped URL open |
| `browser_page_state` | url / title / ready |
| `browser_fetch_rendered` | Rendered text (mock → fixture HTML) |
| `browser_click` / `browser_type` / `browser_wait` | Best-effort actions (mock no-op success) |
| `browser_cancel` | Cancel in-flight / close session(s) |

## Library

Crate `impetus-ext-browser` also exposes serde wire shapes under `protocol`
(`BrowserCapability`, negotiate/status/session/navigate/fetch/actions) **without**
depending on `impetus-core`.

## Tests

```bash
cargo test -p impetus-ext-browser
```

Mock backend only — no Chrome required.
