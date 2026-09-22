//! Browser Provider Protocol `0.1` wire shapes (serde), matching documented semantics.
//!
//! Source of truth for field meaning: Impetus `docs/reference/browser-provider-protocol.md`.
//! This crate deliberately does not import `impetus-core`.

use serde::{Deserialize, Serialize};

/// Semantic protocol version aligned with the audited reference (`0.1`).
pub const BROWSER_PROVIDER_PROTOCOL_VERSION: &str = "0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserCapability {
    Navigate,
    Snapshot,
    RenderedReadableText,
    RenderedLinks,
    Screenshot,
    Click,
    Type,
    Wait,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserProviderDescriptor {
    pub provider_id: String,
    pub provider_version: Option<String>,
    pub protocol_version: String,
    pub browser_families: Vec<String>,
    pub capabilities: Vec<BrowserCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum BrowserServiceStatus {
    Unavailable {
        reason: String,
    },
    Degraded {
        reason: String,
    },
    Misconfigured {
        reason: String,
    },
    Available {
        provider_id: String,
        capabilities: Vec<BrowserCapability>,
    },
}

fn default_protocol_version() -> String {
    BROWSER_PROVIDER_PROTOCOL_VERSION.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserNegotiateRequest {
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    #[serde(default)]
    pub required_capabilities: Vec<BrowserCapability>,
    #[serde(default)]
    pub optional_capabilities: Vec<BrowserCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserNegotiateResult {
    pub protocol_version: String,
    pub compatible: bool,
    pub provider: BrowserProviderDescriptor,
    pub granted_capabilities: Vec<BrowserCapability>,
    pub missing_required: Vec<BrowserCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSessionEnsureRequest {
    pub client_session_id: String,
    #[serde(default)]
    pub browser_preference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSessionHandle {
    pub session_id: String,
    pub browser_family: Option<String>,
    pub default_page_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserNavigateRequest {
    pub session_id: String,
    pub url: String,
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub new_page: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserNavigateResult {
    pub page_id: String,
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserPageState {
    pub session_id: String,
    pub page_id: String,
    pub url: String,
    pub title: Option<String>,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserFetchRenderedRequest {
    pub session_id: String,
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserFetchRenderedResult {
    pub page_id: String,
    pub url: String,
    pub title: Option<String>,
    pub content_type: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserActionRequest {
    pub session_id: String,
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserActionResult {
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserCancelRequest {
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Major-component protocol compatibility (`0.1` ≈ `0.1.x`).
pub fn protocol_compatible(requested: &str, offered: &str) -> bool {
    major_component(requested) == major_component(offered)
}

fn major_component(version: &str) -> &str {
    version.split('.').next().unwrap_or(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_tag_shape() {
        let s = BrowserServiceStatus::Available {
            provider_id: "mock".into(),
            capabilities: vec![BrowserCapability::Navigate],
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["status"], "available");
        assert_eq!(v["provider_id"], "mock");
    }

    #[test]
    fn protocol_major_match() {
        assert!(protocol_compatible("0.1", "0.1.2"));
        assert!(!protocol_compatible("1.0", "0.1"));
    }
}
