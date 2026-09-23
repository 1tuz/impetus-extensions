# Manifest

Canonical authoring manifest: **`extension.toml`**
(`impetus.extension_package.v1`, public `impetus-extension-sdk`).

There is no second plugin API and no competing package SoT under `extensions/`.

## Required fields

| Field | Rule |
|---|---|
| `schema_version` | `1` |
| `id` | lowercase Impetus extension id |
| `name` | human-readable name |
| `version` | semver |
| `description` | package description |
| `author` | package author |
| `extension_api_version` | API major; currently `1` |
| `capabilities` | non-empty closed set from the SDK |
| `permissions` | explicit, default-deny list (may be empty) |
| `entrypoint` | one of the three kinds below |

## Entrypoints

### Instruction pack

```toml
schema_version = 1
id = "example-skill"
name = "Example Skill"
version = "0.1.0"
description = "Portable instructions"
author = "Impetus Contributors"
extension_api_version = 1
capabilities = ["skill_provider"]
permissions = []

[entrypoint]
kind = "instruction_pack"
root = "skills"
```

`root` is package-relative and must not contain absolute paths or `..`.

### MCP bridge

```toml
schema_version = 1
id = "example-tools"
name = "Example Tools"
version = "0.1.0"
description = "Tools exposed through MCP"
author = "Impetus Contributors"
extension_api_version = 1
capabilities = ["mcp_integration", "tool"]
permissions = ["mcp"]

[entrypoint]
kind = "mcp_bridge"
module_id = "example-tools"
```

The MCP module must exist in the daemon MCP source of truth before activation.
Remote installation of config + binary is a Core package-manager task.

### Host process

```toml
schema_version = 1
id = "example-host"
name = "Example Host"
version = "0.1.0"
description = "Crash-isolated external integration"
author = "Impetus Contributors"
extension_api_version = 1
capabilities = ["tool"]
permissions = ["process_spawn"]

[entrypoint]
kind = "host_process"
command = "./bin/example-host"
args = ["--stdio"]
```

Child implements the public Impetus host JSON-RPC protocol. Implementation
language is irrelevant. **No** in-process `dlopen` ABI.

## Legacy envelopes (dist only)

`impetus-ext package` may emit under `dist/{id}-{version}/`:

- `extension.toml` (copy)
- generated `package.toml` + `manifest.json` (`impetus.extension.v1`) for the
  old Skill/MCP CLI install path

Those generated files are **not** an authoring source of truth. Do not commit
`package.toml` under `extensions/`.
