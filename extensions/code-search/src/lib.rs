//! Workspace-sandboxed text search for Impetus MCP.

use anyhow::{Context, Result, anyhow, bail};
use impetus_ext_mcp::ToolResult;
use regex::RegexBuilder;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use walkdir::WalkDir;

const DEFAULT_MAX_MATCHES: usize = 200;
const HARD_MAX_MATCHES: usize = 2000;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

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

/// Canonicalize `user` and require it stays under `root` (blocks symlink escape).
pub fn resolve_under_root(root: &Path, user: &str) -> Result<PathBuf> {
    if user.contains('\0') {
        bail!("path contains NUL");
    }
    let candidate = if user.is_empty() || user == "." {
        root.to_path_buf()
    } else if Path::new(user).is_absolute() {
        PathBuf::from(user)
    } else {
        root.join(user)
    };
    let canon = candidate
        .canonicalize()
        .with_context(|| format!("canonicalize {}", candidate.display()))?;
    if !canon.starts_with(root) {
        bail!("path escapes workspace root");
    }
    Ok(canon)
}

async fn ripgrep_available() -> bool {
    Command::new("rg")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn search_with_rg(
    root: &Path,
    pattern: &str,
    path: &Path,
    case_insensitive: bool,
    max_matches: usize,
) -> Result<String> {
    let mut cmd = Command::new("rg");
    cmd.arg("--line-number")
        .arg("--no-heading")
        .arg("--color")
        .arg("never")
        .arg("--max-count")
        .arg(max_matches.to_string());
    if case_insensitive {
        cmd.arg("-i");
    }
    cmd.arg("--")
        .arg(pattern)
        .arg(path)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let output = cmd.output().await.context("spawn rg")?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        let code = output.status.code();
        if code != Some(1) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("rg failed ({}): {}", output.status, stderr.trim());
        }
    }
    filter_rg_output(root, &stdout)
}

fn filter_rg_output(root: &Path, raw: &str) -> Result<String> {
    let mut out = String::new();
    for line in raw.lines() {
        let Some((path_part, rest)) = line.split_once(':') else {
            continue;
        };
        let candidate = if Path::new(path_part).is_absolute() {
            PathBuf::from(path_part)
        } else {
            root.join(path_part)
        };
        let Ok(canon) = candidate.canonicalize() else {
            continue;
        };
        if !canon.starts_with(root) {
            continue;
        }
        let rel = canon.strip_prefix(root).unwrap_or(&canon);
        out.push_str(&format!("{}:{}\n", rel.display(), rest));
    }
    Ok(out)
}

fn search_with_walk(
    root: &Path,
    pattern: &str,
    path: &Path,
    case_insensitive: bool,
    max_matches: usize,
) -> Result<String> {
    let re = RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()
        .map_err(|e| anyhow!("invalid regex: {e}"))?;
    let mut out = String::new();
    let mut matches = 0usize;
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(canon) = entry.path().canonicalize() else {
            continue;
        };
        if !canon.starts_with(root) {
            continue;
        }
        let meta = match fs::metadata(&canon) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > MAX_FILE_BYTES {
            continue;
        }
        let Ok(text) = fs::read_to_string(&canon) else {
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            if re.is_match(line) {
                let rel = canon.strip_prefix(root).unwrap_or(&canon);
                out.push_str(&format!("{}:{}:{}\n", rel.display(), idx + 1, line));
                matches += 1;
                if matches >= max_matches {
                    return Ok(out);
                }
            }
        }
    }
    Ok(out)
}

pub async fn search_text(args: Value) -> Result<String> {
    let root = require_workspace_root()?;
    let pattern = args
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("pattern is required"))?;
    if pattern.is_empty() {
        bail!("pattern must be non-empty");
    }
    if pattern.len() > 512 {
        bail!("pattern too long");
    }
    let path_arg = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let search_root = resolve_under_root(&root, path_arg)?;
    let case_insensitive = args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let max_matches = args
        .get("max_matches")
        .or_else(|| args.get("max_results"))
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_MAX_MATCHES as u64)
        .clamp(1, HARD_MAX_MATCHES as u64) as usize;

    let raw = if ripgrep_available().await {
        search_with_rg(&root, pattern, &search_root, case_insensitive, max_matches).await?
    } else {
        search_with_walk(&root, pattern, &search_root, case_insensitive, max_matches)?
    };
    Ok(if raw.is_empty() {
        "(no matches)".into()
    } else {
        raw
    })
}

pub async fn tool_search_text(args: Value) -> Result<ToolResult> {
    match search_text(args).await {
        Ok(t) => Ok(ToolResult::ok(t)),
        Err(e) => Ok(ToolResult::err(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn setup_tree() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn hello() {}\nfn world() {}\n").unwrap();
        fs::write(root.join("README.md"), "hello workspace\n").unwrap();
        (dir, root)
    }

    #[test]
    fn rejects_path_traversal() {
        let (_d, root) = setup_tree();
        let err = resolve_under_root(&root, "../").unwrap_err();
        assert!(
            err.to_string().contains("escapes") || err.to_string().contains("canonicalize"),
            "{err}"
        );
    }

    #[test]
    fn rejects_symlink_escape() {
        let (_d, root) = setup_tree();
        let outside = root.parent().unwrap().join("secret-code-search.txt");
        fs::write(&outside, "SECRET").unwrap();
        let link = root.join("leak");
        symlink(&outside, &link).unwrap();
        let err = resolve_under_root(&root, "leak").unwrap_err();
        assert!(err.to_string().contains("escapes"), "{err}");
    }

    #[test]
    fn accepts_inner_path() {
        let (_d, root) = setup_tree();
        let p = resolve_under_root(&root, "src").unwrap();
        assert!(p.starts_with(&root));
    }

    #[tokio::test]
    async fn finds_pattern() {
        let (_d, root) = setup_tree();
        // Scope env mutation to this test; Rust 1.98 treats set_var as unsafe.
        unsafe {
            std::env::set_var("IMPETUS_WORKSPACE_ROOT", &root);
        }
        let out = search_text(serde_json::json!({
            "pattern": "hello",
            "path": "."
        }))
        .await
        .unwrap();
        assert!(out.contains("hello"), "output was: {out}");
        unsafe {
            std::env::remove_var("IMPETUS_WORKSPACE_ROOT");
        }
    }

    #[test]
    fn walk_fallback_matches() {
        let (_d, root) = setup_tree();
        let out = search_with_walk(&root, "world", &root, false, 10).unwrap();
        assert!(out.contains("world"));
    }
}
