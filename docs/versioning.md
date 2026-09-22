# Versioning

- Each extension has its **own** semver (`package.toml` `version`). It is **not** the Impetus version.
- Compatibility is pinned via `compatibility.impetus_tag` (currently `v0.1.2`) and schema ids.
- Stand-in `extension_api_version` (`0.1.0-skill-mcp`) until core ships a real constant.
- Breaking tool schemas or permission needs → bump major.
- Deprecation: keep old tool names one minor release with warnings in README; then remove on major.

See `compatibility.json`.
