//! Canonical `extension.toml` validation, catalog consistency, and legacy packaging.
//!
//! Source of truth for packages is `extension.toml` (`impetus.extension_package.v1`)
//! via the public `impetus-extension-sdk`. Legacy `package.toml` / `manifest.json`
//! (`impetus.extension.v1`) are generated only into `dist/` for the old Skill/MCP
//! CLI install path.

pub use impetus_extension_sdk::{
    ExtensionEntrypoint, ExtensionPackageManifest, ExtensionPermission,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Legacy install envelope schema id (Impetus Skill/MCP CLI adapter).
pub const LEGACY_EXTENSION_SCHEMA_ID: &str = "impetus.extension.v1";
/// Legacy install envelope schema version.
pub const LEGACY_EXTENSION_SCHEMA_VERSION: u16 = 1;
/// MCP config schema id (legacy helper pin).
pub const MCP_SCHEMA_ID: &str = "impetus.mcp.v1";
/// MCP config schema version.
pub const MCP_SCHEMA_VERSION: u16 = 1;
/// Browser provider protocol version (implementation detail of browser pack).
pub const BROWSER_PROVIDER_PROTOCOL_VERSION: &str = "0.1";
/// Supported Impetus release tag for the legacy CLI install helper.
pub const SUPPORTED_IMPETUS_TAG: &str = "v0.1.2";
/// Immutable Impetus git rev that provides `impetus-extension-sdk`.
pub const SDK_GIT_REV: &str = "7dc456a0062fd61c9d47d1abe2fed76fec56a12c";
/// Canonical package schema id (matches SDK).
pub const PACKAGE_SCHEMA_ID: &str = "impetus.extension_package.v1";

/// Allowed entrypoint kinds (closed set for v1).
pub const ALLOWED_ENTRYPOINT_KINDS: &[&str] = &["instruction_pack", "mcp_bridge", "host_process"];

/// Closed permission vocabulary (matches SDK `ExtensionPermission`).
pub const ALLOWED_PERMISSIONS: &[&str] = &[
    "filesystem_read",
    "filesystem_write",
    "network",
    "process_spawn",
    "pty",
    "git",
    "mcp",
    "browser",
    "lsp",
    "memory",
    "secrets_provider",
];

#[derive(Debug, Error)]
pub enum SupportError {
    #[error("{0}")]
    Msg(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("extension.toml: {0}")]
    Manifest(String),
    #[error("catalog: {0}")]
    Catalog(String),
    #[error("legacy envelope: {0}")]
    Legacy(String),
}

/// Content digest matching Impetus ownership style: `sha256:` + lowercase hex.
pub fn content_digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    format!("sha256:{}", hex::encode(hash))
}

pub fn file_digest(path: &Path) -> Result<String, SupportError> {
    Ok(content_digest(&fs::read(path)?))
}

/// Load and validate canonical `extension.toml` via the public SDK.
pub fn load_extension_manifest(path: &Path) -> Result<ExtensionPackageManifest, SupportError> {
    let raw = fs::read_to_string(path)
        .map_err(|e| SupportError::Msg(format!("read {}: {e}", path.display())))?;
    ExtensionPackageManifest::from_toml_str(&raw)
        .map_err(|e| SupportError::Manifest(format!("{}: {e}", path.display())))
}

pub fn load_extension_dir(dir: &Path) -> Result<ExtensionPackageManifest, SupportError> {
    let path = dir.join("extension.toml");
    if !path.is_file() {
        return Err(SupportError::Msg(format!(
            "missing canonical manifest {}",
            path.display()
        )));
    }
    // Dual source of truth is forbidden in package sources.
    let legacy = dir.join("package.toml");
    if legacy.is_file() {
        return Err(SupportError::Msg(format!(
            "{}: legacy package.toml must not exist beside extension.toml (delete it; legacy envelopes are generated only under dist/)",
            dir.display()
        )));
    }
    load_extension_manifest(&path)
}

/// Discover package directories under `extensions/` that contain `extension.toml`.
pub fn discover_extensions(repo_root: &Path) -> Result<Vec<PathBuf>, SupportError> {
    let ext_root = repo_root.join("extensions");
    if !ext_root.is_dir() {
        return Err(SupportError::Msg(format!(
            "missing extensions directory {}",
            ext_root.display()
        )));
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&ext_root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && entry.path().join("extension.toml").is_file() {
            out.push(entry.path());
        }
    }
    out.sort();
    Ok(out)
}

