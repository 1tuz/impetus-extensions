# Versioning

- Each extension has its **own** semver in `extension.toml` (`version`). It is **not** the Impetus app version.
- Compatibility is `extension_api_version` (integer major, currently `1`) validated by the SDK against Core's supported range.
- SDK pin: `compatibility.json` → `sdk.rev` (immutable Impetus git SHA).
  Keep `Cargo.toml` workspace dep + `SDK_GIT_REV` in `impetus-ext-support` in
  lockstep. Current pin includes package Install/Remove IPC (#447).
- Legacy Skill/MCP CLI helper still mentions Impetus tag `v0.1.2` under `legacy_helper` — that path is compatibility-only.
- Breaking tool schemas or permission needs → bump package major.
- Deprecation: keep old tool names one minor release with warnings in README; then remove on major.
