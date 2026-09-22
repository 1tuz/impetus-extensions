# Integration tests

- Always run MCP mock stdio smokes (no Impetus binary required).
- Skill/MCP lifecycle tests run when `impetus` is on `PATH` (prefer build from tag `v0.1.2`).
- AgentLoop skill injection may be incomplete in core; those asserts are skipped with a note.
