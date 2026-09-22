---
name: hello-extension
description: Canonical Impetus skill tutorial — initialize, capabilities, zero permissions
version: "0.1.0"
triggers:
  - hello-extension
  - impetus tutorial skill
---

# Hello Extension

Tiny public Impetus install kind `skill`. No binary. No config. No FS/process/secrets.

## Manifest (core envelope)

Packaging writes `impetus.extension.v1` with only documented fields:

- `schema_version`, `id`, `kind`, `version`, `digest`, `capabilities`

`package.toml` is ecosystem metadata; core `impetus extension install` does not read it.

## Initialize

```bash
# from this directory
impetus extension install --kind skill . --root /path/to/project

# or via repo helper after packaging
impetus-ext package extensions/hello-extension
impetus-ext install-local dist/hello-extension-0.1.0 --root /path/to/project
```

Install lifecycle: Impetus imports `SKILL.md` → registers skill module → capabilities available to the agent.

## Capabilities

Registered as `instructions` + `triggers` (from body + frontmatter triggers). No tools.

## Permissions

None. Skills need no FS/process/secrets. Keep `permissions = []`.

## Configuration

None required. No settings file, no env vars.

## Tests

```bash
# no impetus binary needed
bash extensions/hello-extension/tests/smoke.sh
cargo test -p impetus-ext-support package_hello_extension_from_repo
```
