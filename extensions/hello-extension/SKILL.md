---
name: hello-extension
description: Canonical Impetus instruction_pack tutorial — zero permissions
version: "0.1.0"
triggers:
  - hello-extension
  - impetus tutorial skill
---

# Hello Extension

Tiny public Impetus `instruction_pack`. No binary. No config. No FS/process/secrets.

Canonical authoring file: `extension.toml`. Skills live under `skills/`.

## Initialize

Preferred: drop the package under the Extension Host package root and reload.

Legacy Skill CLI (compatibility):

```bash
impetus-ext package extensions/hello-extension
impetus-ext install-local dist/hello-extension-0.1.0 --root /path/to/project
```

## Permissions

None. Keep `permissions = []` in `extension.toml`.

## Tests

```bash
bash extensions/hello-extension/tests/smoke.sh
```
