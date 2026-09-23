# Packaging

The repository contains three different things and they should not be confused:

1. **package source** - `extension.toml`, skills/config and optional source code;
2. **runtime artifact** - files installed under the Impetus extension package root;
3. **publisher artifact** - a GitHub release archive referenced by the catalog once remote install exists.

## Portable instruction package

```text
my-skill/
  extension.toml
  skills/
    SKILL.md
```

No Rust toolchain is required at runtime.

## MCP-backed package

```text
my-tools/
  extension.toml
  mcp.json
  [bin/my-tools]
```

The server may be written in any language. A future Core installer should install the MCP config and packaged binary atomically, then activate the `mcp_bridge` package.

## Host-process package

```text
my-host/
  extension.toml
  bin/
    my-host
```

The executable speaks the Impetus host JSON-RPC protocol over stdio. It stays out of process and outside the trusted kernel.

## Browser and LSP

Browser and LSP are Rust programs today because Rust is a good implementation choice for those binaries. Their **extension ABI is not Rust**. Until the binaries implement the host-process protocol, they remain valid MCP-backed packages.

When release packaging is added, publish prebuilt target-specific archives instead of requiring end users to compile Rust source.

## Legacy helper output

`impetus-ext-support package` still emits the older layout:

```text
dist/{id}-{version}/
  manifest.json
  package.toml
  README.md
  SKILL.md | mcp.json
  [binary]
```

Keep this only for compatibility with the legacy CLI install path. New package metadata lives in `extension.toml`.

Never package `target/`, `.git/`, credentials or raw secrets.
