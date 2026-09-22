# Packaging

Canonical layout after `impetus-ext package`:

```
dist/{id}-{version}/
  manifest.json     # impetus.extension.v1 only
  package.toml
  README.md
  SKILL.md | mcp.json
  [binary if copied]
```

Validate `manifest.json` before install. Package contains only needed artifacts — no `target/`, no `.git`, no secrets.
