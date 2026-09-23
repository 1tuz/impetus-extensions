# Core API blockers

Only current blockers for the main [Impetus](https://github.com/1tuz/impetus) repository belong here. Historical `v0.1.2` blockers were removed after Core shipped the public Extension SDK/package host.

## Shipped in Core

These are no longer blockers:

- public `impetus-extension-sdk` surface (git revision pin; not crates.io yet)
- `impetus.extension_package.v1` / `extension.toml`
- `instruction_pack`
- `mcp_bridge`
- `host_process`
- extension API compatibility checks
- package list/get/enable/disable/reload/operate IPC
- durable disabled-package state
- policy/permission gating at activation
- isolated package failures
- public Browser/LSP host operations (`browser/*`, `coding/*`)

Do not reintroduce local copies of private `impetus-core` host code to solve extension problems.

---
----
----
----

## 1. No remote catalog/install/update/remove in Core

Current package discovery starts from local package directories. Core does not yet consume this repository's `catalog.json`, download GitHub release artifacts, or expose a complete remote package lifecycle.

Needed for one-click CLI/Desktop UX:

- refresh first-party catalog
- list available vs installed versions
- install a selected package
- update one / update all
- remove package
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

`impetus-extension-sdk` is usable through an immutable Impetus git revision, but is still `publish = false`.

This is not a runtime blocker, but publishing/tagging the SDK would simplify third-party authoring and reproducible builds.

---

## 4. Generic host-process Tool/Command catalog wiring remains partial

Core can operate active `host_process` packages and has public Browser/LSP routes, but arbitrary extension Tool/Command capabilities are not yet a general AgentLoop tool catalog.

Do not block Browser/LSP packaging on this when MCP already provides the required tool surface.

---

## Repository migration work (not Core blockers)

- Browser and LSP binaries currently speak MCP. Keep them as `mcp_bridge` packages until they intentionally adopt the Impetus host-process protocol.
- Legacy `package.toml`, `manifest.json` generation, and `impetus-ext-support install-local` remain compatibility tooling and should be removed only after the new Core installer replaces them.
- Do not create a second memory store in this repository. Core MemoryStore remains authoritative.

## Non-goals

- copying `impetus-core` private modules into extensions
- loading untrusted native code in-process with `dlopen`
- putting Chromium/Playwright/language servers into the trusted Core
- making Rust the extension ABI
- implementing a second package manager in Desktop
