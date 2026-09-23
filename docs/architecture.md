# Extension architecture

`impetus-extensions` is a mixed first-party registry, not a Rust plugin framework.

```text
                         Impetus Core / impetusd
                   policy + lifecycle + permissions
                                |
                     public extension contract
                                |
          +---------------------+---------------------+
          |                     |                     |
          v                     v                     v
   instruction_pack         mcp_bridge            host_process
     SKILL.md              MCP server             JSON-RPC child
          |                     |                     |
    declarative          any language            any language
```

## Design rules

1. **Use portable primitives first.** Skills and MCP should be preferred when they cover the capability.
2. **Rust is an implementation detail.** A package may contain a Rust binary, Python program, Node server, Go executable, or no executable at all.
3. **No in-process native ABI.** Native integrations run as external processes; a crash must not take down `impetusd`.
4. **Core owns authority.** Extensions request permissions; Core decides. Extensions never own Policy, Approval, Sandbox, EventStore, secrets or authoritative session state.
5. **Do not emulate every foreign plugin API.** Reuse portable Skills/MCP content and let Core import adapters handle supported Codex/Cursor/Claude layouts.
6. **One package manager.** Installation/update/removal belongs in Core so CLI and Desktop see the same catalog and installed state.

## Browser

Browser stays outside Core. The repository contains the implementation and, later, prebuilt release artifacts.

Current path:

```text
Impetus -> mcp_bridge -> impetus-ext-browser -> optional CDP/Chromium
```

Possible future path after the binary implements the public host protocol:

```text
Impetus -> host_process -> impetus-ext-browser -> CDP/Chromium
```

Both preserve the same architectural rule: Chromium/browser automation is an extension, not trusted-kernel code.

## LSP

LSP is also an external bridge, not a language engine embedded into Core:

```text
Impetus -> mcp_bridge/host_process -> impetus-ext-lsp -> language server
                                                   -> rust-analyzer
                                                   -> gopls
                                                   -> pyright
                                                   -> ...
```

The language server remains replaceable and project-specific.

## Registry vs package format

The repository may contain Rust crates because some first-party packages need compiled binaries. That does not define the package format.

`catalog.json` answers **what can be installed**. `extension.toml` answers **how Impetus activates one installed package**. A `SKILL.md`, `mcp.json`, or executable answers **what the package actually does**.

Do not commit legacy `package.toml` under `extensions/` — that envelope is generated only under `dist/` for the old Skill/MCP CLI.
