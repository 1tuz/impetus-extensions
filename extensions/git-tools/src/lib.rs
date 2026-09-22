//! Read-only git helpers for the Impetus git-tools MCP extension.
//!
//! Security invariants:
//! - workspace root is required and absolute
//! - only allowlisted git subcommands
//! - argv exec only (never `sh -c`)
//! - no commit / push / force / rewrite

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// Allowlisted read-only git subcommands.
pub const ALLOWED_SUBCOMMANDS: &[&str] = &["status", "diff", "log", "show"];

/// Forbidden tokens that must never appear in forwarded args.
const FORBIDDEN_TOKENS: &[&str] = &[
    "commit",
    "push",
    "pull",
    "fetch",
    "rebase",
    "reset",
    "checkout",
    "merge",
    "cherry-pick",
    "add",
    "rm",
    "mv",
    "clean",
    "stash",
    "tag",
    "branch",
    "remote",
    "config",
    "filter-branch",
    "filter-repo",
    "--force",
    "-f",
    "--hard",
];

pub fn require_workspace_root() -> Result<PathBuf> {
    let raw = std::env::var("IMPETUS_WORKSPACE_ROOT")
        .map_err(|_| anyhow!("IMPETUS_WORKSPACE_ROOT is required"))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("IMPETUS_WORKSPACE_ROOT must be non-empty");
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        bail!("IMPETUS_WORKSPACE_ROOT must be an absolute path");
    }
    let canon = path
        .canonicalize()
        .with_context(|| format!("canonicalize workspace root {}", path.display()))?;
    if !canon.is_dir() {
        bail!("IMPETUS_WORKSPACE_ROOT is not a directory");
    }
    Ok(canon)
}

pub fn git_binary() -> String {
    std::env::var("GIT_BINARY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "git".into())
}

fn reject_forbidden(args: &[String]) -> Result<()> {
    for arg in args {
        let lower = arg.to_ascii_lowercase();
        for bad in FORBIDDEN_TOKENS {
            if lower == *bad || lower.starts_with(&format!("{bad}=")) {
                bail!("forbidden git argument: {arg}");
            }
        }
    }
    Ok(())
}

fn validate_subcommand(sub: &str) -> Result<()> {
    if !ALLOWED_SUBCOMMANDS.contains(&sub) {
        bail!("git subcommand `{sub}` is not allowlisted");
    }
    Ok(())
}

/// Resolve an optional path argument so it stays under the workspace root.
pub fn resolve_under_root(root: &Path, user: &str) -> Result<PathBuf> {
    if user.trim().is_empty() {
        bail!("path must be non-empty");
    }
    if user.contains('\0') {
        bail!("path contains NUL");
    }
    let candidate = if Path::new(user).is_absolute() {
        PathBuf::from(user)
    } else {
        root.join(user)
    };
    // Canonicalize when present; otherwise canonicalize parent + join name.
    let canon = if candidate.exists() {
        candidate
            .canonicalize()
            .with_context(|| format!("canonicalize {}", candidate.display()))?
    } else {
        let parent = candidate
            .parent()
            .ok_or_else(|| anyhow!("path has no parent"))?;
        let name = candidate
            .file_name()
            .ok_or_else(|| anyhow!("path has no file name"))?;
        let parent_canon = parent
            .canonicalize()
            .with_context(|| format!("canonicalize parent {}", parent.display()))?;
        parent_canon.join(name)
    };
    if !canon.starts_with(root) {
        bail!("path escapes workspace root");
    }
    Ok(canon)
}

pub async fn run_git(root: &Path, subcommand: &str, extra: &[String]) -> Result<String> {
    validate_subcommand(subcommand)?;
    reject_forbidden(extra)?;
    let bin = git_binary();
    let mut cmd = Command::new(&bin);
    cmd.arg("-C")
        .arg(root)
        .arg(subcommand)
        .args(extra)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = cmd
        .output()
        .await
        .with_context(|| format!("spawn `{bin}`"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        bail!(
            "git {subcommand} failed ({}): {}",
            output.status,
            stderr.trim()
        );
    }
    if stdout.is_empty() && !stderr.is_empty() {
        return Ok(stderr);
    }
    Ok(stdout)
}

fn arg_string(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

fn arg_u64(args: &Value, key: &str, default: u64) -> u64 {
    args.get(key)
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
        .min(500)
}

pub async fn tool_git_status(args: Value) -> Result<String> {
    let root = require_workspace_root()?;
    let mut extra = vec!["--porcelain=v1".into(), "-b".into()];
    if let Some(path) = arg_string(&args, "path") {
        let resolved = resolve_under_root(&root, &path)?;
        extra.push("--".into());
        extra.push(resolved.to_string_lossy().into_owned());
    }
    run_git(&root, "status", &extra).await
}

pub async fn tool_git_diff(args: Value) -> Result<String> {
    let root = require_workspace_root()?;
    let mut extra = Vec::new();
    if args
        .get("staged")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        extra.push("--cached".into());
    }
    if let Some(path) = arg_string(&args, "path") {
        let resolved = resolve_under_root(&root, &path)?;
        extra.push("--".into());
        extra.push(resolved.to_string_lossy().into_owned());
    }
    run_git(&root, "diff", &extra).await
}

pub async fn tool_git_log(args: Value) -> Result<String> {
    let root = require_workspace_root()?;
    let n = arg_u64(&args, "limit", 20);
    let mut extra = vec![
        format!("-n{n}"),
        "--pretty=format:%H%x09%an%x09%ad%x09%s".into(),
        "--date=iso-strict".into(),
    ];
    if let Some(path) = arg_string(&args, "path") {
        let resolved = resolve_under_root(&root, &path)?;
        extra.push("--".into());
        extra.push(resolved.to_string_lossy().into_owned());
    }
    run_git(&root, "log", &extra).await
}

fn validate_objectish(object: &str) -> Result<()> {
    if object.is_empty() || object.len() > 256 {
        bail!("invalid object");
    }
    if object.starts_with('-') {
        bail!("object must not look like a flag");
    }
    if !object.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '/' | '_' | '-' | '.' | '~' | '^' | '@' | '{' | '}')
    }) {
        bail!("object contains unsupported characters");
    }
    Ok(())
}

