//! One long-lived LSP session (real child process or in-process `--mock`).

use crate::allowlist::is_allowed_request;
use crate::framing::{read_message, write_message};
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncWrite, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock, oneshot};
use tokio::task::JoinHandle;
use tokio::time::timeout;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

type DynStdin = Box<dyn AsyncWrite + Unpin + Send>;

#[derive(Debug, Clone)]
pub struct StartConfig {
    pub command: String,
    pub args: Vec<String>,
    pub workspace: PathBuf,
    pub mock: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionInfo {
    pub running: bool,
    pub mock: bool,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub workspace: Option<String>,
}

struct PendingMap {
    map: HashMap<u64, oneshot::Sender<Result<Value>>>,
}

struct LiveSession {
    child: Mutex<Option<Child>>,
    stdin: Mutex<DynStdin>,
    next_id: AtomicU64,
    pending: Arc<Mutex<PendingMap>>,
    diagnostics: Arc<RwLock<HashMap<String, Value>>>,
    reader: JoinHandle<()>,
    command: String,
    args: Vec<String>,
    workspace: PathBuf,
    mock: bool,
}

pub struct LspSession {
    inner: Mutex<Option<Arc<LiveSession>>>,
    default_command: Option<String>,
    default_args: Vec<String>,
    default_workspace: Option<PathBuf>,
    prefer_mock: bool,
}

impl LspSession {
    pub fn new(
        prefer_mock: bool,
        default_command: Option<String>,
        default_args: Vec<String>,
        default_workspace: Option<PathBuf>,
    ) -> Self {
        Self {
            inner: Mutex::new(None),
            default_command,
            default_args,
            default_workspace,
            prefer_mock,
        }
    }

    pub async fn info(&self) -> SessionInfo {
        let guard = self.inner.lock().await;
        match guard.as_ref() {
            Some(s) => SessionInfo {
                running: true,
                mock: s.mock,
                command: Some(s.command.clone()),
                args: s.args.clone(),
                workspace: Some(s.workspace.display().to_string()),
            },
            None => SessionInfo {
                running: false,
                mock: self.prefer_mock,
                command: self.default_command.clone(),
                args: self.default_args.clone(),
                workspace: self
                    .default_workspace
                    .as_ref()
                    .map(|p| p.display().to_string()),
            },
        }
    }

    pub async fn start(&self, mut cfg: StartConfig) -> Result<Value> {
        if cfg.command.is_empty() {
            if let Some(c) = &self.default_command {
                cfg.command = c.clone();
            }
        }
        if cfg.args.is_empty() && !self.default_args.is_empty() {
            cfg.args = self.default_args.clone();
        }
        if cfg.workspace.as_os_str().is_empty() {
            if let Some(w) = &self.default_workspace {
                cfg.workspace = w.clone();
            } else {
                cfg.workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            }
        }
        if self.prefer_mock {
            cfg.mock = true;
        }
        if !cfg.mock && cfg.command.is_empty() {
            bail!("lsp_start: command required (tool arg, --command, or IMPETUS_LSP_COMMAND)");
        }

        let mut guard = self.inner.lock().await;
        if guard.is_some() {
            bail!("LSP session already running; call lsp_stop or lsp_restart");
        }
        let live = if cfg.mock {
            spawn_mock(&cfg).await?
        } else {
            spawn_real(&cfg).await?
        };
        let live = Arc::new(live);
        let init = handshake(&live).await?;
        let command_out = if cfg.mock {
            "mock-ls".to_string()
        } else {
            cfg.command.clone()
        };
        *guard = Some(live);
        Ok(json!({
            "started": true,
            "mock": cfg.mock,
            "command": command_out,
            "args": cfg.args,
            "workspace": cfg.workspace.display().to_string(),
            "initialize": init,
        }))
    }

    pub async fn stop(&self) -> Result<Value> {
        let mut guard = self.inner.lock().await;
        if let Some(live) = guard.take() {
            shutdown_session(live).await;
            Ok(json!({ "stopped": true }))
        } else {
            Ok(json!({ "stopped": false, "reason": "not running" }))
        }
    }

