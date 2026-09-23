# Catalog

`catalog.json` is publisher discovery metadata. **`extension.toml` remains
authoritative** for id, name, version, entrypoint, capabilities, and permissions.

- Discovery fields in the catalog (`id`, `name`, `version`, `entrypoint`,
  `source_path`, `portable_surface`) must match the canonical manifest.
- The catalog must not grant permissions or override package manifests.
- CI runs `impetus-ext validate-catalog` (also invoked from `validate-manifests`).

## Sync from manifests

```bash
cargo run -p impetus-ext-support -- sync-catalog --root . --write
```

Keeps `summary` / `implementation` / `distribution` when already present;
rewrites id/name/version/entrypoint/portable_surface/source_path from
`extension.toml`.

## Current state

Catalog points at source directories in this repository. Core does not yet
fetch it automatically — see [CORE_API_BLOCKERS.md](../CORE_API_BLOCKERS.md).

## Target state

Release pipeline later replaces `repository_source` with immutable GitHub
release artifacts containing `extension.toml`, skills/config, binaries, and
digests. Desktop and CLI ask Core for catalog state; neither downloads alone.
