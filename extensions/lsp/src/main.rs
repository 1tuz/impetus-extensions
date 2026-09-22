//! Generic LSP MCP extension for Impetus.

mod lsp;

use anyhow::Result;
use clap::Parser;
use impetus_ext_mcp::{McpServer, ServerInfo, ToolCall, ToolDef, ToolHandler, ToolResult};
use lsp::{LspSession, MockLsp, ProcessLsp};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
#[command(name = "impetus-ext-lsp")]
struct Args {
    /// Fake LS for CI (default true when flag present; use --no-mock for real process).
    #[arg(long, default_value_t = true, action = clap::ArgAction::SetTrue)]
    mock: bool,
    #[arg(long, overrides_with = "mock")]
    no_mock: bool,
}

struct Handler {
    mock: bool,
    session: Arc<Mutex<Option<Box<dyn LspSession>>>>,
}

impl ToolHandler for Handler {
    fn call<'a>(
        &'a self,
        call: ToolCall<'a>,
    ) -> impetus_ext_mcp::BoxFuture<'a, Result<ToolResult>> {
        Box::pin(async move {
            match call.name {
                "lsp_start" => {
                    let command = call
                        .arguments
                        .get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("rust-analyzer");
                    let workspace = call
                        .arguments
                        .get("workspace")
                        .and_then(|v| v.as_str())
                        .unwrap_or(".");
                    let args: Vec<String> = call
                        .arguments
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let mut slot = self.session.lock().await;
                    if let Some(existing) = slot.as_mut() {
                        existing.stop().await.ok();
                    }
                    let sess: Box<dyn LspSession> = if self.mock {
                        Box::new(MockLsp::start(command, args, workspace).await?)
                    } else {
                        Box::new(ProcessLsp::start(command, args, workspace).await?)
                    };
                    *slot = Some(sess);
                    ok_json(json!({"started": true, "command": command, "workspace": workspace}))
                }
                "lsp_stop" => {
                    let mut slot = self.session.lock().await;
                    if let Some(mut s) = slot.take() {
                        s.stop().await?;
                    }
                    ok_json(json!({"stopped": true}))
                }
                "lsp_restart" => {
                    let mut slot = self.session.lock().await;
                    let Some(s) = slot.as_mut() else {
                        return Ok(ToolResult::err("no session"));
                    };
                    s.restart().await?;
                    ok_json(json!({"restarted": true}))
                }
                "lsp_diagnostics" => {
                    let uri = call.arguments.get("uri").and_then(|v| v.as_str());
                    let slot = self.session.lock().await;
                    let Some(s) = slot.as_ref() else {
                        return Ok(ToolResult::err("no session"));
                    };
                    ok_json(s.diagnostics(uri).await?)
                }
                "lsp_request" => {
                    let method = call
                        .arguments
                        .get("method")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("missing method"))?;
                    let params = call
                        .arguments
                        .get("params")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    let slot = self.session.lock().await;
                    let Some(s) = slot.as_ref() else {
                        return Ok(ToolResult::err("no session"));
                    };
                    ok_json(s.request(method, params).await?)
                }
                "lsp_hover" | "lsp_definition" | "lsp_references" | "lsp_symbols" => {
                    let slot = self.session.lock().await;
                    let Some(s) = slot.as_ref() else {
                        return Ok(ToolResult::err("no session"));
                    };
                    ok_json(s.convenience(call.name, call.arguments.clone()).await?)
                }
                "lsp_cancel" => {
                    let slot = self.session.lock().await;
                    let Some(s) = slot.as_ref() else {
                        return Ok(ToolResult::err("no session"));
                    };
                    s.cancel().await?;
                    ok_json(json!({"cancelled": true}))
                }
                other => Ok(ToolResult::err(format!("unknown tool `{other}`"))),
            }
        })
    }
}

fn ok_json(v: Value) -> Result<ToolResult> {
    Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?))
}

fn tools() -> Vec<ToolDef> {
    let schema = json!({"type":"object"});
    [
        "lsp_start",
        "lsp_stop",
        "lsp_restart",
        "lsp_diagnostics",
        "lsp_request",
        "lsp_hover",
        "lsp_definition",
        "lsp_references",
        "lsp_symbols",
        "lsp_cancel",
    ]
    .into_iter()
    .map(|name| ToolDef {
        name: name.into(),
        description: name.into(),
        input_schema: schema.clone(),
    })
    .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_writer(std::io::stderr)
        .init();
    let args = Args::parse();
    let mock = !args.no_mock;
    let handler = Handler {
        mock,
        session: Arc::new(Mutex::new(None)),
    };
    McpServer::new(
        ServerInfo {
            name: "impetus-ext-lsp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        tools(),
        handler,
    )
    .serve_stdio()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_diagnostics() {
        let handler = Handler {
            mock: true,
            session: Arc::new(Mutex::new(None)),
        };
        handler
            .call(ToolCall {
                name: "lsp_start",
                arguments: json!({"command":"rust-analyzer","workspace":"."}),
            })
            .await
            .unwrap();
        let diag = handler
            .call(ToolCall {
                name: "lsp_diagnostics",
                arguments: json!({}),
            })
            .await
            .unwrap();
        assert!(!diag.is_error);
        handler
            .call(ToolCall {
                name: "lsp_stop",
                arguments: json!({}),
            })
            .await
            .unwrap();
    }
}
