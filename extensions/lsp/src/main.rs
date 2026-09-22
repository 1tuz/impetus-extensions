use anyhow::Result;
use clap::Parser;
use impetus_ext_lsp::session::{LspSession, run_mock_child};
use impetus_ext_lsp::{build_server, defaults_from_env, parse_args_list};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "impetus-ext-lsp",
    about = "Generic LSP MCP stdio server for Impetus"
)]
struct Cli {
    /// Fake language server (initialize + sample diagnostics). For CI without rust-analyzer.
    #[arg(long)]
    mock: bool,

    /// Internal: act as the mock language server child (Content-Length stdio).
    #[arg(long, hide = true)]
    mock_child: bool,

    /// Default language server binary (overridden by lsp_start / IMPETUS_LSP_COMMAND).
    #[arg(long)]
    command: Option<String>,

    /// Default language server args (JSON array or whitespace-separated).
    #[arg(long)]
    args: Option<String>,

    /// Default workspace root.
    #[arg(long)]
    workspace: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    if cli.mock_child {
        let workspace = cli
            .workspace
            .or_else(|| {
                std::env::var("IMPETUS_LSP_WORKSPACE")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        return run_mock_child(workspace).await;
    }

    let (env_cmd, env_args, env_ws) = defaults_from_env();
    let command = cli.command.or(env_cmd);
    let args = cli
        .args
        .as_deref()
        .map(|s| parse_args_list(Some(s)))
        .filter(|v| !v.is_empty())
        .unwrap_or(env_args);
    let workspace = cli.workspace.or(env_ws);

    let session = Arc::new(LspSession::new(cli.mock, command, args, workspace));
    let server = build_server(session);
    server.serve_stdio().await
}
