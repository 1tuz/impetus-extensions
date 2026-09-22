use anyhow::Result;
use impetus_ext_git_tools::{tool_git_diff, tool_git_log, tool_git_show, tool_git_status};
use impetus_ext_mcp::{MapHandler, McpServer, ServerInfo, ToolDef, ToolResult};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("impetus_ext_git_tools=info".parse()?),
        )
        .with_writer(std::io::stderr)
        .init();

    let tools = vec![
        ToolDef {
            name: "git_status".into(),
            description: "Read-only git status (--porcelain) for the workspace root.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Optional path under workspace" }
                }
            }),
        },
        ToolDef {
            name: "git_diff".into(),
            description: "Read-only git diff (optionally staged) for the workspace.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "staged": { "type": "boolean", "default": false }
                }
            }),
        },
        ToolDef {
            name: "git_log".into(),
            description: "Read-only git log (bounded).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500, "default": 20 },
                    "path": { "type": "string" }
                }
            }),
        },
        ToolDef {
            name: "git_show".into(),
            description: "Read-only git show for an object (commit/tree/blob).".into(),
            input_schema: json!({
                "type": "object",
                "required": ["object"],
                "properties": {
                    "object": { "type": "string" },
                    "path": { "type": "string" }
                }
            }),
        },
    ];

    let handler = MapHandler::new()
        .on("git_status", |args| {
            Box::pin(async move {
                Ok(match tool_git_status(args).await {
                    Ok(t) => ToolResult::ok(t),
                    Err(e) => ToolResult::err(e.to_string()),
                })
            })
        })
        .on("git_diff", |args| {
            Box::pin(async move {
                Ok(match tool_git_diff(args).await {
                    Ok(t) => ToolResult::ok(t),
                    Err(e) => ToolResult::err(e.to_string()),
                })
            })
        })
        .on("git_log", |args| {
            Box::pin(async move {
                Ok(match tool_git_log(args).await {
                    Ok(t) => ToolResult::ok(t),
                    Err(e) => ToolResult::err(e.to_string()),
                })
            })
        })
        .on("git_show", |args| {
            Box::pin(async move {
                Ok(match tool_git_show(args).await {
                    Ok(t) => ToolResult::ok(t),
                    Err(e) => ToolResult::err(e.to_string()),
                })
            })
        });

    let server = McpServer::new(
        ServerInfo {
            name: "impetus-ext-git-tools".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        tools,
        handler,
    );
    server.serve_stdio().await
}
