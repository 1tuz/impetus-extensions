# Manifest

## Core envelope (`impetus.extension.v1`)

Documented fields only (Impetus `v0.1.2`):

| Field | Rule |
|-------|------|
| `schema_version` | `1` |
| `id` | `^[a-z0-9][a-z0-9_-]{0,63}$`, len ≤ 64 |
| `kind` | `skill` \| `mcp_config` |
| `version` | non-empty |
| `digest` | `sha256:` + 64 lowercase hex |
| `capabilities` | ≥1 unique tokens `^[a-z][a-z0-9_:-]{0,63}$` |

Unknown critical top-level keys → reject.

## Ecosystem `package.toml`

Not read by `impetus extension install`. Used for packaging, CI, permission rationale, compatibility pins.

Do not put undocumented keys into `manifest.json`.
