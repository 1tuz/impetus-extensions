# Integration tests (`tests/integration/`)

Exercises the **public** Impetus extension path from this repo. Does **not** modify or link Impetus core.

## Requirements

| Variable / tool | Required? | Purpose |
|---|---|---|
| Rust toolchain (`rust-toolchain.toml`, channel **1.98.0**) | yes | build packages / MCP binaries |
| `python3` | yes | MCP stdio JSON-RPC probe |
| `IMPETUS_BIN` or `impetus` on `PATH` | optional | install / list / enable / disable |
| `IMPETUSD_BIN` or `impetusd` on `PATH` | optional | daemon MCP SoT note only |
| `IMPETUS_TAG` | optional (default **`v0.1.2`**) | must match `compatibility.json` |
| Chrome / CDP | **no** | browser tests use `--mock` |
| `rust-analyzer` | **no** | LSP tests use `--mock` |
| Network | **no** | offline |

Pin: Impetus release tag **`v0.1.2`** (`compatibility.json` → `impetus_tag`).

## What is covered

Always (CI without Impetus):

1. **`test_mcp_browser_mock.sh`** — build `impetus-ext-browser`, stdio MCP `initialize` → `tools/list` → `tools/call` (`--mock`).
2. **`test_mcp_lsp_mock.sh`** — same for `impetus-ext-lsp` (`--mock`).
3. **`test_hello_skill.sh`** — package `extensions/hello-extension` to a valid `impetus.extension.v1` manifest.

When `impetus` / `IMPETUS_BIN` is available:

4. **`test_hello_skill.sh`** — `impetus extension install --kind skill` into a temp `--root`, then `extension list`.
5. **`test_lifecycle_enable_disable.sh`** — install → list → disable → enable; optional copy of browser `mcp.json` into `$IMPETUS_DATA_DIR/mcp`.

## What is skipped / not asserted

| Condition | Behavior |
|---|---|
| No `impetus` / `IMPETUS_BIN` | Lifecycle + install steps **SKIP** (exit **0**) with a clear `SKIP:` message |
| No `impetusd` | Daemon reload **not** probed; SoT file may still be written |
| AgentLoop skill inject | **Not tested** — Impetus may not inject installed skills into a live agent session yet. Public API stops at install/list/enable/disable |
| Dynamic `BrowserProvider` / coding-tools registration | Unsupported until core unblocks (see `compatibility.json` → `unsupported_until_core_unblocks`) |

## Run

```bash
# MCP mocks only (what CI should always run)
bash tests/integration/test_mcp_browser_mock.sh
bash tests/integration/test_mcp_lsp_mock.sh

# Everything (skips impetus-dependent parts if CLI missing)
bash tests/integration/run_all.sh

# Point at a local Impetus build
export IMPETUS_BIN=/path/to/impetus
export IMPETUS_TAG=v0.1.2
bash tests/integration/run_all.sh
```

CI job: `.github/workflows/ci.yml` → `integration-mocks` runs MCP mock scripts + hello packaging (no Impetus install required).
