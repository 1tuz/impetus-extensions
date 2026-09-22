//! Shared packaging, manifest validation, and digests for Impetus first-party extensions.
//!
//! Validates only documented fields of `impetus.extension.v1` and packaging metadata
//! that is *not* claimed to be part of the core install envelope.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Core install envelope schema id (Impetus `v0.1.2`).
pub const EXTENSION_SCHEMA_ID: &str = "impetus.extension.v1";
/// Core install envelope schema version.
pub const EXTENSION_SCHEMA_VERSION: u16 = 1;
/// MCP config schema id.
pub const MCP_SCHEMA_ID: &str = "impetus.mcp.v1";
/// MCP config schema version.
pub const MCP_SCHEMA_VERSION: u16 = 1;
/// Browser provider protocol version documented by Impetus.
pub const BROWSER_PROVIDER_PROTOCOL_VERSION: &str = "0.1";
/// Supported Impetus release tag for CI compatibility checks.
pub const SUPPORTED_IMPETUS_TAG: &str = "v0.1.2";

/// Ecosystem packaging metadata (NOT the core install envelope).
///
/// Lives in `package.toml` beside installable artifacts. Core `impetus extension install`
/// never reads this file; it exists for packaging, CI, and developer UX.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageMeta {
    pub id: String,
    pub version: String,
    pub kind: PackageKind,
    pub display_name: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub permission_rationale: Vec<PermissionRationale>,
    pub compatibility: Compatibility,
    #[serde(default)]
    pub entrypoints: Entrypoints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageKind {
    Skill,
    McpConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PermissionRationale {
    pub permission: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Compatibility {
    /// Documented stand-in until Impetus ships `extension_api_version`.
    pub extension_api_version: String,
    pub impetus_tag: String,
    pub extension_schema: String,
    pub mcp_schema: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_protocol: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entrypoints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_md: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,
}

/// Core `impetus.extension.v1` install envelope (documented fields only).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExtensionManifest {
    #[serde(default = "default_extension_schema_version")]
    pub schema_version: u16,
    pub id: String,
    pub kind: ExtensionManifestKind,
    pub version: String,
    pub digest: String,
    pub capabilities: Vec<String>,
}

fn default_extension_schema_version() -> u16 {
    EXTENSION_SCHEMA_VERSION
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
    #[error("unsupported schema_version {0} (expected {EXTENSION_SCHEMA_VERSION})")]
    UnsupportedSchemaVersion(u16),
    #[error("unknown critical field in envelope (undocumented fields are rejected)")]
    UndocumentedField,
}

#[derive(Debug, Error)]
pub enum SupportError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("package.toml parse error: {0}")]
    Toml(String),
    #[error("{0}")]
    Msg(String),
}

impl ExtensionManifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.schema_version != EXTENSION_SCHEMA_VERSION {
            return Err(ManifestError::UnsupportedSchemaVersion(self.schema_version));
        }
        if !is_valid_extension_id(&self.id) {
            return Err(ManifestError::InvalidId);
        }
        if self.version.trim().is_empty() {
            return Err(ManifestError::EmptyVersion);
        }
        validate_digest(&self.digest)?;
        validate_capabilities(&self.capabilities)?;
        Ok(())
    }

    /// Reject JSON objects that contain undocumented critical top-level keys.
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

pub fn validate_capabilities(capabilities: &[String]) -> Result<(), ManifestError> {
    if capabilities.is_empty() {
        return Err(ManifestError::InvalidCapabilities);
    }
    let mut seen = std::collections::BTreeSet::new();
    for token in capabilities {
        if !is_valid_capability_token(token) || !seen.insert(token.as_str()) {
            return Err(ManifestError::InvalidCapabilities);
        }
    }
    Ok(())
}

pub fn is_valid_capability_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    let Some((first, rest)) = bytes.split_first() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    rest.iter().all(|b| {
        b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_' || *b == b':' || *b == b'-'
    })
}

/// Content digest matching Impetus ownership style: `sha256:` + lowercase hex.
pub fn content_digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    format!("sha256:{}", hex::encode(hash))
}