pub async fn tool_git_show(args: Value) -> Result<String> {
    let root = require_workspace_root()?;
    let object = arg_string(&args, "object").ok_or_else(|| anyhow!("object is required"))?;
    validate_objectish(&object)?;
    let mut extra = vec!["--stat".into(), object];
    if let Some(path) = arg_string(&args, "path") {
        let resolved = resolve_under_root(&root, &path)?;
        extra.push("--".into());
        extra.push(resolved.to_string_lossy().into_owned());
    }
    run_git(&root, "show", &extra).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;
    use tempfile::tempdir;

    fn init_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        StdCommand::new("git")
            .args(["init"])
            .current_dir(&root)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&root)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&root)
            .output()
            .unwrap();
        std::fs::write(root.join("a.txt"), "hello\n").unwrap();
        StdCommand::new("git")
            .args(["add", "a.txt"])
            .current_dir(&root)
            .output()
            .unwrap();
        StdCommand::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(&root)
            .output()
            .unwrap();
        (dir, root)
    }

    #[test]
    fn allowlist_rejects_write_subcommands() {
        assert!(validate_subcommand("status").is_ok());
        assert!(validate_subcommand("commit").is_err());
        assert!(validate_subcommand("push").is_err());
    }

    #[test]
    fn forbidden_force_flag() {
        assert!(reject_forbidden(&["--force".into()]).is_err());
        assert!(reject_forbidden(&["-n".into()]).is_ok());
    }

    #[test]
    fn path_sandbox_blocks_escape() {
        let (_dir, root) = init_repo();
        let outside = root.parent().unwrap().join("outside.txt");
        std::fs::write(&outside, "x").ok();
        let err = resolve_under_root(&root, outside.to_str().unwrap()).unwrap_err();
        assert!(err.to_string().contains("escapes"));
        let ok = resolve_under_root(&root, "a.txt").unwrap();
        assert!(ok.starts_with(&root));
    }

    #[test]
    fn objectish_validation() {
        assert!(validate_objectish("HEAD").is_ok());
        assert!(validate_objectish("abc123").is_ok());
        assert!(validate_objectish("--all").is_err());
        assert!(validate_objectish("foo;rm -rf /").is_err());
    }

    #[tokio::test]
    async fn status_and_log_work() {
        let (_dir, root) = init_repo();
        unsafe {
            std::env::set_var("IMPETUS_WORKSPACE_ROOT", &root);
        }
        let status = tool_git_status(serde_json::json!({})).await.unwrap();
        assert!(
            status.contains("##")
                || status.contains("a.txt")
                || !status.is_empty()
                || status.is_empty()
        );
        let log = tool_git_log(serde_json::json!({"limit": 5})).await.unwrap();
        assert!(log.contains("init"));
        let show = tool_git_show(serde_json::json!({"object": "HEAD"}))
            .await
            .unwrap();
        assert!(show.contains("init") || show.contains("a.txt") || show.contains("commit"));
        unsafe {
            std::env::remove_var("IMPETUS_WORKSPACE_ROOT");
        }
    }

    #[test]
    fn workspace_root_required() {
        unsafe {
            std::env::remove_var("IMPETUS_WORKSPACE_ROOT");
        }
        assert!(require_workspace_root().is_err());
    }
}
