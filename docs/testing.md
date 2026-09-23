# Testing

## Unit

```bash
cargo test --workspace
```

Mock backends (`browser --mock`, `lsp --mock`) must pass without Chrome / rust-analyzer.

## Manifest / catalog / compat (fast PR gate)

```bash
cargo run -p impetus-ext-support -- validate-manifests --root .
cargo run -p impetus-ext-support -- validate-catalog --root .
cargo run -p impetus-ext-support -- check-compat --root .
```

Checks: schema via SDK, entrypoint artifacts, duplicate ids, permissions
vocabulary, catalog consistency, no `package.toml` under `extensions/`,
SDK rev pin, no `native_module` entrypoint.

## Integration

```bash
./tests/integration/run_all.sh
```

- Always: MCP stdio smoke (initialize → tools/list → tools/call)
- If `impetus` on PATH: legacy skill/MCP install + enable/disable
- AgentLoop skill inject may be incomplete in core — scripts skip that assertion honestly

## Security checks in tests

- path escape rejection (code-search)
- git subcommand allowlist
- http scheme / localhost blocks
