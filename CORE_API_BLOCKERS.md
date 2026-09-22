# Core API blockers

Work items for the main [Impetus](https://github.com/1tuz/impetus) repository.
This extensions repo uses **only** public install kinds (`skill`, `mcp_config`) and documented wire shapes.
It does **not** copy private host code or patch the daemon.

Compatibility pin: Impetus git tag **`v0.1.2`**.

---

## 1. No published Extension SDK / `extension_api_version`

| | |
|--|--|
| **Required API** | Stable crate (e.g. `impetus-extension-sdk`) + version constant `extension_api_version` negotiated at load time |
| **Why** | External authors cannot depend on a crates.io/git SDK surface; only path/`impetus-core` exists and is not a plugin ABI |
| **Minimal interface** | `EXTENSION_API_VERSION: &str`, `negotiate(client) -> Result<Compat>`, documented semver policy |
| **Blocked / degraded** | All native providers; this repo uses stand-in `extension_api_version = "0.1.0-skill-mcp"` in `package.toml` / `compatibility.json` |

---

## 2. Install kinds limited to Skill + MCP config

| | |
|--|--|
| **Required API** | Additional `ExtensionManifestKind` values (e.g. `native_module`, `browser_provider`, `coding_tools_provider`) with package install path |
| **Why** | Browser/LSP implementations cannot be installed via `impetus extension install` as first-class providers |
| **Minimal interface** | Kind + digest + capabilities + entrypoint path; lifecycle enable/disable identical to skill/mcp |
| **Blocked / degraded** | `browser`, `lsp` ship as **MCP stand-ins**, not in-process provider registration |

---

## 3. No dynamic `BrowserProvider` registration

| | |
|--|--|
| **Required API** | Load external provider (module IPC or dynamic library) implementing protocol `0.1` without recompiling `impetusd` |
| **Why** | Core trait is in-process only; default production = absent provider |
| **Minimal interface** | `BrowserService::register(Box<dyn BrowserProvider>)` from discovered module descriptor OR MCP→provider bridge in core |
| **Blocked / degraded** | Official browser backend cannot plug into `OptionalBrowserService` / doctor browser probe without a core PR |

---

## 4. No dynamic `CodingToolsProvider` / LSP registration

| | |
|--|--|
| **Required API** | Same as browser: discover + register external coding tools / LSP backend |
| **Why** | `ProcessLspBackend` lives in core host wiring |
| **Minimal interface** | Register `CodingToolsProvider` from package entrypoint |
| **Blocked / degraded** | `lsp` extension is MCP proxy, not core coding-tools seam |

---

## 5. No `MemoryProvider` trait

| | |
|--|--|
| **Required API** | `MemoryProvider` with store/recall/list scoped entries; must **not** auto-grant sandbox effects |
| **Why** | Memory is daemon-owned (`MemoryStore` / session JSONL); no pluggable backend |
| **Minimal interface** | `async fn remember/recall/list`; config for persistence root; capability token `memory.provider` |
| **Blocked** | First-party memory extension **not shipped** (would duplicate session storage or require private hooks) |

---

## 6. ExtensionRuntime → AgentLoop skill inject incomplete

| | |
|--|--|
| **Required API** | Enabled skills from ExtensionRuntime appear in AgentLoop instruction path |
| **Why** | Install/list/doctor work; agent may not see skill content yet (Impetus TODO) |
| **Minimal interface** | On session start, merge enabled skill bodies from install store into instruction resolver |
| **Blocked / degraded** | `hello-extension` install path works; end-to-end “agent uses skill” may skip until core closes TODO |

---

## 7. Dual MCP SoT (project `.impetus/mcp` vs `$IMPETUS_DATA_DIR/mcp`)

| | |
|--|--|
| **Required API** | Single documented SoT or sync API CLI→daemon |
| **Why** | `impetus extension install --kind mcp` writes project tree; daemon tools read data dir |
| **Minimal interface** | `impetus extension publish-mcp --to-daemon` or daemon watches project install store |
| **Blocked / degraded** | Dev loop needs `--daemon-mcp` copy step (`impetus-ext install-local`) |

---

## Non-goals (do not “fix” in this repo)

- Copying `impetus-core` private modules into extensions
- Undocumented daemon IPC mutate for install
- Requiring Chromium/Playwright/Node inside Impetus core
- Shipping a fake memory extension that reimplements session event storage
