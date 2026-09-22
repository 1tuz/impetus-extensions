//! Generic LSP MCP extension for Impetus.
//!
//! Spawns a configurable language server over stdio (Content-Length framed
//! JSON-RPC) and exposes MCP tools. Architecture is language-agnostic;
//! `presets/rust-analyzer.json` is a reference config only.

#![allow(clippy::collapsible_if)]
#![allow(clippy::while_let_loop)]

pub mod allowlist;
pub mod framing;
pub mod session;

use anyhow::{Result, anyhow};
use impetus_ext_mcp::{BoxFuture, MapHandler, McpServer, ServerInfo, ToolDef, ToolResult};
use serde_json::{Value, json};
use session::{LspSession, StartConfig};
use std::path::PathBuf;
use std::sync::Arc;

pub fn parse_args_list(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    if raw.starts_with('[') {
        if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(raw) {
            return items
                .into_iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
        }
    }
    raw.split_whitespace().map(str::to_string).collect()
}

pub fn defaults_from_env() -> (Option<String>, Vec<String>, Option<PathBuf>) {
    let command = std::env::var("IMPETUS_LSP_COMMAND")
        .ok()
        .filter(|s| !s.is_empty());
    let args = parse_args_list(std::env::var("IMPETUS_LSP_ARGS").ok().as_deref());
    let workspace = std::env::var("IMPETUS_LSP_WORKSPACE")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
    (command, args, workspace)
}

pub fn build_server(session: Arc<LspSession>) -> McpServer<MapHandler> {
    let tools = tool_defs();
    let handler = MapHandler::new()
        .on("lsp_start", {
            let session = session.clone();
            move |args| {
                let session = session.clone();
                Box::pin(async move { tool_start(&session, args).await }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_stop", {
            let session = session.clone();
            move |_args| {
                let session = session.clone();
                Box::pin(async move {
                    match session.stop().await {
                        Ok(v) => Ok(ToolResult::ok(v.to_string())),
                        Err(e) => Ok(ToolResult::err(e.to_string())),
                    }
                }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_restart", {
            let session = session.clone();
            move |args| {
                let session = session.clone();
                Box::pin(async move { tool_restart(&session, args).await }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_diagnostics", {
            let session = session.clone();
            move |args| {
                let session = session.clone();
                Box::pin(async move {
                    let uri = args.get("uri").and_then(|v| v.as_str());
                    match session.diagnostics(uri).await {
                        Ok(v) => Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?)),
                        Err(e) => Ok(ToolResult::err(e.to_string())),
                    }
                }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_request", {
            let session = session.clone();
            move |args| {
                let session = session.clone();
                Box::pin(async move {
                    let method = args
                        .get("method")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow!("missing method"))?;
                    let params = args.get("params").cloned().unwrap_or(json!({}));
                    match session.request(method, params).await {
                        Ok(v) => Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?)),
                        Err(e) => Ok(ToolResult::err(e.to_string())),
                    }
                }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_hover", {
            let session = session.clone();
            move |args| {
                let session = session.clone();
                Box::pin(async move {
                    let (uri, line, character) = position_args(&args)?;
                    match session.hover(&uri, line, character).await {
                        Ok(v) => Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?)),
                        Err(e) => Ok(ToolResult::err(e.to_string())),
                    }
                }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_definition", {
            let session = session.clone();
            move |args| {
                let session = session.clone();
                Box::pin(async move {
                    let (uri, line, character) = position_args(&args)?;
                    match session.definition(&uri, line, character).await {
                        Ok(v) => Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?)),
                        Err(e) => Ok(ToolResult::err(e.to_string())),
                    }
                }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_references", {
            let session = session.clone();
            move |args| {
                let session = session.clone();
                Box::pin(async move {
                    let (uri, line, character) = position_args(&args)?;
                    match session.references(&uri, line, character).await {
                        Ok(v) => Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?)),
                        Err(e) => Ok(ToolResult::err(e.to_string())),
                    }
                }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_symbols", {
            let session = session.clone();
            move |args| {
                let session = session.clone();
                Box::pin(async move {
                    let uri = args.get("uri").and_then(|v| v.as_str());
                    let query = args.get("query").and_then(|v| v.as_str());
                    match session.symbols(uri, query).await {
                        Ok(v) => Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?)),
                        Err(e) => Ok(ToolResult::err(e.to_string())),
                    }
                }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_cancel", {
            let session = session.clone();
            move |args| {
                let session = session.clone();
                Box::pin(async move {
                    let id = args
                        .get("id")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| anyhow!("missing id"))?;
                    match session.cancel(id).await {
                        Ok(v) => Ok(ToolResult::ok(v.to_string())),
                        Err(e) => Ok(ToolResult::err(e.to_string())),
                    }
                }) as BoxFuture<'_, _>
            }
        })
        .on("lsp_status", {
            let session = session.clone();
            move |_args| {
                let session = session.clone();
                Box::pin(async move {
                    let info = session.info().await;
                    Ok(ToolResult::ok(serde_json::to_string_pretty(&info)?))
                }) as BoxFuture<'_, _>
            }
        });

    McpServer::new(
        ServerInfo {
            name: "impetus-ext-lsp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        tools,
        handler,
    )
}

fn position_args(args: &Value) -> Result<(String, u32, u32)> {
    let uri = args
        .get("uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("missing uri"))?
        .to_string();
    let line = args
        .get("line")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("missing line"))? as u32;
    let character = args
        .get("character")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("missing character"))? as u32;
    Ok((uri, line, character))
}

fn start_config_from_args(args: &Value) -> StartConfig {
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let args_list = match args.get("args") {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(Value::String(s)) => parse_args_list(Some(s)),
        _ => Vec::new(),
    };
    let workspace = args
        .get("workspace")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_default();
    let mock = args.get("mock").and_then(|v| v.as_bool()).unwrap_or(false);
    StartConfig {
        command,
        args: args_list,
        workspace,
        mock,
    }
}

async fn tool_start(session: &LspSession, args: Value) -> Result<ToolResult> {
    match session.start(start_config_from_args(&args)).await {
        Ok(v) => Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?)),
        Err(e) => Ok(ToolResult::err(e.to_string())),
    }
}

async fn tool_restart(session: &LspSession, args: Value) -> Result<ToolResult> {
    let cfg = if args.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
        Some(start_config_from_args(&args))
    } else {
        None
    };
    match session.restart(cfg).await {
        Ok(v) => Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?)),
        Err(e) => Ok(ToolResult::err(e.to_string())),
    }
}