    pub async fn restart(&self, cfg: Option<StartConfig>) -> Result<Value> {
        let snapshot = {
            let guard = self.inner.lock().await;
            guard.as_ref().map(|s| StartConfig {
                command: s.command.clone(),
                args: s.args.clone(),
                workspace: s.workspace.clone(),
                mock: s.mock,
            })
        };
        let _ = self.stop().await?;
        let cfg = cfg
            .or(snapshot)
            .ok_or_else(|| anyhow!("lsp_restart: no prior session and no start config provided"))?;
        self.start(cfg).await
    }

    pub async fn diagnostics(&self, uri: Option<&str>) -> Result<Value> {
        let live = self.require().await?;
        let map = live.diagnostics.read().await;
        if let Some(uri) = uri {
            Ok(json!({
                "uri": uri,
                "diagnostics": map.get(uri).cloned().unwrap_or_else(|| json!([])),
            }))
        } else {
            let obj: serde_json::Map<String, Value> =
                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            Ok(json!({ "diagnostics": obj }))
        }
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        if !is_allowed_request(method) {
            bail!(
                "method `{method}` not on allowlist; use convenience tools or allowed LSP methods"
            );
        }
        let live = self.require().await?;
        request(&live, method, params).await
    }

    pub async fn cancel(&self, id: u64) -> Result<Value> {
        let live = self.require().await?;
        {
            let mut pending = live.pending.lock().await;
            if let Some(tx) = pending.map.remove(&id) {
                let _ = tx.send(Err(anyhow!("cancelled")));
            }
        }
        notify(&live, "$/cancelRequest", json!({ "id": id })).await?;
        Ok(json!({ "cancelled": id }))
    }

    pub async fn hover(&self, uri: &str, line: u32, character: u32) -> Result<Value> {
        let live = self.require().await?;
        request(
            &live,
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
        .await
    }

    pub async fn definition(&self, uri: &str, line: u32, character: u32) -> Result<Value> {
        let live = self.require().await?;
        request(
            &live,
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
        .await
    }

    pub async fn references(&self, uri: &str, line: u32, character: u32) -> Result<Value> {
        let live = self.require().await?;
        request(
            &live,
            "textDocument/references",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": true }
            }),
        )
        .await
    }

    pub async fn symbols(&self, uri: Option<&str>, query: Option<&str>) -> Result<Value> {
        let live = self.require().await?;
        if let Some(uri) = uri {
            request(
                &live,
                "textDocument/documentSymbol",
                json!({ "textDocument": { "uri": uri } }),
            )
            .await
        } else {
            request(
                &live,
                "workspace/symbol",
                json!({ "query": query.unwrap_or("") }),
            )
            .await
        }
    }

    async fn require(&self) -> Result<Arc<LiveSession>> {
        self.inner
            .lock()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("LSP not started; call lsp_start first"))
    }
}

async fn spawn_real(cfg: &StartConfig) -> Result<LiveSession> {
    let mut child = Command::new(&cfg.command)
        .args(&cfg.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("spawn LSP `{}`", cfg.command))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("LSP stdin missing"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("LSP stdout missing"))?;

    let pending = Arc::new(Mutex::new(PendingMap {
        map: HashMap::new(),
    }));
    let diagnostics = Arc::new(RwLock::new(HashMap::new()));
    let reader = spawn_reader(stdout, pending.clone(), diagnostics.clone());

    Ok(LiveSession {
        child: Mutex::new(Some(child)),
        stdin: Mutex::new(Box::new(stdin)),
        next_id: AtomicU64::new(1),
        pending,
        diagnostics,
        reader,
        command: cfg.command.clone(),
        args: cfg.args.clone(),
        workspace: cfg.workspace.clone(),
        mock: false,
    })
}

async fn spawn_mock(cfg: &StartConfig) -> Result<LiveSession> {
    let (client_stdin, server_stdin) = tokio::io::duplex(64 * 1024);
    let (server_stdout, client_stdout) = tokio::io::duplex(64 * 1024);
    let workspace = cfg.workspace.clone();
    tokio::spawn(async move {
        let _ = run_mock_server(server_stdin, server_stdout, workspace).await;
    });

    let pending = Arc::new(Mutex::new(PendingMap {
        map: HashMap::new(),
    }));
    let diagnostics = Arc::new(RwLock::new(HashMap::new()));
    let reader = spawn_reader(client_stdout, pending.clone(), diagnostics.clone());

    Ok(LiveSession {
        child: Mutex::new(None),
        stdin: Mutex::new(Box::new(client_stdin)),
        next_id: AtomicU64::new(1),
        pending,
        diagnostics,
        reader,
        command: "mock-ls".into(),
        args: vec!["--mock".into()],
        workspace: cfg.workspace.clone(),
        mock: true,
    })
}

