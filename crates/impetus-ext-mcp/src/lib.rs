//! Minimal MCP stdio JSON-RPC helpers for first-party Impetus extensions.
//!
//! Implements only the subset Impetus MCP tools path needs today:
//! `initialize`, `notifications/initialized`, `tools/list`, `tools/call`, `ping`.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

pub struct ToolCall<'a> {
    pub name: &'a str,
    pub arguments: Value,
}

pub trait ToolHandler: Send + Sync {
    fn call<'a>(&'a self, call: ToolCall<'a>) -> BoxFuture<'a, Result<ToolResult>>;
}

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub text: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn ok(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: false,
        }
    }

    pub fn err(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

pub struct McpServer<H: ToolHandler> {
    info: ServerInfo,
    tools: Vec<ToolDef>,
    handler: H,
}

impl<H: ToolHandler> McpServer<H> {
    pub fn new(info: ServerInfo, tools: Vec<ToolDef>, handler: H) -> Self {
        Self {
            info,
            tools,
            handler,
        }
    }

    pub async fn serve_stdio(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let msg: JsonRpcMessage = serde_json::from_str(trimmed)
                .with_context(|| format!("invalid JSON-RPC line: {trimmed}"))?;
            if let Some(response) = self.handle_message(msg).await? {
                let encoded = serde_json::to_string(&response)?;
                stdout.write_all(encoded.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }
        Ok(())
    }

    async fn handle_message(&self, msg: JsonRpcMessage) -> Result<Option<JsonRpcResponse>> {
        match msg {
            JsonRpcMessage::Request(req) => {
                let result = self.dispatch(&req.method, req.params.clone()).await;
                Ok(Some(match result {
                    Ok(value) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id: req.id,
                        result: Some(value),
                        error: None,
                    },
                    Err(err) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id: req.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: err.to_string(),
                        }),
                    },
                }))
            }
            JsonRpcMessage::Notification(_) => Ok(None),
        }
    }

    async fn dispatch(&self, method: &str, params: Option<Value>) -> Result<Value> {
        match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": self.info.name,
                    "version": self.info.version,
                }
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": self.tools })),
            "tools/call" => {
                let params = params.ok_or_else(|| anyhow!("tools/call missing params"))?;
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("tools/call missing name"))?;
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                if !self.tools.iter().any(|t| t.name == name) {
                    return Err(anyhow!("unknown tool `{name}`"));
                }
                let result = self.handler.call(ToolCall { name, arguments }).await?;
                Ok(json!({
                    "content": [{ "type": "text", "text": result.text }],
                    "isError": result.is_error
                }))
            }
            other => Err(anyhow!("method not found: {other}")),
        }
    }
}

/// Owned future returned by [`MapHandler`] tool closures.
pub type OwnedToolFuture = Pin<Box<dyn Future<Output = Result<ToolResult>> + Send + 'static>>;

/// Simple handler backed by an async map of closures.
pub struct MapHandler {
    map: HashMap<String, Box<dyn Fn(Value) -> OwnedToolFuture + Send + Sync>>,
}

impl MapHandler {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn on<F>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn(Value) -> OwnedToolFuture + Send + Sync + 'static,
    {
        self.map.insert(name.into(), Box::new(f));
        self
    }
}

impl Default for MapHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolHandler for MapHandler {
    fn call<'a>(&'a self, call: ToolCall<'a>) -> BoxFuture<'a, Result<ToolResult>> {
        Box::pin(async move {
            let f = self
                .map
                .get(call.name)
                .ok_or_else(|| anyhow!("no handler for `{}`", call.name))?;
            f(call.arguments).await
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcNotification {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    method: String,
    #[allow(dead_code)]
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo;

    impl ToolHandler for Echo {
        fn call<'a>(&'a self, call: ToolCall<'a>) -> BoxFuture<'a, Result<ToolResult>> {
            Box::pin(async move { Ok(ToolResult::ok(format!("{}:{}", call.name, call.arguments))) })
        }
    }

    #[tokio::test]
    async fn tools_list_and_call() {
        let server = McpServer::new(
            ServerInfo {
                name: "t".into(),
                version: "0.1.0".into(),
            },
            vec![ToolDef {
                name: "echo".into(),
                description: "echo".into(),
                input_schema: json!({"type":"object"}),
            }],
            Echo,
        );
        let listed = server.dispatch("tools/list", None).await.unwrap();
        assert!(listed["tools"].as_array().unwrap().len() == 1);
        let called = server
            .dispatch(
                "tools/call",
                Some(json!({"name":"echo","arguments":{"x":1}})),
            )
            .await
            .unwrap();
        assert_eq!(called["isError"], false);
    }
}