fn tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "lsp_start".into(),
            description: "Start language server child (stdio JSON-RPC). Pass command/args/workspace or rely on env / CLI defaults. Set mock=true for CI fake LS.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Language server binary" },
                    "args": {
                        "oneOf": [
                            { "type": "array", "items": { "type": "string" } },
                            { "type": "string" }
                        ]
                    },
                    "workspace": { "type": "string", "description": "Workspace root path" },
                    "mock": { "type": "boolean" }
                }
            }),
        },
        ToolDef {
            name: "lsp_stop".into(),
            description: "Stop the running language server session.".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        ToolDef {
            name: "lsp_restart".into(),
            description: "Restart the language server (optional new command/args/workspace).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "args": {
                        "oneOf": [
                            { "type": "array", "items": { "type": "string" } },
                            { "type": "string" }
                        ]
                    },
                    "workspace": { "type": "string" },
                    "mock": { "type": "boolean" }
                }
            }),
        },
        ToolDef {
            name: "lsp_diagnostics".into(),
            description: "Return cached publishDiagnostics (optionally filtered by uri).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": { "type": "string" }
                }
            }),
        },
        ToolDef {
            name: "lsp_request".into(),
            description: "Generic allowlisted LSP request escape hatch.".into(),
            input_schema: json!({
                "type": "object",
                "required": ["method"],
                "properties": {
                    "method": { "type": "string" },
                    "params": {}
                }
            }),
        },
        ToolDef {
            name: "lsp_hover".into(),
            description: "textDocument/hover convenience.".into(),
            input_schema: position_schema(),
        },
        ToolDef {
            name: "lsp_definition".into(),
            description: "textDocument/definition convenience.".into(),
            input_schema: position_schema(),
        },
        ToolDef {
            name: "lsp_references".into(),
            description: "textDocument/references convenience.".into(),
            input_schema: position_schema(),
        },
        ToolDef {
            name: "lsp_symbols".into(),
            description: "documentSymbol (uri) or workspace/symbol (query).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": { "type": "string" },
                    "query": { "type": "string" }
                }
            }),
        },
        ToolDef {
            name: "lsp_cancel".into(),
            description: "Cancel in-flight LSP request by id ($/cancelRequest).".into(),
            input_schema: json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "integer" }
                }
            }),
        },
        ToolDef {
            name: "lsp_status".into(),
            description: "Session status (running, mock, command, workspace).".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
    ]
}

fn position_schema() -> Value {
    json!({
        "type": "object",
        "required": ["uri", "line", "character"],
        "properties": {
            "uri": { "type": "string" },
            "line": { "type": "integer", "minimum": 0 },
            "character": { "type": "integer", "minimum": 0 }
        }
    })
}