pub fn file_digest(path: &Path) -> Result<String, SupportError> {
    let bytes = fs::read(path)?;
    Ok(content_digest(&bytes))
}

/// Digest of a skill directory = digest of canonical SKILL.md bytes.
pub fn skill_digest(skill_dir: &Path) -> Result<String, SupportError> {
    let skill_md = if skill_dir.is_file() {
        skill_dir.to_path_buf()
    } else {
        skill_dir.join("SKILL.md")
    };
    file_digest(&skill_md)
}

pub fn mcp_digest(mcp_json: &Path) -> Result<String, SupportError> {
    file_digest(mcp_json)
}

pub fn load_package_meta(path: &Path) -> Result<PackageMeta, SupportError> {
    let raw = fs::read_to_string(path)?;
    let meta: PackageMeta = toml::from_str(&raw).map_err(|e| SupportError::Toml(e.to_string()))?;
    if !is_valid_extension_id(&meta.id) {
        return Err(SupportError::Manifest(ManifestError::InvalidId));
    }
    if meta.compatibility.impetus_tag != SUPPORTED_IMPETUS_TAG {
        return Err(SupportError::Msg(format!(
            "package {} pins impetus_tag={}, expected {}",
            meta.id, meta.compatibility.impetus_tag, SUPPORTED_IMPETUS_TAG
        )));
    }
    if meta.compatibility.extension_schema
        != format!("{EXTENSION_SCHEMA_ID}@{EXTENSION_SCHEMA_VERSION}")
    {
        return Err(SupportError::Msg(format!(
            "package {} has unexpected extension_schema",
            meta.id
        )));
    }
    Ok(meta)
}

/// Build a core install envelope for a skill or MCP artifact.
pub fn build_core_manifest(
    id: &str,
    kind: ExtensionManifestKind,
    version: &str,
    digest: &str,
    capabilities: Vec<String>,
) -> Result<ExtensionManifest, ManifestError> {
    let manifest = ExtensionManifest {
        schema_version: EXTENSION_SCHEMA_VERSION,
        id: id.to_string(),
        kind,
        version: version.to_string(),
        digest: digest.to_string(),
        capabilities,
    };
    manifest.validate()?;
    Ok(manifest)
}

/// Canonical distributable layout written under `dist/{id}-{version}/`.
#[derive(Debug, Clone)]
pub struct PackageLayout {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub package_meta_path: PathBuf,
}

pub fn package_extension(
    extension_dir: &Path,
    out_root: &Path,
) -> Result<PackageLayout, SupportError> {
    let meta = load_package_meta(&extension_dir.join("package.toml"))?;
    let out = out_root.join(format!("{}-{}", meta.id, meta.version));
    if out.exists() {
        fs::remove_dir_all(&out)?;
    }
    fs::create_dir_all(&out)?;

    let (kind, digest, capabilities, artifact_name) = match meta.kind {
        PackageKind::Skill => {
            let skill_rel = meta
                .entrypoints
                .skill_md
                .clone()
                .unwrap_or_else(|| "SKILL.md".into());
            let src = extension_dir.join(&skill_rel);
            let dst = out.join("SKILL.md");
            fs::copy(&src, &dst)?;
            fs::copy(extension_dir.join("package.toml"), out.join("package.toml"))?;
            if extension_dir.join("README.md").exists() {
                fs::copy(extension_dir.join("README.md"), out.join("README.md"))?;
            }
            let digest = file_digest(&dst)?;
            (
                ExtensionManifestKind::Skill,
                digest,
                vec!["instructions".into(), "triggers".into()],
                "SKILL.md".to_string(),
            )
        }
        PackageKind::McpConfig => {
            let mcp_rel = meta
                .entrypoints
                .mcp_json
                .clone()
                .unwrap_or_else(|| "mcp.json".into());
            let src = extension_dir.join(&mcp_rel);
            let dst = out.join("mcp.json");
            fs::copy(&src, &dst)?;
            fs::copy(extension_dir.join("package.toml"), out.join("package.toml"))?;
            if extension_dir.join("README.md").exists() {
                fs::copy(extension_dir.join("README.md"), out.join("README.md"))?;
            }
            // Binary is referenced by mcp.json command; packaging copies release binary if present.
            if let Some(bin_rel) = &meta.entrypoints.binary {
                let bin_src = extension_dir.join(bin_rel);
                if bin_src.exists() {
                    let bin_name = bin_src
                        .file_name()
                        .ok_or_else(|| SupportError::Msg("binary path has no file name".into()))?;
                    fs::copy(&bin_src, out.join(bin_name))?;
                }
            }
            let digest = file_digest(&dst)?;
            (
                ExtensionManifestKind::McpConfig,
                digest,
                vec!["mcp".into(), "stdio".into(), "tools".into()],
                "mcp.json".to_string(),
            )
        }
    };

    let _ = artifact_name;
    let core = build_core_manifest(&meta.id, kind, &meta.version, &digest, capabilities)?;
    let manifest_path = out.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&core)? + "\n")?;

    Ok(PackageLayout {
        package_meta_path: out.join("package.toml"),
        root: out,
        manifest_path,
    })
}

