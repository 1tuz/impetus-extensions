use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use impetus_ext_support::{
    ExtensionEntrypoint, SUPPORTED_IMPETUS_TAG, check_compatibility_file, discover_extensions,
    load_extension_dir, package_extension, sync_catalog_from_manifests, validate_all_extensions,
    validate_catalog,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "impetus-ext",
    about = "Dev helpers for Impetus first-party extensions (canonical extension.toml)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Validate extension.toml + entrypoint artifacts under extensions/
    ValidateManifests {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Validate catalog.json against canonical manifests
    ValidateCatalog {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Rewrite catalog id/name/version/entrypoint from extension.toml
    SyncCatalog {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        write: bool,
    },
    /// Check compatibility.json SDK + legacy helper pins
    CheckCompat {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Build all workspace extension binaries
    BuildAll,
    /// Test all workspace crates
    TestAll,
    /// Package one extension into dist/ (legacy Skill/MCP layout derived from extension.toml)
    Package {
        extension: PathBuf,
        #[arg(long, default_value = "dist")]
        out: PathBuf,
    },
    /// Package every packagable extension (instruction_pack + mcp_bridge)
    PackageAll {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "dist")]
        out: PathBuf,
    },
    /// Install a packaged skill/MCP into a local Impetus project via legacy CLI
    InstallLocal {
        /// Path to extension source dir (with extension.toml) or packaged dist dir
        extension: PathBuf,
        #[arg(long)]
        root: PathBuf,
        /// Also copy MCP json into IMPETUS_DATA_DIR/mcp for daemon SoT
        #[arg(long)]
        daemon_mcp: bool,
    },
    /// Print status via impetus CLI when available
    Status {
        #[arg(long)]
        root: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::ValidateManifests { root } => validate_manifests(&root),
        Cmd::ValidateCatalog { root } => {
            let catalog = validate_catalog(&root)?;
            println!("catalog ok: {} extension(s)", catalog.extensions.len());
            Ok(())
        }
        Cmd::SyncCatalog { root, write } => {
            let catalog = sync_catalog_from_manifests(&root)?;
            let pretty = serde_json::to_string_pretty(&catalog)? + "\n";
            if write {
                fs::write(root.join("catalog.json"), &pretty)?;
                println!("wrote catalog.json ({} entries)", catalog.extensions.len());
            } else {
                print!("{pretty}");
            }
            Ok(())
        }
        Cmd::CheckCompat { root } => {
            check_compatibility_file(&root)?;
            println!("compat ok (legacy tag {SUPPORTED_IMPETUS_TAG})");
            Ok(())
        }
        Cmd::BuildAll => run_cargo(&["build", "--workspace"]),
        Cmd::TestAll => run_cargo(&["test", "--workspace"]),
        Cmd::Package { extension, out } => {
            let layout = package_extension(&extension, &out)?;
            println!("{}", layout.root.display());
            Ok(())
        }
        Cmd::PackageAll { root, out } => {
            for ext in discover_extensions(&root)? {
                let manifest = load_extension_dir(&ext)?;
                if matches!(manifest.entrypoint, ExtensionEntrypoint::HostProcess { .. }) {
                    println!("skip host_process {}", manifest.id.as_str());
                    continue;
                }
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
    let manifests = validate_all_extensions(root)?;
    for m in &manifests {
        println!(
            "ok {} ({})",
            m.id.as_str(),
            match &m.entrypoint {
                ExtensionEntrypoint::InstructionPack { .. } => "instruction_pack",
                ExtensionEntrypoint::McpBridge { .. } => "mcp_bridge",
                ExtensionEntrypoint::HostProcess { .. } => "host_process",
            }
        );
    }
    println!("validated {} extension(s)", manifests.len());
    // Catalog is part of the fast gate.
    let catalog = validate_catalog(root)?;
    println!("catalog ok: {} entry(ies)", catalog.extensions.len());
    Ok(())
}

fn install_local(extension: &Path, project_root: &Path, daemon_mcp: bool) -> Result<()> {
    let packaged =
        if extension.join("manifest.json").is_file() && extension.join("package.toml").is_file() {
            extension.to_path_buf()
        } else if extension.join("extension.toml").is_file() {
            let tmp = tempfile::tempdir()?;
            let layout = package_extension(extension, tmp.path())?;
            // Keep packaged tree for install; move into project-adjacent temp owned by us.
            let keep = project_root.join(".impetus-ext-staging");
            if keep.exists() {
                fs::remove_dir_all(&keep)?;
            }
            fs::create_dir_all(&keep)?;
            let dest = keep.join(
                layout
                    .root
                    .file_name()
                    .context("package layout missing name")?,
            );
            copy_dir(&layout.root, &dest)?;
            dest
        } else {
            bail!(
                "expected extension.toml or packaged dist under {}",
                extension.display()
            );
        };

    let meta_raw = fs::read_to_string(packaged.join("package.toml"))?;
    let kind = if meta_raw.contains("kind = \"skill\"") {
        "skill"
    } else if meta_raw.contains("kind = \"mcp_config\"") {
        "mcp"
    } else {
        bail!("packaged package.toml missing skill/mcp_config kind");
    };

    match kind {
        "skill" => {
            let skill_dir = if packaged.join("SKILL.md").is_file() {
                packaged.clone()
            } else {
                bail!("packaged skill missing SKILL.md");
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
        "mcp" => {
            let mcp = packaged.join("mcp.json");
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
                // Prefer extension id from extension.toml when present.
                let id = if packaged.join("extension.toml").is_file() {
                    load_extension_dir(&packaged)?.id.as_str().to_string()
                } else {
                    packaged
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("extension")
                        .split('-')
                        .next()
                        .unwrap_or("extension")
                        .to_string()
                };
                let dest = dest_dir.join(format!("{id}.json"));
                fs::copy(&mcp, &dest)?;
                println!("copied daemon MCP SoT -> {}", dest.display());
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
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

fn run_cargo(args: &[&str]) -> Result<()> {
    let status = Command::new("cargo").args(args).status()?;
    if !status.success() {
        bail!("cargo {:?} failed", args);
    }
    Ok(())
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
