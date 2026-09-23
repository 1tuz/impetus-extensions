# TODO

Repo-local follow-ups. Core gaps stay in [CORE_API_BLOCKERS.md](CORE_API_BLOCKERS.md).

## Done (architecture migration)

- [x] `extension.toml` as sole authoring SoT under `extensions/`
- [x] Remove committed `package.toml` sources
- [x] Three entrypoints only: `instruction_pack` | `mcp_bridge` | `host_process`
- [x] Validate manifests via public `impetus-extension-sdk` git pin
- [x] Catalog consistency checks in CI
- [x] Reject `native_module` as a public entrypoint
- [x] Docs/authoring flow simplified

## Next

- [ ] When Core remote installer lands: drop legacy `package` / `install-local` Skill/MCP path or quarantine under `legacy/`
- [ ] Publish prebuilt Browser/LSP release archives + switch catalog `distribution` off `repository_source`
- [ ] Optional: migrate Browser/LSP binaries from MCP stdio to Impetus `host_process` protocol (keep `mcp_bridge` until then)
- [ ] Bump `sdk.rev` when Impetus cuts a tagged SDK-bearing release
- [ ] crates.io publish of `impetus-extension-sdk` (tracked as Core blocker #3)

## Non-goals

- Second plugin API / in-process native ABI
- Second memory store
- Desktop-side package manager
