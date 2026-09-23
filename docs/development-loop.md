# Development loop

1. `git clone https://github.com/1tuz/impetus-extensions.git`
2. Create/edit package under `extensions/<id>/` with `extension.toml`
3. `cargo run -p impetus-ext-support -- validate-manifests --root .`
4. For Rust MCP binaries: `cargo build -p <crate>` (or `./scripts/dev.sh`)
5. Optional legacy local install: `cargo run -p impetus-ext-support -- install-local extensions/<id> --root <project> [--daemon-mcp]`
6. Preferred runtime path: copy pack under `$IMPETUS_DATA_DIR/extensions/packages/` and reload via Core IPC
7. Status: `impetus extension list --root <project>` (legacy) / daemon package list IPC
8. `cargo test -p <crate>`

Helper: `./scripts/install-local.sh extensions/<id> <project-root>`
