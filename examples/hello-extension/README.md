# hello-extension (canonical tutorial)

Pointer to the installable skill:

**[`extensions/hello-extension/`](../../extensions/hello-extension/)**

That directory is the source of truth (`package.toml`, `SKILL.md`, smoke tests).
This `examples/` copy exists so docs and onboarding can say “start at `examples/hello-extension`”.

`SKILL.md` here is a symlink to the extension tree.

## Quick install

```bash
impetus extension install --kind skill ../../extensions/hello-extension --root /path/to/project
```

See the extension [README](../../extensions/hello-extension/README.md) for `impetus-ext package` / `install-local` and CI tests.
