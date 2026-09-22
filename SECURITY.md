# Security

Impetus extensions can read workspaces, spawn processes (MCP stdio), and talk to networks. Treat packs as untrusted code until reviewed.

## No sudo / root

- Install and run as a normal user only.
- Data lives under project `.impetus/` and `$IMPETUS_DATA_DIR` (default `~/.local/share/impetus`).
- Never document or require root, setuid, or world-writable install paths.
- Refuse to run packagers/install helpers if the target tree is not user-owned.

## Reporting

1. Prefer GitHub private vulnerability reporting on this repository when enabled.
2. Otherwise contact a maintainer via their GitHub profile and ask for a private channel first.
3. Do not file public issues with secrets, Keychain labels + values, full exploit payloads, or local file dumps.

Include: affected revision / extension id, minimal reproduction, impact, and any mitigation tested.

## Threat model (MCP extensions)

| Surface | Risk | Mitigations |
|---------|------|-------------|
| MCP `command` / `args` | Arbitrary process spawn | Review `mcp.json`; pin binary paths; least `env_keys` |
| Tool arguments | Path escape, SSRF, command injection | Sandbox in extension code; no shell-join of untrusted strings |
| `env` / secrets | Credential leak into logs/manifests | Keys only in envelopes (`env_keys`); never secret values in git |
| Dual MCP paths | Project install vs daemon SoT diverge | Explicit `--daemon-mcp` / copy; document which path is live |
| Skill text | Prompt injection into agent instructions | Small skills; no embedded secrets; review before enable |
| Network tools (`http-fetch`, browser) | Data exfil / SSRF | Allowlists, deny private ranges unless declared |

Extensions do not bypass Impetus `Policy → Approval → Sandbox`. If a pack claims otherwise, treat it as a bug.

## Scope

In scope: first-party packs and helpers in this repo. Core daemon/CLI flaws → report against [impetus](https://github.com/1tuz/impetus) ([SECURITY.md](https://github.com/1tuz/impetus/blob/main/SECURITY.md)).