pub fn discover_extensions(repo_root: &Path) -> Result<Vec<PathBuf>, SupportError> {
    let ext_root = repo_root.join("extensions");
    let mut out = Vec::new();
    for entry in fs::read_dir(ext_root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && entry.path().join("package.toml").exists() {
            out.push(entry.path());
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn digest_and_manifest_roundtrip() {
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
    fn rejects_undocumented_fields() {
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
    fn package_skill() {
        let dir = tempfile::tempdir().unwrap();
        let ext = dir.path().join("hello");
        fs::create_dir_all(&ext).unwrap();
        fs::write(
            ext.join("SKILL.md"),
            "---\nname: hello-extension\ndescription: demo\nversion: \"0.1.0\"\n---\n\nHi\n",
        )
        .unwrap();
        let mut pkg = fs::File::create(ext.join("package.toml")).unwrap();
        write!(
            pkg,
            r#"
id = "hello-extension"
version = "0.1.0"
kind = "skill"
display_name = "Hello"
description = "demo"
permissions = []
permission_rationale = []

[compatibility]
extension_api_version = "0.1.0-skill-mcp"
impetus_tag = "v0.1.2"
extension_schema = "impetus.extension.v1@1"
mcp_schema = "impetus.mcp.v1@1"

[entrypoints]
skill_md = "SKILL.md"
"#
        )
        .unwrap();
        let out = dir.path().join("dist");
        let layout = package_extension(&ext, &out).unwrap();
        assert!(layout.manifest_path.exists());
        let raw = fs::read_to_string(layout.manifest_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        ExtensionManifest::from_json_value(&v).unwrap();
    }

    /// Canonical tutorial under `extensions/hello-extension/` — CI without impetus CLI.
    #[test]
    fn package_hello_extension_from_repo() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ext = repo.join("extensions/hello-extension");
        assert!(
            ext.join("package.toml").is_file(),
            "expected {}",
            ext.join("package.toml").display()
        );
        let meta = load_package_meta(&ext.join("package.toml")).unwrap();
        assert_eq!(meta.id, "hello-extension");
        assert_eq!(meta.kind, PackageKind::Skill);
        assert!(meta.permissions.is_empty());
        assert_eq!(meta.compatibility.extension_api_version, "0.1.0-skill-mcp");

        let skill = fs::read_to_string(ext.join("SKILL.md")).unwrap();
        assert!(skill.starts_with("---\n"));
        assert!(skill.contains("name: hello-extension"));
        assert!(skill.contains("description:"));
        assert!(skill.contains("version:"));

        let out = tempfile::tempdir().unwrap();
        let layout = package_extension(&ext, out.path()).unwrap();
        let raw = fs::read_to_string(&layout.manifest_path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let m = ExtensionManifest::from_json_value(&v).unwrap();
        assert_eq!(m.id, "hello-extension");
        assert_eq!(m.kind, ExtensionManifestKind::Skill);
        assert_eq!(m.version, "0.1.0");
        assert!(m.capabilities.contains(&"instructions".into()));
        assert!(m.capabilities.contains(&"triggers".into()));
    }
}
