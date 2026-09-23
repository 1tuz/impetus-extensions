# Creating an extension

Five-minute path. One manifest. Three models.

## 1. Create a directory

```bash
mkdir -p extensions/my-ext
```

## 2. Add `extension.toml`

Canonical contract: `impetus.extension_package.v1` (public `impetus-extension-sdk`).

Pick **one** entrypoint:

| Kind | When |
|------|------|
| `instruction_pack` | Skills / declarative guidance only |
| `mcp_bridge` | Tools already exposed through an MCP server |
| `host_process` | Isolated native/heavy integration (JSON-RPC child) |

### Instruction pack

```toml
schema_version = 1
id = "my-ext"
name = "My Ext"
version = "0.1.0"
description = "Portable instructions"
author = "You"
extension_api_version = 1
capabilities = ["skill_provider"]
permissions = []

[entrypoint]
kind = "instruction_pack"
root = "skills"
```

Then add `skills/SKILL.md`.

### MCP bridge

```toml
schema_version = 1
id = "my-tools"
name = "My Tools"
version = "0.1.0"
description = "Tools via MCP"
author = "You"
extension_api_version = 1
capabilities = ["mcp_integration", "tool"]
permissions = ["mcp", "process_spawn"]

[entrypoint]
kind = "mcp_bridge"
module_id = "my-tools"
```

Then add `mcp.json` whose `id`/`name` matches `module_id`.

### Host process

```toml
schema_version = 1
id = "my-host"
name = "My Host"
version = "0.1.0"
description = "Crash-isolated integration"
author = "You"
extension_api_version = 1
capabilities = ["tool"]
permissions = ["process_spawn"]

[entrypoint]
kind = "host_process"
command = "./bin/my-host"
args = ["--stdio"]
```

Child speaks the Impetus host JSON-RPC protocol. Language does not matter. **No** in-process native ABI.

## 3. Declare only needed permissions

Closed vocabulary (SDK): `filesystem_read`, `filesystem_write`, `network`, `process_spawn`, `pty`, `git`, `mcp`, `browser`, `lsp`, `memory`, `secrets_provider`.

Extensions cannot bypass daemon policy, approval, sandbox, or secrets boundaries by omitting or inventing tokens.

## 4. Validate

```bash
cargo run -p impetus-ext-support -- validate-manifests --root .
cargo run -p impetus-ext-support -- validate-catalog --root .
```

## 5. Add to catalog

```bash
cargo run -p impetus-ext-support -- sync-catalog --root . --write
```

Or edit `catalog.json` so `id` / `name` / `version` / `entrypoint` / `source_path` match `extension.toml`.

## What not to do

- Do not add `package.toml` under `extensions/` (legacy envelopes are generated only under `dist/`).
- Do not invent `native_module` or a second plugin API.
- Do not copy private `impetus-core` types — pin `impetus-extension-sdk` (see `compatibility.json` → `sdk.rev`).

See [manifest.md](manifest.md), [permissions.md](permissions.md), [architecture.md](architecture.md).
