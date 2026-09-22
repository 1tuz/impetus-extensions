//! MCP tool wiring for the browser extension.

use crate::backend::{Backend, BackendError, SharedBackend};
use crate::protocol::*;
use anyhow::Result;
use impetus_ext_mcp::{ToolCall, ToolDef, ToolHandler, ToolResult};
use serde_json::{Value, json};

pub fn tool_defs() -> Vec<ToolDef> {
    let obj = json!({"type": "object", "additionalProperties": true});
    [
        ("browser_status", "Provider health/status (protocol 0.1)"),
        ("browser_negotiate", "Negotiate protocol 0.1 capabilities"),
        (
            "browser_ensure_session",
            "Create or reuse a browser session",
        ),
        ("browser_close_session", "Close a session"),
        ("browser_navigate", "Navigate session to http(s) URL"),
        ("browser_page_state", "Current page URL/title/id"),
        ("browser_fetch_rendered", "Fetch rendered readable text"),
        ("browser_click", "Click selector (best-effort)"),
        ("browser_type", "Type into selector (best-effort)"),
        ("browser_wait", "Wait for condition (best-effort)"),
        ("browser_cancel", "Cancel in-flight work / cleanup"),
    ]
    .into_iter()
    .map(|(name, description)| ToolDef {
        name: name.into(),
        description: description.into(),
        input_schema: obj.clone(),
    })
    .collect()
}

pub struct BrowserToolHandler {
    pub backend: SharedBackend,
}

pub fn build_handler(backend: SharedBackend) -> BrowserToolHandler {
    BrowserToolHandler { backend }
}

/// Shared mock backend factory for tests and main.
pub fn mock_shared() -> SharedBackend {
    Backend::mock()
}

impl ToolHandler for BrowserToolHandler {
    fn call<'a>(
        &'a self,
        call: ToolCall<'a>,
    ) -> impetus_ext_mcp::BoxFuture<'a, Result<ToolResult>> {
        Box::pin(async move {
            match call.name {
                "browser_status" => ok_json(serde_json::to_value(self.backend.status().await)?),
                "browser_negotiate" => {
                    let req: BrowserNegotiateRequest = parse_args(call.arguments)?;
                    map_result(self.backend.negotiate(req).await)
                }
                "browser_ensure_session" => {
                    let req: BrowserSessionEnsureRequest = parse_args(call.arguments)?;
                    map_result(self.backend.ensure_session(req).await)
                }
                "browser_close_session" => {
                    let sid = req_str(&call.arguments, "session_id")?;
                    map_unit(
                        self.backend.close_session(&sid).await,
                        json!({"closed": true, "session_id": sid}),
                    )
                }
                "browser_navigate" => {
                    let req: BrowserNavigateRequest = parse_args(call.arguments)?;
                    map_result(self.backend.navigate(req).await)
                }
                "browser_page_state" => {
                    let sid = req_str(&call.arguments, "session_id")?;
                    let page_id = call.arguments.get("page_id").and_then(|v| v.as_str());
                    map_result(self.backend.page_state(&sid, page_id).await)
                }
                "browser_fetch_rendered" => {
                    let req: BrowserFetchRenderedRequest = parse_args(call.arguments)?;
                    map_result(self.backend.fetch_rendered(req).await)
                }
                "browser_click" => {
                    let req: BrowserActionRequest = parse_args(call.arguments)?;
                    map_result(self.backend.click(req).await)
                }
                "browser_type" => {
                    let req: BrowserActionRequest = parse_args(call.arguments)?;
                    map_result(self.backend.type_text(req).await)
                }
                "browser_wait" => {
                    let req: BrowserActionRequest = parse_args(call.arguments)?;
                    map_result(self.backend.wait(req).await)
                }
                "browser_cancel" => {
                    let sid = call.arguments.get("session_id").and_then(|v| v.as_str());
                    map_result(self.backend.cancel(sid).await)
                }
                other => Ok(ToolResult::err(format!("unknown tool `{other}`"))),
            }
        })
    }
}

fn parse_args<T: serde::de::DeserializeOwned>(args: Value) -> Result<T> {
    Ok(serde_json::from_value(args)?)
}

fn req_str(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("missing `{key}`"))
}

fn ok_json(v: Value) -> Result<ToolResult> {
    Ok(ToolResult::ok(serde_json::to_string_pretty(&v)?))
}

fn map_result<T: serde::Serialize>(r: Result<T, BackendError>) -> Result<ToolResult> {
    match r {
        Ok(v) => ok_json(serde_json::to_value(v)?),
        Err(e) => Ok(ToolResult::err(e.to_string())),
    }
}

fn map_unit(r: Result<(), BackendError>, ok: Value) -> Result<ToolResult> {
    match r {
        Ok(()) => ok_json(ok),
        Err(e) => Ok(ToolResult::err(e.to_string())),
    }
}
