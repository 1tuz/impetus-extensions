use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use impetus_ext_support::{
    ExtensionManifest, SUPPORTED_IMPETUS_TAG, discover_extensions, load_package_meta,
    package_extension,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "impetus-ext",
    about = "Dev helpers for Impetus first-party extensions"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Validate all package.toml + generated/core manifests under extensions/
    ValidateManifests {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Check compatibility pins against supported Impetus tag
    CheckCompat {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Build all workspace extension binaries
    BuildAll,
    /// Test all workspace crates
    TestAll,
    /// Package one extension into dist/
    Package {
        extension: PathBuf,
        #[arg(long, default_value = "dist")]
        out: PathBuf,
    },
    /// Package every extension
    PackageAll {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "dist")]
        out: PathBuf,
    },
    /// Install a packaged skill/MCP into a local Impetus project root via `impetus extension install`
    InstallLocal {
        /// Path to extension source dir (with package.toml) or packaged dist dir
        extension: PathBuf,
        #[arg(long)]
        root: PathBuf,
        /// Also copy MCP json into IMPETUS_DATA_DIR/mcp for daemon SoT
        #[arg(long)]
        daemon_mcp: bool,
    },
    /// Print status / doctor via impetus CLI when available
    Status {
        #[arg(long)]
        root: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::ValidateManifests { root } => validate_manifests(&root),
        Cmd::CheckCompat { root } => check_compat(&root),
        Cmd::BuildAll => run_cargo(&["build", "--workspace"]),
        Cmd::TestAll => run_cargo(&["test", "--workspace"]),
        Cmd::Package { extension, out } => {
            let layout = package_extension(&extension, &out)?;
            println!("{}", layout.root.display());
            Ok(())
        }
        Cmd::PackageAll { root, out } => {
            for ext in discover_extensions(&root)? {
                let layout = package_extension(&ext, &out)?;
                println!("packaged {}", layout.root.display());
            }
            Ok(())
        }
        Cmd::InstallLocal {
            extension,
            root,
            daemon_mcp,
        } => install_local(&extension, &root, daemon_mcp),
        Cmd::Status { root } => status(&root),
    }
}

fn validate_manifests(root: &Path) -> Result<()> {
    let mut ok = 0usize;
    for ext in discover_extensions(root)? {
        let meta = load_package_meta(&ext.join("package.toml"))
            .with_context(|| format!("load {}", ext.join("package.toml").display()))?;
        // Package into a temp dist to ensure core manifest validates.
        let tmp = tempfile::tempdir()?;
        let layout = package_extension(&ext, tmp.path())?;
        let raw = fs::read_to_string(&layout.manifest_path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        ExtensionManifest::from_json_value(&value)
            .with_context(|| format!("manifest for {}", meta.id))?;
        println!("ok {}", meta.id);
        ok += 1;
    }
    println!("validated {ok} extension(s)");
    Ok(())
}

fn check_compat(root: &Path) -> Result<()> {
    let matrix = root.join("compatibility.json");
    let raw =
        fs::read_to_string(&matrix).with_context(|| format!("missing {}", matrix.display()))?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let tag = value
        .get("impetus_tag")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if tag != SUPPORTED_IMPETUS_TAG {
        bail!("compatibility.json impetus_tag={tag}, expected {SUPPORTED_IMPETUS_TAG}");
    }
    for ext in discover_extensions(root)? {
        let meta = load_package_meta(&ext.join("package.toml"))?;
        if meta.compatibility.impetus_tag != SUPPORTED_IMPETUS_TAG {
            bail!(
                "{} pins {}, expected {}",
                meta.id,
                meta.compatibility.impetus_tag,
                SUPPORTED_IMPETUS_TAG
            );
        }
        println!(
            "compat {} api={}",
            meta.id, meta.compatibility.extension_api_version
        );
    }
    Ok(())
}

fn run_cargo(args: &[&str]) -> Result<()> {
    let status = Command::new("cargo").args(args).status()?;
    if !status.success() {
        bail!("cargo {:?} failed", args);
    }
    Ok(())
}

fn install_local(extension: &Path, project_root: &Path, daemon_mcp: bool) -> Result<()> {
    let meta_path = if extension.join("package.toml").exists() {
        extension.join("package.toml")
    } else {
        bail!("expected package.toml under {}", extension.display());
    };
    let meta = load_package_meta(&meta_path)?;
    match meta.kind {
        impetus_ext_support::PackageKind::Skill => {
            let skill = extension.join(
                meta.entrypoints
                    .skill_md
                    .unwrap_or_else(|| "SKILL.md".into()),
            );
            let skill_dir = if skill.file_name().and_then(|s| s.to_str()) == Some("SKILL.md") {
                skill.parent().unwrap().to_path_buf()
            } else {
                skill
            };
            run_impetus(&[
                "extension",
                "install",
                "--kind",
                "skill",
                skill_dir.to_str().unwrap(),
                "--root",
                project_root.to_str().unwrap(),
            ])?;
        }
        impetus_ext_support::PackageKind::McpConfig => {
            let mcp = extension.join(
                meta.entrypoints
                    .mcp_json
                    .clone()
                    .unwrap_or_else(|| "mcp.json".into()),
            );
            run_impetus(&[
                "extension",
                "install",
                "--kind",
                "mcp",
                mcp.to_str().unwrap(),
                "--root",
                project_root.to_str().unwrap(),
            ])?;
            if daemon_mcp {
                let data = std::env::var("IMPETUS_DATA_DIR")
                    .unwrap_or_else(|_| default_impetus_data_dir());
                let dest_dir = PathBuf::from(data).join("mcp");
                fs::create_dir_all(&dest_dir)?;
                let dest = dest_dir.join(format!("{}.json", meta.id));
                fs::copy(&mcp, &dest)?;
                println!("copied daemon MCP SoT -> {}", dest.display());
            }
        }
    }
    Ok(())
}

fn status(root: &Path) -> Result<()> {
    run_impetus(&[
        "extension",
        "list",
        "--root",
        root.to_str().unwrap(),
        "--json",
    ])
}

fn run_impetus(args: &[&str]) -> Result<()> {
    let status = Command::new("impetus").args(args).status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => bail!("impetus {:?} exited {}", args, s),
        Err(err) => bail!("impetus CLI not available: {err}"),
    }
}

fn default_impetus_data_dir() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("{home}/.local/share/impetus")
}
