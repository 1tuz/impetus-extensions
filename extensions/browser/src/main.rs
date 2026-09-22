//! `impetus-ext-browser` — MCP stdio server for Browser Provider Protocol 0.1.

use anyhow::Result;
use clap::Parser;
use impetus_ext_browser::{Backend, build_handler, mock_shared, tool_defs};
use impetus_ext_mcp::{McpServer, ServerInfo};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "impetus-ext-browser",
    about = "Impetus first-party browser MCP extension"
)]
struct Args {
    /// In-memory mock backend (default; CI-safe).
    #[arg(long, default_value_t = true)]
    mock: bool,
    /// CDP backend (requires `--features cdp` build + allowlisted `BROWSER_BIN`).
    #[arg(long, default_value_t = false)]
    cdp: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();
    let backend = if args.cdp {
        Backend::cdp_from_env()
    } else {
        let _ = args.mock;
        mock_shared()
    };

    let handler = build_handler(backend);
    McpServer::new(
        ServerInfo {
            name: "browser".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        tool_defs(),
        handler,
    )
    .serve_stdio()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use impetus_ext_mcp::{ToolCall, ToolHandler};
    use serde_json::json;

    #[tokio::test]
    async fn mock_lifecycle_via_tools() {
        let handler = build_handler(mock_shared());
        let status = handler
            .call(ToolCall {
                name: "browser_status",
                arguments: json!({}),
            })
            .await
            .unwrap();
        assert!(!status.is_error);
        let session = handler
            .call(ToolCall {
                name: "browser_ensure_session",
                arguments: json!({"client_session_id": "c1"}),
            })
            .await
            .unwrap();
        assert!(!session.is_error);
        let sid: serde_json::Value = serde_json::from_str(&session.text).unwrap();
        let session_id = sid["session_id"].as_str().unwrap();
        let nav = handler
            .call(ToolCall {
                name: "browser_navigate",
                arguments: json!({"session_id": session_id, "url": "https://example.com"}),
            })
            .await
            .unwrap();
        assert!(!nav.is_error);
        let fetch = handler
            .call(ToolCall {
                name: "browser_fetch_rendered",
                arguments: json!({"session_id": session_id}),
            })
            .await
            .unwrap();
        assert!(fetch.text.to_lowercase().contains("example"));
        handler
            .call(ToolCall {
                name: "browser_cancel",
                arguments: json!({"session_id": session_id}),
            })
            .await
            .unwrap();
    }
}