fn spawn_reader<R>(
    stdout: R,
    pending: Arc<Mutex<PendingMap>>,
    diagnostics: Arc<RwLock<HashMap<String, Value>>>,
) -> JoinHandle<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_message(&mut reader).await {
                Ok(msg) => {
                    if msg.get("method").is_none() {
                        if let Some(id) = msg.get("id").and_then(json_id_u64) {
                            let result = if let Some(err) = msg.get("error") {
                                Err(anyhow!("LSP error: {err}"))
                            } else {
                                Ok(msg.get("result").cloned().unwrap_or(Value::Null))
                            };
                            let mut pend = pending.lock().await;
                            if let Some(tx) = pend.map.remove(&id) {
                                let _ = tx.send(result);
                            }
                            continue;
                        }
                    }
                    if msg.get("method").and_then(|m| m.as_str())
                        == Some("textDocument/publishDiagnostics")
                    {
                        if let Some(params) = msg.get("params") {
                            if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                                let diags = params
                                    .get("diagnostics")
                                    .cloned()
                                    .unwrap_or_else(|| json!([]));
                                diagnostics.write().await.insert(uri.to_string(), diags);
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
    })
}

async fn handshake(live: &LiveSession) -> Result<Value> {
    let root = path_to_uri(&live.workspace);
    let result = request(
        live,
        "initialize",
        json!({
            "processId": std::process::id(),
            "rootUri": root,
            "rootPath": live.workspace.display().to_string(),
            "capabilities": {
                "textDocument": {
                    "hover": { "contentFormat": ["markdown", "plaintext"] },
                    "publishDiagnostics": {},
                    "definition": { "linkSupport": true },
                    "references": {},
                    "documentSymbol": { "hierarchicalDocumentSymbolSupport": true },
                },
                "workspace": {
                    "symbol": {},
                    "workspaceFolders": true
                }
            },
            "workspaceFolders": [{
                "uri": root,
                "name": live.workspace.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("workspace")
            }],
            "clientInfo": { "name": "impetus-ext-lsp", "version": env!("CARGO_PKG_VERSION") }
        }),
    )
    .await?;
    notify(live, "initialized", json!({})).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    Ok(result)
}

async fn request(live: &LiveSession, method: &str, params: Value) -> Result<Value> {
    let id = live.next_id.fetch_add(1, Ordering::SeqCst);
    let (tx, rx) = oneshot::channel();
    {
        let mut pend = live.pending.lock().await;
        pend.map.insert(id, tx);
    }
    let body = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });
    {
        let mut stdin = live.stdin.lock().await;
        write_message(&mut *stdin, &body).await?;
    }
    match timeout(REQUEST_TIMEOUT, rx).await {
        Ok(Ok(res)) => res,
        Ok(Err(_)) => Err(anyhow!("LSP request channel closed for id={id}")),
        Err(_) => {
            let mut pend = live.pending.lock().await;
            pend.map.remove(&id);
            Err(anyhow!("LSP request timeout for `{method}` id={id}"))
        }
    }
}

async fn notify(live: &LiveSession, method: &str, params: Value) -> Result<()> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });
    let mut stdin = live.stdin.lock().await;
    write_message(&mut *stdin, &body).await
}

async fn shutdown_session(live: Arc<LiveSession>) {
    let _ = request(&live, "shutdown", Value::Null).await;
    let _ = notify(&live, "exit", Value::Null).await;
    live.reader.abort();
    if let Some(mut child) = live.child.lock().await.take() {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }
}

fn path_to_uri(path: &Path) -> String {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    format!("file://{}", abs.display())
}

fn json_id_u64(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_i64().map(|i| i as u64))
        .or_else(|| v.as_str()?.parse().ok())
}

