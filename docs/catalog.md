# Catalog

`catalog.json` is publisher metadata for first-party extension discovery. It is intentionally separate from `extension.toml`.

- `extension.toml` is runtime metadata consumed by the Impetus Extension Host.
- `catalog.json` is discovery/distribution metadata consumed by a future Core Extension Manager.

The catalog must not grant permissions or override package manifests. The installed package manifest remains authoritative for compatibility and permission checks.

## Current state

The catalog currently points to source directories in this repository. Core does not yet fetch it automatically.

## Target state

A release pipeline can later replace `repository_source` distribution entries with immutable GitHub release artifacts containing:

- `extension.toml`
- required skills/config
- target-specific binaries where needed
- content digest/checksum metadata

Core should download to a staging directory, validate compatibility/permissions/digest, and atomically promote the package only after validation.

Desktop and CLI should both ask Core for catalog state; neither client should download packages independently.

## Refresh policy

Recommended client behavior:

- show cached catalog immediately;
- refresh catalog asynchronously on open/start;
- expose an explicit Refresh action;
- do not automatically update installed extensions when a new package version or new permissions appear.
