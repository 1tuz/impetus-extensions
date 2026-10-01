# hello-host-process

Installable `host_process` demo for Impetus Extension Host.

Child `./ext.sh` speaks newline-delimited JSON-RPC
(`extension/initialize` / `operate` / `cancel` / `shutdown`) and answers
`op=echo` — same shape as the Impetus SDK `host-process-echo` fixture.

## Install smoke (local package path)

Requires Impetus CLI/daemon with package Install IPC (#447+):

```bash
# From an Impetus checkout with daemon sock live (or offline host fallback):
impetus extension install package /path/to/impetus-extensions/extensions/hello-host-process
impetus extension list   # or IPC ListExtensionPackages
# Enable if needed, then:
# OperateExtensionPackage id=hello-host-process op=echo
```

Validate in this repo:

```bash
cargo run -p impetus-ext-support -- validate-manifests --root .
```

See Impetus [host-protocol.md](https://github.com/1tuz/impetus/blob/main/docs/extensions/host-protocol.md).
