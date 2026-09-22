# Testing

## Unit

```bash
cargo test --workspace
```

Mock backends (`browser --mock`, `lsp --mock`) must pass without Chrome / rust-analyzer.

## Manifest / compat

```bash
cargo run -p impetus-ext-support -- validate-manifests
cargo run -p impetus-ext-support -- check-compat
```

## Integration

```bash
./tests/integration/run_all.sh
```

- Always: MCP stdio smoke (initialize → tools/list → tools/call)
- If `impetus` on PATH (built from tag `v0.1.2`): skill/MCP install + enable/disable
- AgentLoop skill inject may be incomplete in core — scripts skip that assertion honestly

## Security checks in tests

- path escape rejection (code-search)
- git subcommand allowlist
- http scheme / localhost blocks
