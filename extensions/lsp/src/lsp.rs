use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

const ALLOWED_METHODS: &[&str] = &[
    "textDocument/hover",
    "textDocument/definition",
    "textDocument/references",
    "textDocument/documentSymbol",
    "workspace/symbol",
    "textDocument/diagnostic",
    "initialize",
    "shutdown",
];

#[async_trait::async_trait]
pub trait LspSession: Send {
    async fn stop(&mut self) -> Result<()>;
    async fn restart(&mut self) -> Result<()>;
    async fn diagnostics(&self, uri: Option<&str>) -> Result<Value>;
    async fn request(&self, method: &str, params: Value) -> Result<Value>;
    async fn convenience(&self, tool: &str, args: Value) -> Result<Value>;
    async fn cancel(&self) -> Result<()>;
}

pub struct MockLsp {
    command: String,
    workspace: String,
    cancelled: Mutex<bool>,
}

impl MockLsp {
    pub async fn start(command: &str, _args: Vec<String>, workspace: &str) -> Result<Self> {
        Ok(Self {
            command: command.into(),
            workspace: workspace.into(),
            cancelled: Mutex::new(false),
        })
    }
}

#[async_trait::async_trait]
impl LspSession for MockLsp {
    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    async fn restart(&mut self) -> Result<()> {
        *self.cancelled.lock().await = false;
        Ok(())
    }

    async fn diagnostics(&self, uri: Option<&str>) -> Result<Value> {
        if *self.cancelled.lock().await {
            bail!("cancelled");
        }
        Ok(json!({
            "items": [{
                "uri": uri.unwrap_or("file:///mock.rs"),
                "severity": 2,
                "message": "mock diagnostic from impetus-ext-lsp",
                "range": {"start":{"line":0,"character":0},"end":{"line":0,"character":1}}
            }],
            "command": self.command,
            "workspace": self.workspace
        }))
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value> {
        if !ALLOWED_METHODS.contains(&method) {
            bail!("method `{method}` not allowlisted");
        }
        Ok(json!({"method": method, "params": params, "mock": true}))
    }

    async fn convenience(&self, tool: &str, args: Value) -> Result<Value> {
        let method = match tool {
            "lsp_hover" => "textDocument/hover",
            "lsp_definition" => "textDocument/definition",
            "lsp_references" => "textDocument/references",
            "lsp_symbols" => "textDocument/documentSymbol",
            other => bail!("unknown convenience {other}"),
        };
        self.request(method, args).await
    }

    async fn cancel(&self) -> Result<()> {
        *self.cancelled.lock().await = true;
        Ok(())
    }
}

pub struct ProcessLsp {
    child: Child,
    stdin: Mutex<ChildStdin>,
    stdout: Mutex<BufReader<ChildStdout>>,
    command: String,
    args: Vec<String>,
    workspace: String,
    next_id: Mutex<u64>,
    cancelled: Mutex<bool>,
}

impl ProcessLsp {
    pub async fn start(command: &str, args: Vec<String>, workspace: &str) -> Result<Self> {
        if command.chars().any(|c| "|&;<>$`(){}[]".contains(c)) {
            bail!("invalid language server command");
        }
        let mut child = Command::new(command)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("no stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("no stdout"))?;
        let session = Self {
            child,
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            command: command.into(),
            args,
            workspace: workspace.into(),
            next_id: Mutex::new(1),
            cancelled: Mutex::new(false),
        };
        let root = std::fs::canonicalize(workspace)?;
        let init = json!({
            "processId": null,
            "rootUri": format!("file://{}", root.display()),
            "capabilities": {},
            "workspaceFolders": null
        });
        let _ = session.rpc("initialize", init).await?;
        session.notify("initialized", json!({})).await?;
        Ok(session)
    }

    async fn rpc(&self, method: &str, params: Value) -> Result<Value> {
        if *self.cancelled.lock().await {
            bail!("cancelled");
        }
        let id = {
            let mut n = self.next_id.lock().await;
            let id = *n;
            *n += 1;
            id
        };
        let msg = json!({"jsonrpc":"2.0","id": id, "method": method, "params": params});
        self.write_msg(&msg).await?;
        loop {
            let resp = self.read_msg().await?;
            if resp.get("id") == Some(&json!(id)) {
                if let Some(err) = resp.get("error") {
                    bail!("lsp error: {err}");
                }
                return Ok(resp.get("result").cloned().unwrap_or(json!(null)));
            }
        }
    }

    async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let msg = json!({"jsonrpc":"2.0","method": method, "params": params});
        self.write_msg(&msg).await
    }

    async fn write_msg(&self, msg: &Value) -> Result<()> {
        let body = serde_json::to_vec(msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(&body).await?;
        stdin.flush().await?;
        Ok(())
    }

    async fn read_msg(&self) -> Result<Value> {
        let mut stdout = self.stdout.lock().await;
        let mut content_length = None;
        loop {
            let mut line = String::new();
            stdout.read_line(&mut line).await?;
            if line == "\r\n" || line == "\n" {
                break;
            }
            let lower = line.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length:") {
                content_length = Some(rest.trim().parse::<usize>()?);
            }
        }
        let len = content_length.ok_or_else(|| anyhow::anyhow!("missing Content-Length"))?;
        let mut buf = vec![0u8; len];
        stdout.read_exact(&mut buf).await?;
        Ok(serde_json::from_slice(&buf)?)
    }
}

#[async_trait::async_trait]
impl LspSession for ProcessLsp {
    async fn stop(&mut self) -> Result<()> {
        let _ = self.rpc("shutdown", json!(null)).await;
        let _ = self.notify("exit", json!(null)).await;
        let _ = self.child.kill().await;
        Ok(())
    }

    async fn restart(&mut self) -> Result<()> {
        let command = self.command.clone();
        let args = self.args.clone();
        let workspace = self.workspace.clone();
        self.stop().await.ok();
        *self = Self::start(&command, args, &workspace).await?;
        Ok(())
    }

    async fn diagnostics(&self, uri: Option<&str>) -> Result<Value> {
        Ok(json!({
            "items": [],
            "note": "pull diagnostics via lsp_request textDocument/diagnostic when supported",
            "uri": uri
        }))
    }

    async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let allow: HashSet<&str> = ALLOWED_METHODS.iter().copied().collect();
        if !allow.contains(method) {
            bail!("method `{method}` not allowlisted");
        }
        self.rpc(method, params).await
    }

    async fn convenience(&self, tool: &str, args: Value) -> Result<Value> {
        let method = match tool {
            "lsp_hover" => "textDocument/hover",
            "lsp_definition" => "textDocument/definition",
            "lsp_references" => "textDocument/references",
            "lsp_symbols" => "textDocument/documentSymbol",
            other => bail!("unknown convenience {other}"),
        };
        self.request(method, args).await
    }

    async fn cancel(&self) -> Result<()> {
        *self.cancelled.lock().await = true;
        Ok(())
    }
}
