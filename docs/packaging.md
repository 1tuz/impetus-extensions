# Packaging

Three layers — do not confuse them:

1. **package source** — `extension.toml`, skills/config, optional source;
2. **runtime artifact** — files under the Impetus extension package root;
3. **publisher artifact** — GitHub release archive referenced by the catalog once remote install exists.

## Portable instruction package

```text
my-skill/
  extension.toml
  skills/
    SKILL.md
```

No Rust toolchain required at runtime.

## MCP-backed package

```text
my-tools/
  extension.toml
  mcp.json
  [bin/my-tools]
```

## Host-process package

```text
my-host/
  extension.toml
  bin/
    my-host
```

Executable speaks Impetus host JSON-RPC over stdio. Out of process; outside the trusted kernel.

## Browser and LSP

Rust binaries today; **ABI is not Rust**. Until they implement the host-process protocol they remain valid `mcp_bridge` packages.

## Legacy helper output (compatibility only)

`impetus-ext package` emits a Skill/MCP CLI layout derived from `extension.toml`:

```text
dist/{id}-{version}/
  extension.toml
  package.toml          # GENERATED — do not author this
  manifest.json         # impetus.extension.v1
  README.md
  SKILL.md | mcp.json
  [binary]
```

Do **not** commit `package.toml` under `extensions/`. New metadata lives only in `extension.toml`.

Never package `target/`, `.git/`, credentials or raw secrets.