fn entrypoint_kind(ep: &ExtensionEntrypoint) -> &'static str {
    match ep {
        ExtensionEntrypoint::InstructionPack { .. } => "instruction_pack",
        ExtensionEntrypoint::McpBridge { .. } => "mcp_bridge",
        ExtensionEntrypoint::HostProcess { .. } => "host_process",
    }
}

fn portable_surface(ep: &ExtensionEntrypoint) -> &'static str {
    match ep {
        ExtensionEntrypoint::InstructionPack { .. } => "skill",
        ExtensionEntrypoint::McpBridge { .. } => "mcp",
        ExtensionEntrypoint::HostProcess { .. } => "host_process",
    }
}

/// Validate entrypoint artifacts exist on disk relative to the package directory.
pub fn validate_entrypoint_artifacts(
    dir: &Path,
    manifest: &ExtensionPackageManifest,
) -> Result<(), SupportError> {
    match &manifest.entrypoint {
        ExtensionEntrypoint::InstructionPack { root } => {
            let root_path = dir.join(root);
            if !root_path.is_dir() {
                return Err(SupportError::Msg(format!(
                    "{}: instruction_pack root `{}` is not a directory",
                    manifest.id.as_str(),
                    root
                )));
            }
            let skill = root_path.join("SKILL.md");
            if !skill.is_file() {
                return Err(SupportError::Msg(format!(
                    "{}: missing {}/SKILL.md",
                    manifest.id.as_str(),
                    root
                )));
            }
        }
        ExtensionEntrypoint::McpBridge { module_id } => {
            let mcp = dir.join("mcp.json");
            if !mcp.is_file() {
                return Err(SupportError::Msg(format!(
                    "{}: mcp_bridge requires mcp.json beside extension.toml",
                    manifest.id.as_str()
                )));
            }
            let raw = fs::read_to_string(&mcp)?;
            let value: serde_json::Value = serde_json::from_str(&raw)?;
            let mcp_id = value
                .get("id")
                .or_else(|| value.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if mcp_id.is_empty() {
                return Err(SupportError::Msg(format!(
                    "{}: mcp.json must set id (or name) equal to entrypoint.module_id `{module_id}`",
                    manifest.id.as_str()
                )));
            }
            if mcp_id != module_id {
                return Err(SupportError::Msg(format!(
                    "{}: mcp.json id/name `{mcp_id}` != entrypoint.module_id `{module_id}`",
                    manifest.id.as_str()
                )));
            }
            let command = value.get("command").and_then(|v| v.as_str()).unwrap_or("");
            if command.is_empty() {
                return Err(SupportError::Msg(format!(
                    "{}: mcp.json missing command",
                    manifest.id.as_str()
                )));
            }
        }
        ExtensionEntrypoint::HostProcess { command, .. } => {
            if command.trim().is_empty() {
                return Err(SupportError::Msg(format!(
                    "{}: host_process command must be non-empty",
                    manifest.id.as_str()
                )));
            }
            // Relative commands must resolve under the package directory.
            if !Path::new(command).is_absolute() {
                let candidate = dir.join(command);
                if !candidate.is_file() {
                    return Err(SupportError::Msg(format!(
                        "{}: host_process command `{}` not found under package",
                        manifest.id.as_str(),
                        command
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Validate all packages under `extensions/` (manifest + artifacts + no legacy SoT).
pub fn validate_all_extensions(
    repo_root: &Path,
) -> Result<Vec<ExtensionPackageManifest>, SupportError> {
    let mut manifests = Vec::new();
    let mut ids = BTreeSet::new();
    for dir in discover_extensions(repo_root)? {
        let manifest = load_extension_dir(&dir)?;
        validate_entrypoint_artifacts(&dir, &manifest)?;
        let id = manifest.id.as_str().to_string();
        if !ids.insert(id.clone()) {
            return Err(SupportError::Msg(format!(
                "duplicate extension id `{id}` under extensions/"
            )));
        }
        // Permissions must stay inside the closed SDK vocabulary (serde already
        // rejects unknowns; re-assert for clearer CI messages).
        for perm in &manifest.permissions {
            let token = perm.as_str();
            if !ALLOWED_PERMISSIONS.contains(&token) {
                return Err(SupportError::Msg(format!(
                    "{id}: unknown permission `{token}`"
                )));
            }
        }
        if !ALLOWED_ENTRYPOINT_KINDS.contains(&entrypoint_kind(&manifest.entrypoint)) {
            return Err(SupportError::Msg(format!("{id}: unknown entrypoint kind")));
        }
        manifests.push(manifest);
    }
    Ok(manifests)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CatalogFile {
    pub schema_version: u32,
    pub repository: String,
    pub extensions: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub summary: String,
    pub source_path: String,
    pub entrypoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portable_surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    #[serde(default)]
    pub distribution: serde_json::Value,
}

/// Validate `catalog.json` against canonical manifests (no second SoT drift).
pub fn validate_catalog(repo_root: &Path) -> Result<CatalogFile, SupportError> {
    let path = repo_root.join("catalog.json");
    let raw = fs::read_to_string(&path)
        .map_err(|e| SupportError::Catalog(format!("read {}: {e}", path.display())))?;
    let catalog: CatalogFile =
        serde_json::from_str(&raw).map_err(|e| SupportError::Catalog(format!("parse: {e}")))?;
    if catalog.schema_version != 1 {
        return Err(SupportError::Catalog(format!(
            "unsupported schema_version {}",
            catalog.schema_version
        )));
    }

    let manifests = validate_all_extensions(repo_root)?;
    let by_id: BTreeMap<String, ExtensionPackageManifest> = manifests
        .into_iter()
        .map(|m| (m.id.as_str().to_string(), m))
        .collect();

    let mut seen = BTreeSet::new();
    for entry in &catalog.extensions {
        if !seen.insert(entry.id.clone()) {
            return Err(SupportError::Catalog(format!(
                "duplicate catalog id `{}`",
                entry.id
            )));
        }
        if entry.version.trim().is_empty() {
            return Err(SupportError::Catalog(format!(
                "{}: empty version",
                entry.id
            )));
        }
        if semver::Version::parse(entry.version.trim()).is_err() {
            // soft: SDK uses semver; catalog must match
            return Err(SupportError::Catalog(format!(
                "{}: invalid semver `{}`",
                entry.id, entry.version
            )));
        }
        if !ALLOWED_ENTRYPOINT_KINDS.contains(&entry.entrypoint.as_str()) {
            return Err(SupportError::Catalog(format!(
                "{}: unknown entrypoint `{}`",
                entry.id, entry.entrypoint
            )));
        }

        let dir = repo_root.join(&entry.source_path);
        let manifest = load_extension_dir(&dir)
            .map_err(|e| SupportError::Catalog(format!("{}: {}", entry.id, e)))?;
        if manifest.id.as_str() != entry.id {
            return Err(SupportError::Catalog(format!(
                "{}: catalog id != extension.toml id `{}`",
                entry.id,
                manifest.id.as_str()
            )));
        }
        if manifest.name != entry.name {
            return Err(SupportError::Catalog(format!(
                "{}: catalog name `{}` != extension.toml name `{}`",
                entry.id, entry.name, manifest.name
            )));
        }
        if manifest.version != entry.version {
            return Err(SupportError::Catalog(format!(
                "{}: catalog version `{}` != extension.toml version `{}`",
                entry.id, entry.version, manifest.version
            )));
        }
        let kind = entrypoint_kind(&manifest.entrypoint);
        if entry.entrypoint != kind {
            return Err(SupportError::Catalog(format!(
                "{}: catalog entrypoint `{}` != extension.toml `{kind}`",
                entry.id, entry.entrypoint
            )));
        }
        if let Some(surface) = &entry.portable_surface {
            let expected = portable_surface(&manifest.entrypoint);
            if surface != expected {
                return Err(SupportError::Catalog(format!(
                    "{}: portable_surface `{surface}` != derived `{expected}`",
                    entry.id
                )));
            }
        }
        by_id.get(&entry.id).ok_or_else(|| {
            SupportError::Catalog(format!(
                "{}: listed in catalog but not discovered under extensions/",
                entry.id
            ))
        })?;
    }

    for id in by_id.keys() {
        if !seen.contains(id) {
            return Err(SupportError::Catalog(format!(
                "extension `{id}` has extension.toml but is missing from catalog.json"
            )));
        }
    }

    Ok(catalog)
}

/// Rebuild catalog discovery fields from canonical manifests (keeps summary /
/// implementation / distribution when present).
pub fn sync_catalog_from_manifests(repo_root: &Path) -> Result<CatalogFile, SupportError> {
    let path = repo_root.join("catalog.json");
    let existing = if path.is_file() {
        let raw = fs::read_to_string(&path)?;
        serde_json::from_str::<CatalogFile>(&raw).ok()
    } else {
        None
    };
    let old_by_id: BTreeMap<String, CatalogEntry> = existing
        .as_ref()
        .map(|c| {
            c.extensions
                .iter()
                .cloned()
                .map(|e| (e.id.clone(), e))
                .collect()
        })
        .unwrap_or_default();

    let mut extensions = Vec::new();
    for dir in discover_extensions(repo_root)? {
        let manifest = load_extension_dir(&dir)?;
        let id = manifest.id.as_str().to_string();
        let rel = dir
            .strip_prefix(repo_root)
            .unwrap_or(&dir)
            .to_string_lossy()
            .replace('\\', "/");
        let prev = old_by_id.get(&id);
        extensions.push(CatalogEntry {
            id: id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            summary: prev
                .map(|p| p.summary.clone())
                .unwrap_or_else(|| manifest.description.clone()),
            source_path: rel,
            entrypoint: entrypoint_kind(&manifest.entrypoint).to_string(),
            portable_surface: Some(portable_surface(&manifest.entrypoint).to_string()),
            implementation: prev
                .and_then(|p| p.implementation.clone())
                .or_else(|| match &manifest.entrypoint {
                    ExtensionEntrypoint::InstructionPack { .. } => Some("declarative".into()),
                    _ => Some("rust".into()),
                }),
            distribution: prev
                .map(|p| p.distribution.clone())
                .unwrap_or_else(|| serde_json::json!({ "kind": "repository_source" })),
        });
    }
    extensions.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(CatalogFile {
        schema_version: 1,
        repository: existing
            .map(|c| c.repository)
            .unwrap_or_else(|| "https://github.com/1tuz/impetus-extensions".into()),
        extensions,
    })
}

// --- Legacy Skill/MCP install envelope (generated under dist/ only) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageKind {
    Skill,
    McpConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExtensionManifest {
    #[serde(default = "default_legacy_schema_version")]
    pub schema_version: u16,
    pub id: String,
    pub kind: ExtensionManifestKind,
    pub version: String,
    pub digest: String,
    pub capabilities: Vec<String>,
}

fn default_legacy_schema_version() -> u16 {
    LEGACY_EXTENSION_SCHEMA_VERSION
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionManifestKind {
    Skill,
    McpConfig,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("extension id must be non-empty and match ^[a-z0-9][a-z0-9_-]{{0,63}}$")]
    InvalidId,
    #[error("extension version must be non-empty")]
    EmptyVersion,
    #[error("invalid digest `{0}`: expected sha256: + 64 lowercase hex")]
    InvalidDigest(String),
    #[error("capabilities must contain at least one unique valid token")]
    InvalidCapabilities,
    #[error("unsupported schema_version {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("unknown critical field in envelope (undocumented fields are rejected)")]
    UndocumentedField,
}

impl ExtensionManifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != LEGACY_EXTENSION_SCHEMA_VERSION {
            return Err(ManifestError::UnsupportedSchemaVersion(self.schema_version));
        }
        if !is_valid_extension_id(&self.id) {
            return Err(ManifestError::InvalidId);
        }
        if self.version.trim().is_empty() {
            return Err(ManifestError::EmptyVersion);
        }
        validate_digest(&self.digest)?;
        validate_legacy_capabilities(&self.capabilities)?;
        Ok(())
    }

    pub fn from_json_value(value: &serde_json::Value) -> Result<Self, ManifestError> {
        let obj = value.as_object().ok_or(ManifestError::UndocumentedField)?;
        const ALLOWED: &[&str] = &[
            "schema_version",
            "id",
            "kind",
            "version",
            "digest",
            "capabilities",
        ];
        for key in obj.keys() {
            if !ALLOWED.contains(&key.as_str()) {
                return Err(ManifestError::UndocumentedField);
            }
        }
        let manifest: ExtensionManifest =
            serde_json::from_value(value.clone()).map_err(|_| ManifestError::UndocumentedField)?;
        manifest.validate()?;
        Ok(manifest)
    }
}

pub fn is_valid_extension_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    let Some((first, rest)) = bytes.split_first() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    rest.iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-')
}

pub fn validate_digest(digest: &str) -> Result<(), ManifestError> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(ManifestError::InvalidDigest(digest.to_string()));
    };
    if hex.len() != 64 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(ManifestError::InvalidDigest(digest.to_string()));
    }
    Ok(())
}

fn validate_legacy_capabilities(capabilities: &[String]) -> Result<(), ManifestError> {
    if capabilities.is_empty() {
        return Err(ManifestError::InvalidCapabilities);
    }
    let mut seen = BTreeSet::new();
    for token in capabilities {
        if token.is_empty() || !seen.insert(token.as_str()) {
            return Err(ManifestError::InvalidCapabilities);
        }
    }
    Ok(())
}

pub fn build_core_manifest(
    id: &str,
    kind: ExtensionManifestKind,
    version: &str,
    digest: &str,
    capabilities: Vec<String>,
) -> Result<ExtensionManifest, ManifestError> {
    let manifest = ExtensionManifest {
        schema_version: LEGACY_EXTENSION_SCHEMA_VERSION,
        id: id.to_string(),
        kind,
        version: version.to_string(),
        digest: digest.to_string(),
        capabilities,
    };
    manifest.validate()?;
    Ok(manifest)
}

fn permission_tokens(manifest: &ExtensionPackageManifest) -> Vec<String> {
    manifest
        .permissions
        .iter()
        .map(|p| p.as_str().to_string())
        .collect()
}

fn write_legacy_package_toml(
    out: &Path,
    manifest: &ExtensionPackageManifest,
    kind: PackageKind,
    skill_md: Option<&str>,
    mcp_json: Option<&str>,
    binary: Option<&str>,
) -> Result<(), SupportError> {
    let mut doc = String::new();
    doc.push_str("# GENERATED from extension.toml — legacy Skill/MCP CLI adapter only.\n");
    doc.push_str(&format!("id = {:?}\n", manifest.id.as_str()));
    doc.push_str(&format!("version = {:?}\n", manifest.version));
    doc.push_str(&format!(
        "kind = {:?}\n",
        match kind {
            PackageKind::Skill => "skill",
            PackageKind::McpConfig => "mcp_config",
        }
    ));
    doc.push_str(&format!("display_name = {:?}\n", manifest.name));
    doc.push_str(&format!("description = {:?}\n", manifest.description));
    let perms = permission_tokens(manifest);
    doc.push_str("permissions = [");
    for (i, p) in perms.iter().enumerate() {
        if i > 0 {
            doc.push_str(", ");
        }
        doc.push_str(&format!("{p:?}"));
    }
    doc.push_str("]\n");
    doc.push_str("permission_rationale = []\n\n");
    doc.push_str("[compatibility]\n");
    doc.push_str("extension_api_version = \"0.1.0-skill-mcp\"\n");
    doc.push_str(&format!("impetus_tag = {SUPPORTED_IMPETUS_TAG:?}\n"));
    doc.push_str(&format!(
        "extension_schema = \"{LEGACY_EXTENSION_SCHEMA_ID}@{LEGACY_EXTENSION_SCHEMA_VERSION}\"\n"
    ));
    doc.push_str(&format!(
        "mcp_schema = \"{MCP_SCHEMA_ID}@{MCP_SCHEMA_VERSION}\"\n"
    ));
    if manifest.id.as_str() == "browser" {
        doc.push_str(&format!(
            "browser_protocol = {BROWSER_PROVIDER_PROTOCOL_VERSION:?}\n"
        ));
    }
    doc.push('\n');
    doc.push_str("[entrypoints]\n");
    if let Some(s) = skill_md {
        doc.push_str(&format!("skill_md = {s:?}\n"));
    }
    if let Some(m) = mcp_json {
        doc.push_str(&format!("mcp_json = {m:?}\n"));
    }
    if let Some(b) = binary {
        doc.push_str(&format!("binary = {b:?}\n"));
    }
    fs::write(out.join("package.toml"), doc)?;
    Ok(())
}

/// Canonical distributable layout written under `dist/{id}-{version}/`.
#[derive(Debug, Clone)]
pub struct PackageLayout {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub package_meta_path: PathBuf,
}

/// Package from canonical `extension.toml` into a legacy Skill/MCP install layout.
///
/// `host_process` packages are not installable via the legacy CLI — this returns an error.
pub fn package_extension(
    extension_dir: &Path,
    out_root: &Path,
) -> Result<PackageLayout, SupportError> {
    let manifest = load_extension_dir(extension_dir)?;
    validate_entrypoint_artifacts(extension_dir, &manifest)?;

    let out = out_root.join(format!("{}-{}", manifest.id.as_str(), manifest.version));
    if out.exists() {
        fs::remove_dir_all(&out)?;
    }
    fs::create_dir_all(&out)?;
    fs::copy(
        extension_dir.join("extension.toml"),
        out.join("extension.toml"),
    )?;
    if extension_dir.join("README.md").exists() {
        fs::copy(extension_dir.join("README.md"), out.join("README.md"))?;
    }

    let (kind, digest, capabilities) = match &manifest.entrypoint {
        ExtensionEntrypoint::InstructionPack { root } => {
            let src = extension_dir.join(root).join("SKILL.md");
            let dst = out.join("SKILL.md");
            fs::copy(&src, &dst)?;
            // Also copy skills tree for package-host consumers.
            let skills_out = out.join(root);
            fs::create_dir_all(&skills_out)?;
            fs::copy(&src, skills_out.join("SKILL.md"))?;
            write_legacy_package_toml(
                &out,
                &manifest,
                PackageKind::Skill,
                Some("SKILL.md"),
                None,
                None,
            )?;
            let digest = file_digest(&dst)?;
            (
                ExtensionManifestKind::Skill,
                digest,
                vec!["instructions".into(), "triggers".into()],
            )
        }
        ExtensionEntrypoint::McpBridge { .. } => {
            let src = extension_dir.join("mcp.json");
            let dst = out.join("mcp.json");
            fs::copy(&src, &dst)?;
            let raw: serde_json::Value = serde_json::from_str(&fs::read_to_string(&dst)?)?;
            let binary_name = raw
                .get("command")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            if let Some(ref name) = binary_name {
                let bin_src = extension_dir
                    .join("target/release")
                    .join(name)
                    .exists()
                    .then(|| extension_dir.join("target/release").join(name));
                // Prefer workspace target if present (dev packaging).
                let candidates = [
                    extension_dir.join(name),
                    PathBuf::from("target/release").join(name),
                    PathBuf::from("target/debug").join(name),
                ];
                for c in candidates.into_iter().chain(bin_src) {
                    if c.is_file() {
                        fs::copy(&c, out.join(name))?;
                        break;
                    }
                }
            }
            write_legacy_package_toml(
                &out,
                &manifest,
                PackageKind::McpConfig,
                None,
                Some("mcp.json"),
                binary_name.as_deref(),
            )?;
            let digest = file_digest(&dst)?;
            (
                ExtensionManifestKind::McpConfig,
                digest,
                vec!["mcp".into(), "stdio".into(), "tools".into()],
            )
        }
        ExtensionEntrypoint::HostProcess { .. } => {
            return Err(SupportError::Msg(format!(
                "{}: host_process is not packaged via the legacy Skill/MCP CLI; install under the Extension Host package root",
                manifest.id.as_str()
            )));
        }
    };

    let core = build_core_manifest(
        manifest.id.as_str(),
        kind,
        &manifest.version,
        &digest,
        capabilities,
    )
    .map_err(|e| SupportError::Legacy(e.to_string()))?;
    let manifest_path = out.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&core)? + "\n")?;

    Ok(PackageLayout {
        package_meta_path: out.join("package.toml"),
        root: out,
        manifest_path,
    })
}

/// Check `compatibility.json` pins for SDK + legacy helper.
pub fn check_compatibility_file(repo_root: &Path) -> Result<(), SupportError> {
    let path = repo_root.join("compatibility.json");
    let raw = fs::read_to_string(&path)
        .map_err(|e| SupportError::Msg(format!("missing {}: {e}", path.display())))?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;

    let tag = value
        .pointer("/legacy_helper/impetus_tag")
        .or_else(|| value.get("impetus_tag"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if tag != SUPPORTED_IMPETUS_TAG {
        return Err(SupportError::Msg(format!(
            "compatibility.json legacy impetus_tag={tag}, expected {SUPPORTED_IMPETUS_TAG}"
        )));
    }

    let rev = value
        .pointer("/sdk/rev")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if rev != SDK_GIT_REV {
        return Err(SupportError::Msg(format!(
            "compatibility.json sdk.rev={rev}, expected {SDK_GIT_REV}"
        )));
    }

    let schema = value
        .pointer("/canonical_package_contract/schema")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if schema != PACKAGE_SCHEMA_ID {
        return Err(SupportError::Msg(format!(
            "compatibility.json canonical schema={schema}, expected {PACKAGE_SCHEMA_ID}"
        )));
    }

    let entrypoints = value
        .pointer("/canonical_package_contract/entrypoints")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let kinds: BTreeSet<String> = entrypoints
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    for expected in ALLOWED_ENTRYPOINT_KINDS {
        if !kinds.contains(*expected) {
            return Err(SupportError::Msg(format!(
                "compatibility.json missing entrypoint `{expected}`"
            )));
        }
    }
    if kinds.contains("native_module") {
        return Err(SupportError::Msg(
            "compatibility.json must not list native_module as a public entrypoint".into(),
        ));
    }

    // Ensure no package declares secrets_provider casually without documenting.
    for dir in discover_extensions(repo_root)? {
        let m = load_extension_dir(&dir)?;
        if m.permissions
            .contains(&ExtensionPermission::SecretsProvider)
        {
            return Err(SupportError::Msg(format!(
                "{}: secrets_provider requires explicit SECURITY.md review — refuse casual grants",
                m.id.as_str()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_and_legacy_manifest_roundtrip() {
        let digest = content_digest(b"hello");
        assert!(digest.starts_with("sha256:"));
        let m = build_core_manifest(
            "hello-extension",
            ExtensionManifestKind::Skill,
            "0.1.0",
            &digest,
            vec!["instructions".into()],
        )
        .unwrap();
        m.validate().unwrap();
        let v = serde_json::to_value(&m).unwrap();
        ExtensionManifest::from_json_value(&v).unwrap();
    }

    #[test]
    fn rejects_undocumented_legacy_fields() {
        let v = serde_json::json!({
            "schema_version": 1,
            "id": "x",
            "kind": "skill",
            "version": "0.1.0",
            "digest": content_digest(b"a"),
            "capabilities": ["instructions"],
            "extra": true
        });
        assert!(matches!(
            ExtensionManifest::from_json_value(&v),
            Err(ManifestError::UndocumentedField)
        ));
    }

    #[test]
    fn package_hello_extension_from_repo() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ext = repo.join("extensions/hello-extension");
        assert!(
            ext.join("extension.toml").is_file(),
            "expected {}",
            ext.join("extension.toml").display()
        );
        assert!(
            !ext.join("package.toml").exists(),
            "legacy package.toml must be removed from sources"
        );

        let manifest = load_extension_dir(&ext).unwrap();
        assert_eq!(manifest.id.as_str(), "hello-extension");
        assert!(matches!(
            manifest.entrypoint,
            ExtensionEntrypoint::InstructionPack { .. }
        ));
        assert!(manifest.permissions.is_empty());

        let out = tempfile::tempdir().unwrap();
        let layout = package_extension(&ext, out.path()).unwrap();
        let raw = fs::read_to_string(&layout.manifest_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let m = ExtensionManifest::from_json_value(&v).unwrap();
        assert_eq!(m.id, "hello-extension");
        assert_eq!(m.kind, ExtensionManifestKind::Skill);
        assert_eq!(m.version, "0.1.0");
        assert!(layout.root.join("extension.toml").is_file());
        assert!(layout.package_meta_path.is_file());
    }

    #[test]
    fn catalog_matches_extensions() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        validate_catalog(&repo).expect("catalog must match extension.toml");
    }

    #[test]
    fn sdk_constants_align() {
        assert_eq!(PACKAGE_SCHEMA_ID, "impetus.extension_package.v1");
        assert!(!ALLOWED_ENTRYPOINT_KINDS.contains(&"native_module"));
    }
}
