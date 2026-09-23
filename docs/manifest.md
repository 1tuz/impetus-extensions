# Manifest

The canonical authoring manifest is `extension.toml` using the public `impetus.extension_package.v1` contract from `impetus-extension-sdk`.

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
| `permissions` | explicit, default-deny permission list |
| `entrypoint` | one typed entrypoint |

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

The root is package-relative and must not contain absolute paths or `..`.

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

The MCP module must exist in the daemon MCP source of truth before activation. Remote installation of an MCP config plus its binary is a Core package-manager task, not something the extension should bypass.

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

The child must implement the public Impetus host JSON-RPC protocol. The implementation language is irrelevant.

## Legacy manifests

`package.toml` plus generated `manifest.json` (`impetus.extension.v1`) belong to the old Skill/MCP CLI install adapter. They remain in this repository only while `impetus-ext-support` depends on them.

Do not use the legacy envelope as the design for new packages.
