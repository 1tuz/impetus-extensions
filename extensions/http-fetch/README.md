# HTTP Fetch (`impetus-ext-http-fetch`)

Outbound HTTP tools for Impetus MCP: `http_get` and `http_request`.

## Security

- Schemes allowlisted: `http`, `https` only (`file://` and others rejected).
- Methods allowlisted: `GET`, `HEAD`, `POST`.
- Timeout is **required** (`timeout_ms`).
- Response body size limited (default 1 MiB, hard max 8 MiB).
- Redirects to localhost / loopback blocked unless `allow_localhost_redirects=true` (default **off**).
- `Authorization` (and similar) header values redacted in logs.
- No secrets in `mcp.json` env values.

## Permissions

`network` only.

## Build / run

```bash
cargo build -p impetus-ext-http-fetch --release
./target/release/impetus-ext-http-fetch
```

## Install

```bash
impetus extension install --kind mcp extensions/http-fetch/mcp.json --root /path/to/project
```
