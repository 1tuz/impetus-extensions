use anyhow::Result;
use impetus_ext_code_search::tool_search_text;
use impetus_ext_mcp::{MapHandler, McpServer, ServerInfo, ToolDef};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("impetus_ext_code_search=info".parse()?),
        )
        .with_writer(std::io::stderr)
        .init();

    let tools = vec![ToolDef {
        name: "search_text".into(),
        description:
            "Search text under the workspace root (ripgrep if available, else walk+regex).".into(),
        input_schema: json!({
            "type": "object",
            "required": ["pattern"],
            "properties": {
                "pattern": { "type": "string", "description": "Regex pattern" },
                "path": { "type": "string", "description": "Subpath under workspace", "default": "." },
                "case_insensitive": { "type": "boolean", "default": false },
                "max_matches": { "type": "integer", "minimum": 1, "maximum": 2000, "default": 200 }
            }
        }),
    }];

    let handler = MapHandler::new().on("search_text", |args| {
        Box::pin(async move { tool_search_text(args).await })
    });

    McpServer::new(
        ServerInfo {
            name: "impetus-ext-code-search".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        tools,
        handler,
    )
    .serve_stdio()
    .await
}
