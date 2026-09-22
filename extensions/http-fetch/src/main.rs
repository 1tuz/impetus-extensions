use anyhow::Result;
use impetus_ext_http_fetch::{http_get, http_request};
use impetus_ext_mcp::{MapHandler, McpServer, ServerInfo, ToolDef, ToolResult};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("impetus_ext_http_fetch=info".parse()?),
        )
        .with_writer(std::io::stderr)
        .init();

    let tools = vec![
        ToolDef {
            name: "http_get".into(),
            description:
                "HTTP GET with required timeout, size limit, and localhost-redirect guard.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["url", "timeout_ms"],
                "properties": {
                    "url": { "type": "string" },
                    "timeout_ms": { "type": "integer", "minimum": 1 },
                    "headers": { "type": "object", "additionalProperties": { "type": "string" } },
                    "max_bytes": { "type": "integer", "minimum": 1 },
                    "allow_localhost_redirects": { "type": "boolean", "default": false }
                }
            }),
        },
        ToolDef {
            name: "http_request".into(),
            description: "HTTP GET/HEAD/POST with required timeout and SSRF-oriented guards."
                .into(),
            input_schema: json!({
                "type": "object",
                "required": ["url", "method", "timeout_ms"],
                "properties": {
                    "url": { "type": "string" },
                    "method": { "type": "string", "enum": ["GET", "HEAD", "POST"] },
                    "timeout_ms": { "type": "integer", "minimum": 1 },
                    "headers": { "type": "object", "additionalProperties": { "type": "string" } },
                    "body": { "type": "string" },
                    "max_bytes": { "type": "integer", "minimum": 1 },
                    "allow_localhost_redirects": { "type": "boolean", "default": false }
                }
            }),
        },
    ];

    let handler = MapHandler::new()
        .on("http_get", |args| {
            Box::pin(async move {
                Ok(match http_get(args).await {
                    Ok(t) => ToolResult::ok(t),
                    Err(e) => ToolResult::err(e.to_string()),
                })
            })
        })
        .on("http_request", |args| {
            Box::pin(async move {
                Ok(match http_request(args).await {
                    Ok(t) => ToolResult::ok(t),
                    Err(e) => ToolResult::err(e.to_string()),
                })
            })
        });

    McpServer::new(
        ServerInfo {
            name: "impetus-ext-http-fetch".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        tools,
        handler,
    )
    .serve_stdio()
    .await
}
