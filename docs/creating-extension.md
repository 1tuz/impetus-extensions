# Creating an extension

## Choose a kind

Public Impetus install kinds (`v0.1.2`):

| Kind | Artifact | Install |
|------|----------|---------|
| `skill` | `SKILL.md` | `impetus extension install --kind skill <dir>` |
| `mcp_config` | `mcp.json` + binary on `PATH` | `impetus extension install --kind mcp <file>` |

Native `BrowserProvider` / `CodingToolsProvider` / `MemoryProvider` packages are **not** installable yet — see `CORE_API_BLOCKERS.md`.

## Layout

```
extensions/my-ext/
  package.toml      # ecosystem metadata (CI / packaging; NOT the core envelope)
  README.md
  SKILL.md          # if skill
  mcp.json          # if MCP
  src/ …            # if MCP binary
  tests/
```

## package.toml

Must pin:

- `compatibility.impetus_tag = "v0.1.2"`
- `compatibility.extension_schema = "impetus.extension.v1@1"`
- `compatibility.mcp_schema = "impetus.mcp.v1@1"`
- `compatibility.extension_api_version = "0.1.0-skill-mcp"` (stand-in)

Declare **minimal** `permissions` + `permission_rationale`.

## Core manifest

Generated at package time (`impetus-ext package`). Fields only:

`schema_version`, `id`, `kind`, `version`, `digest`, `capabilities`

Undocumented fields are rejected.

## Dev loop

See [development-loop.md](development-loop.md).
