# Git Tools (`impetus-ext-git-tools`)

Read-only git MCP server for Impetus. Exposes `git_status`, `git_diff`, `git_log`, and `git_show`.

## Security

- Requires `IMPETUS_WORKSPACE_ROOT` (absolute path to the workspace).
- Spawns `git` via argv arrays only — never `sh -c`.
- Allowlisted subcommands: `status`, `diff`, `log`, `show`.
- No commit, push, force, checkout, or rewrite operations.

## Permissions

`process` — only to exec the git binary with read-only allowlisted args.

## Build / run

```bash
cargo build -p impetus-ext-git-tools --release
IMPETUS_WORKSPACE_ROOT=/path/to/repo ./target/release/impetus-ext-git-tools
```

## Install

```bash
impetus-ext package extensions/git-tools
impetus extension install --kind mcp extensions/git-tools/mcp.json --root /path/to/project
```

Set `IMPETUS_WORKSPACE_ROOT` in the Impetus MCP env for the project root. Env keys in `mcp.json` are labels only — never put secrets in values.