/// Mock language server loop (stdio). Used by `--mock-child`.
pub async fn run_mock_child(workspace: PathBuf) -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    run_mock_server(stdin, stdout, workspace).await
}

async fn run_mock_server<R, W>(stdin: R, mut stdout: W, workspace: PathBuf) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(stdin);
    let root_uri = path_to_uri(&workspace);
    let sample_uri = format!("{root_uri}/src/main.rs");

    loop {
        let msg = match read_message(&mut reader).await {
            Ok(m) => m,
            Err(_) => break,
        };
        if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
            let id = msg.get("id").cloned();
            match method {
                "initialize" => {
                    let body = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "capabilities": {
                                "hoverProvider": true,
                                "definitionProvider": true,
                                "referencesProvider": true,
                                "documentSymbolProvider": true,
                                "workspaceSymbolProvider": true,
                                "textDocumentSync": 1
                            },
                            "serverInfo": { "name": "impetus-ext-lsp-mock", "version": "0.1.0" }
                        }
                    });
                    write_message(&mut stdout, &body).await?;
                    let diag = json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/publishDiagnostics",
                        "params": {
                            "uri": sample_uri,
                            "diagnostics": [{
                                "range": {
                                    "start": { "line": 0, "character": 0 },
                                    "end": { "line": 0, "character": 4 }
                                },
                                "severity": 2,
                                "source": "mock-ls",
                                "message": "mock warning: unused import"
                            }]
                        }
                    });
                    write_message(&mut stdout, &diag).await?;
                }
                "initialized" | "exit" | "$/cancelRequest" => {}
                "shutdown" => {
                    if let Some(id) = id {
                        write_message(
                            &mut stdout,
                            &json!({ "jsonrpc": "2.0", "id": id, "result": null }),
                        )
                        .await?;
                    }
                }
                "textDocument/hover" => {
                    respond(
                        &mut stdout,
                        id,
                        json!({
                            "contents": {
                                "kind": "markdown",
                                "value": "```rust\nfn mock()\n```\nMock hover from impetus-ext-lsp."
                            }
                        }),
                    )
                    .await?;
                }
                "textDocument/definition"
                | "textDocument/typeDefinition"
                | "textDocument/implementation" => {
                    respond(
                        &mut stdout,
                        id,
                        json!({
                            "uri": sample_uri,
                            "range": {
                                "start": { "line": 1, "character": 0 },
                                "end": { "line": 1, "character": 3 }
                            }
                        }),
                    )
                    .await?;
                }
                "textDocument/references" => {
                    respond(
                        &mut stdout,
                        id,
                        json!([{
                            "uri": sample_uri,
                            "range": {
                                "start": { "line": 2, "character": 0 },
                                "end": { "line": 2, "character": 4 }
                            }
                        }]),
                    )
                    .await?;
                }
                "textDocument/documentSymbol" => {
                    respond(
                        &mut stdout,
                        id,
                        json!([{
                            "name": "mock",
                            "kind": 12,
                            "range": {
                                "start": { "line": 0, "character": 0 },
                                "end": { "line": 10, "character": 0 }
                            },
                            "selectionRange": {
                                "start": { "line": 0, "character": 3 },
                                "end": { "line": 0, "character": 7 }
                            }
                        }]),
                    )
                    .await?;
                }
                "workspace/symbol" => {
                    respond(
                        &mut stdout,
                        id,
                        json!([{
                            "name": "mock",
                            "kind": 12,
                            "location": {
                                "uri": sample_uri,
                                "range": {
                                    "start": { "line": 0, "character": 0 },
                                    "end": { "line": 0, "character": 4 }
                                }
                            }
                        }]),
                    )
                    .await?;
                }
                other => {
                    if let Some(id) = id {
                        write_message(
                            &mut stdout,
                            &json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": {
                                    "code": -32601,
                                    "message": format!("mock: method not implemented: {other}")
                                }
                            }),
                        )
                        .await?;
                    }
                }
            }
        }
    }
    Ok(())
}

async fn respond<W: AsyncWrite + Unpin>(
    out: &mut W,
    id: Option<Value>,
    result: Value,
) -> Result<()> {
    if let Some(id) = id {
        write_message(
            out,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            }),
        )
        .await?;
    }
    Ok(())
}
