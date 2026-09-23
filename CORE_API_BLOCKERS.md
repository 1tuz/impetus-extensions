# Core API blockers

Only **current** blockers for the main [Impetus](https://github.com/1tuz/impetus) repository belong here.

## Shipped in Core (not blockers)

- public `impetus-extension-sdk` (git revision pin; `publish = false`)
- `impetus.extension_package.v1` / `extension.toml`
- entrypoints: `instruction_pack`, `mcp_bridge`, `host_process`
- extension API compatibility checks (`extension_api_version` major)
- package list/get/enable/disable/reload/operate IPC
- durable disabled-package state
- policy/permission gating at activation
- isolated package failures (crash of a package must not take down `impetusd`)
- public Browser/LSP host operations (`browser/*`, `coding/*`) when an Active `host_process` is present

Do not reintroduce local copies of private `impetus-core` host code.
Do not invent a public `native_module` / in-process ABI for third-party extensions.

---

## 1. No remote catalog/install/update/remove in Core

Current package discovery starts from local package directories. Core does not yet consume this repository's `catalog.json`, download GitHub release artifacts, or expose a complete remote package lifecycle.

Needed for one-click CLI/Desktop UX:

- refresh first-party catalog
- list available vs installed versions
- install / update / remove package
- verify manifest, compatibility and content digest before activation
- atomic install/rollback
- cache the last good catalog for offline use

This logic belongs in Core/daemon so CLI and Desktop share one implementation.

---

## 2. `mcp_bridge` packaging is not atomic yet

`mcp_bridge` activates an MCP module that already exists under the daemon MCP source of truth. A downloadable package such as Browser/LSP currently needs both:

1. its extension package under the extension package root; and
2. the corresponding MCP module config/binary made available to the daemon.

The future installer should install these artifacts as one transaction instead of requiring a manual copy step.

---

## 3. Extension SDK is not published to crates.io

`impetus-extension-sdk` is usable through an immutable Impetus git revision (see `compatibility.json` → `sdk.rev`), but is still `publish = false`.

Not a runtime blocker; publishing/tagging would simplify third-party authoring.

---

## 4. Generic host-process Tool/Command catalog wiring remains partial

Core can operate active `host_process` packages and has public Browser/LSP routes, but arbitrary extension Tool/Command capabilities are not yet a general AgentLoop tool catalog.

Do not block Browser/LSP packaging on this when MCP already provides the required tool surface.

---

## Repository notes (not Core blockers)

- Browser and LSP binaries currently speak MCP → keep as `mcp_bridge` until they intentionally adopt the Impetus host-process protocol.
- `impetus-ext-support package` / `install-local` generate legacy Skill/MCP envelopes from `extension.toml` for the old CLI only. Authoring SoT is always `extension.toml`.
- Do not create a second memory store here. Core MemoryStore remains authoritative.

## Non-goals

- copying `impetus-core` private modules into extensions
- loading untrusted native code in-process with `dlopen`
- putting Chromium/Playwright/language servers into the trusted Core
- making Rust the extension ABI
- implementing a second package manager in Desktop
- a public `native_module` entrypoint
