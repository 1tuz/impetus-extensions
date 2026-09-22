# Development loop

1. `git clone https://github.com/1tuz/impetus-extensions.git`
2. Pick an extension under `extensions/`
3. `cargo build -p <crate>` (or `./scripts/dev.sh`)
4. Put release binary on `PATH` (MCP) or use skill dir
5. `cargo run -p impetus-ext-support -- install-local extensions/<id> --root <project> [--daemon-mcp]`
6. Reload: restart `impetusd` if using daemon MCP SoT; skills are FS-based
7. Status: `impetus extension list --root <project>` / `doctor`
8. Use tools / skill; `cargo test -p <crate>`

Helper: `./scripts/install-local.sh extensions/<id> <project-root>`
