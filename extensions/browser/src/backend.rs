//! Browser backends: mock (default) and optional CDP scaffold.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::protocol::{
    BROWSER_PROVIDER_PROTOCOL_VERSION, BrowserActionRequest, BrowserActionResult,
    BrowserCapability, BrowserFetchRenderedRequest, BrowserFetchRenderedResult,
    BrowserNavigateRequest, BrowserNavigateResult, BrowserNegotiateRequest, BrowserNegotiateResult,
    BrowserPageState, BrowserProviderDescriptor, BrowserServiceStatus, BrowserSessionEnsureRequest,
    BrowserSessionHandle, protocol_compatible,
};

pub type SharedBackend = Arc<Backend>;

/// Runtime backend selection.
pub enum Backend {
    Mock(MockBackend),
    Cdp(CdpBackend),
}

impl Backend {
    pub fn mock() -> SharedBackend {
        Arc::new(Self::Mock(MockBackend::new()))
    }

    pub fn cdp_from_env() -> SharedBackend {
        Arc::new(Self::Cdp(CdpBackend::from_env()))
    }

    pub fn descriptor(&self) -> BrowserProviderDescriptor {
        match self {
            Self::Mock(b) => b.descriptor(),
            Self::Cdp(b) => b.descriptor(),
        }
    }

    pub async fn status(&self) -> BrowserServiceStatus {
        match self {
            Self::Mock(b) => b.status().await,
            Self::Cdp(b) => b.status().await,
        }
    }

    pub async fn negotiate(
        &self,
        request: BrowserNegotiateRequest,
    ) -> Result<BrowserNegotiateResult, BackendError> {
        match self {
            Self::Mock(b) => b.negotiate(request).await,
            Self::Cdp(b) => b.negotiate(request).await,
        }
    }

    pub async fn ensure_session(
        &self,
        request: BrowserSessionEnsureRequest,
    ) -> Result<BrowserSessionHandle, BackendError> {
        match self {
            Self::Mock(b) => b.ensure_session(request).await,
            Self::Cdp(b) => b.ensure_session(request).await,
        }
    }

    pub async fn close_session(&self, session_id: &str) -> Result<(), BackendError> {
        match self {
            Self::Mock(b) => b.close_session(session_id).await,
            Self::Cdp(b) => b.close_session(session_id).await,
        }
    }

    pub async fn navigate(
        &self,
        request: BrowserNavigateRequest,
    ) -> Result<BrowserNavigateResult, BackendError> {
        match self {
            Self::Mock(b) => b.navigate(request).await,
            Self::Cdp(b) => b.navigate(request).await,
        }
    }

    pub async fn page_state(
        &self,
        session_id: &str,
        page_id: Option<&str>,
    ) -> Result<BrowserPageState, BackendError> {
        match self {
            Self::Mock(b) => b.page_state(session_id, page_id).await,
            Self::Cdp(b) => b.page_state(session_id, page_id).await,
        }
    }

    pub async fn fetch_rendered(
        &self,
        request: BrowserFetchRenderedRequest,
    ) -> Result<BrowserFetchRenderedResult, BackendError> {
        match self {
            Self::Mock(b) => b.fetch_rendered(request).await,
            Self::Cdp(b) => b.fetch_rendered(request).await,
        }
    }

    pub async fn click(
        &self,
        request: BrowserActionRequest,
    ) -> Result<BrowserActionResult, BackendError> {
        match self {
            Self::Mock(b) => b.click(request).await,
            Self::Cdp(b) => b.click(request).await,
        }
    }

    pub async fn type_text(
        &self,
        request: BrowserActionRequest,
    ) -> Result<BrowserActionResult, BackendError> {
        match self {
            Self::Mock(b) => b.type_text(request).await,
            Self::Cdp(b) => b.type_text(request).await,
        }
    }

    pub async fn wait(
        &self,
        request: BrowserActionRequest,
    ) -> Result<BrowserActionResult, BackendError> {
        match self {
            Self::Mock(b) => b.wait(request).await,
            Self::Cdp(b) => b.wait(request).await,
        }
    }

    pub async fn cancel(
        &self,
        session_id: Option<&str>,
    ) -> Result<BrowserActionResult, BackendError> {
        match self {
            Self::Mock(b) => b.cancel(session_id).await,
            Self::Cdp(b) => b.cancel(session_id).await,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("{0}")]
    Msg(String),
}

impl BackendError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Msg(s.into())
    }
}

struct SessionState {
    cancel: CancellationToken,
    page_counter: u64,
    /// page_id → (url, title)
    pages: HashMap<String, (String, Option<String>)>,
    default_page_id: String,
}

/// In-memory CI backend. No browser binary. Always works with `--mock`.
pub struct MockBackend {
    sessions: Mutex<HashMap<String, SessionState>>,
    root_cancel: CancellationToken,
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            root_cancel: CancellationToken::new(),
        }
    }

    pub fn descriptor(&self) -> BrowserProviderDescriptor {
        Self::descriptor_inner()
    }

    fn descriptor_inner() -> BrowserProviderDescriptor {
        BrowserProviderDescriptor {
            provider_id: "mock".into(),
            provider_version: Some(env!("CARGO_PKG_VERSION").into()),
            protocol_version: BROWSER_PROVIDER_PROTOCOL_VERSION.into(),
            browser_families: vec!["mock".into()],
            capabilities: vec![
                BrowserCapability::Navigate,
                BrowserCapability::RenderedReadableText,
                BrowserCapability::RenderedLinks,
                BrowserCapability::Click,
                BrowserCapability::Type,
                BrowserCapability::Wait,
            ],
        }
    }

    pub async fn status(&self) -> BrowserServiceStatus {
        if self.root_cancel.is_cancelled() {
            return BrowserServiceStatus::Unavailable {
                reason: "mock backend cancelled".into(),
            };
        }
        BrowserServiceStatus::Available {
            provider_id: "mock".into(),
            capabilities: Self::descriptor_inner().capabilities,
        }
    }

    pub async fn negotiate(
        &self,
        request: BrowserNegotiateRequest,
    ) -> Result<BrowserNegotiateResult, BackendError> {
        if !protocol_compatible(&request.protocol_version, BROWSER_PROVIDER_PROTOCOL_VERSION) {
            return Err(BackendError::msg(format!(
                "incompatible browser protocol version: requested {}, provider {}",
                request.protocol_version, BROWSER_PROVIDER_PROTOCOL_VERSION
            )));
        }
        let provider = Self::descriptor_inner();
        let offered = provider.capabilities.clone();
        let missing_required: Vec<BrowserCapability> = request
            .required_capabilities
            .iter()
            .copied()
            .filter(|cap| !offered.contains(cap))
            .collect();
        let mut granted: Vec<BrowserCapability> = request
            .required_capabilities
            .iter()
            .copied()
            .filter(|cap| offered.contains(cap))
            .collect();
        for cap in &request.optional_capabilities {
            if offered.contains(cap) && !granted.contains(cap) {
                granted.push(*cap);
            }
        }
        if request.required_capabilities.is_empty() && request.optional_capabilities.is_empty() {
            granted = offered;
        }
        Ok(BrowserNegotiateResult {
            protocol_version: BROWSER_PROVIDER_PROTOCOL_VERSION.into(),
            compatible: missing_required.is_empty(),
            provider,
            granted_capabilities: granted,
            missing_required,
        })
    }

    pub async fn ensure_session(
        &self,
        request: BrowserSessionEnsureRequest,
    ) -> Result<BrowserSessionHandle, BackendError> {
        if self.root_cancel.is_cancelled() {
            return Err(BackendError::msg("backend cancelled"));
        }
        let session_id = format!("mock-sess-{}", request.client_session_id);
        let mut sessions = self.sessions.lock().await;
        let entry = sessions
            .entry(session_id.clone())
            .or_insert_with(|| SessionState {
                cancel: self.root_cancel.child_token(),
                page_counter: 0,
                pages: HashMap::new(),
                default_page_id: "page_0".into(),
            });
        if entry.cancel.is_cancelled() {
            *entry = SessionState {
                cancel: self.root_cancel.child_token(),
                page_counter: 0,
                pages: HashMap::new(),
                default_page_id: "page_0".into(),
            };
        }
        Ok(BrowserSessionHandle {
            session_id,
            browser_family: Some("mock".into()),
            default_page_id: Some("page_0".into()),
        })
    }

    pub async fn close_session(&self, session_id: &str) -> Result<(), BackendError> {
        let mut sessions = self.sessions.lock().await;
        let Some(state) = sessions.remove(session_id) else {
            return Err(BackendError::msg(format!(
                "unknown browser session: {session_id}"
            )));
        };
        state.cancel.cancel();
        Ok(())
    }

    pub async fn navigate(
        &self,
        request: BrowserNavigateRequest,
    ) -> Result<BrowserNavigateResult, BackendError> {
        let mut sessions = self.sessions.lock().await;
        let state = sessions.get_mut(&request.session_id).ok_or_else(|| {
            BackendError::msg(format!("unknown browser session: {}", request.session_id))
        })?;
        if state.cancel.is_cancelled() {
            return Err(BackendError::msg("session cancelled"));
        }
        let page_id = if let Some(existing) = request.page_id.filter(|_| !request.new_page) {
            existing
        } else {
            state.page_counter += 1;
            format!("page_{}", state.page_counter)
        };
        let title = Some("Mock Page".to_string());
        state
            .pages
            .insert(page_id.clone(), (request.url.clone(), title.clone()));
        state.default_page_id = page_id.clone();
        Ok(BrowserNavigateResult {
            page_id,
            url: request.url,
            title,
        })
    }

    pub async fn page_state(
        &self,
        session_id: &str,
        page_id: Option<&str>,
    ) -> Result<BrowserPageState, BackendError> {
        let sessions = self.sessions.lock().await;
        let state = sessions
            .get(session_id)
            .ok_or_else(|| BackendError::msg(format!("unknown browser session: {session_id}")))?;
        if state.cancel.is_cancelled() {
            return Err(BackendError::msg("session cancelled"));
        }
        let pid = page_id
            .map(str::to_string)
            .unwrap_or_else(|| state.default_page_id.clone());
        let (url, title) = state
            .pages
            .get(&pid)
            .cloned()
            .unwrap_or_else(|| ("about:blank".into(), Some("Mock Blank".into())));
        Ok(BrowserPageState {
            session_id: session_id.into(),
            page_id: pid,
            url,
            title,
            ready: true,
        })
    }

    pub async fn fetch_rendered(
        &self,
        request: BrowserFetchRenderedRequest,
    ) -> Result<BrowserFetchRenderedResult, BackendError> {
        let sessions = self.sessions.lock().await;
        let state = sessions.get(&request.session_id).ok_or_else(|| {
            BackendError::msg(format!("unknown browser session: {}", request.session_id))
        })?;
        if state.cancel.is_cancelled() {
            return Err(BackendError::msg("session cancelled"));
        }
        let page_id = request
            .page_id
            .clone()
            .unwrap_or_else(|| state.default_page_id.clone());
        let url = request
            .url
            .clone()
            .or_else(|| state.pages.get(&page_id).map(|(u, _)| u.clone()))
            .unwrap_or_else(|| "about:blank".into());
        let title = state
            .pages
            .get(&page_id)
            .and_then(|(_, t)| t.clone())
            .or_else(|| Some("Mock Page".into()));
        Ok(BrowserFetchRenderedResult {
            page_id,
            url: url.clone(),
            title,
            content_type: "text/html; charset=utf-8".into(),
            text: format!(
                "<!doctype html><html><head><title>Mock</title></head>\
                 <body><h1>Mock rendered</h1><p>fixture for {url}</p></body></html>"
            ),
        })
    }

    pub async fn click(
        &self,
        request: BrowserActionRequest,
    ) -> Result<BrowserActionResult, BackendError> {
        self.require_live_session(&request.session_id).await?;
        Ok(BrowserActionResult {
            ok: true,
            detail: format!(
                "mock click ok selector={:?}",
                request.selector.unwrap_or_default()
            ),
        })
    }

    pub async fn type_text(
        &self,
        request: BrowserActionRequest,
    ) -> Result<BrowserActionResult, BackendError> {
        self.require_live_session(&request.session_id).await?;
        Ok(BrowserActionResult {
            ok: true,
            detail: format!(
                "mock type ok chars={}",
                request.text.as_deref().unwrap_or("").len()
            ),
        })
    }

    pub async fn wait(
        &self,
        request: BrowserActionRequest,
    ) -> Result<BrowserActionResult, BackendError> {
        self.require_live_session(&request.session_id).await?;
        Ok(BrowserActionResult {
            ok: true,
            detail: format!(
                "mock wait ok timeout_ms={:?}",
                request.timeout_ms.unwrap_or(0)
            ),
        })
    }

    pub async fn cancel(
        &self,
        session_id: Option<&str>,
    ) -> Result<BrowserActionResult, BackendError> {
        match session_id {
            Some(id) => {
                let mut sessions = self.sessions.lock().await;
                if let Some(state) = sessions.remove(id) {
                    state.cancel.cancel();
                    Ok(BrowserActionResult {
                        ok: true,
                        detail: format!("cancelled session {id}"),
                    })
                } else {
                    Ok(BrowserActionResult {
                        ok: true,
                        detail: format!("no session {id} (already gone)"),
                    })
                }
            }
            None => {
                self.root_cancel.cancel();
                let mut sessions = self.sessions.lock().await;
                for (_, state) in sessions.drain() {
                    state.cancel.cancel();
                }
                Ok(BrowserActionResult {
                    ok: true,
                    detail: "cancelled all sessions".into(),
                })
            }
        }
    }

    async fn require_live_session(&self, session_id: &str) -> Result<(), BackendError> {
        let sessions = self.sessions.lock().await;
        let state = sessions
            .get(session_id)
            .ok_or_else(|| BackendError::msg(format!("unknown browser session: {session_id}")))?;
        if state.cancel.is_cancelled() {
            return Err(BackendError::msg("session cancelled"));
        }
        Ok(())
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Optional CDP path. Without the `cdp` feature this only reports misconfiguration.
///
/// Chromium launch is allowlisted to `BROWSER_BIN` only (process.spawn-equivalent).
/// No filesystem.write, secrets, or arbitrary process.shell.
pub struct CdpBackend {
    browser_bin: Option<String>,
}

impl CdpBackend {
    pub fn from_env() -> Self {
        Self {
            browser_bin: std::env::var("BROWSER_BIN").ok().filter(|s| !s.is_empty()),
        }
    }

    pub fn descriptor(&self) -> BrowserProviderDescriptor {
        BrowserProviderDescriptor {
            provider_id: "cdp".into(),
            provider_version: Some(env!("CARGO_PKG_VERSION").into()),
            protocol_version: BROWSER_PROVIDER_PROTOCOL_VERSION.into(),
            browser_families: vec!["chromium".into()],
            capabilities: vec![BrowserCapability::Navigate],
        }
    }

    pub async fn status(&self) -> BrowserServiceStatus {
        match (&self.browser_bin, cfg!(feature = "cdp")) {
            (None, _) => BrowserServiceStatus::Misconfigured {
                reason: "BROWSER_BIN not set; CDP backend needs allowlisted browser binary path"
                    .into(),
            },
            (Some(_), false) => BrowserServiceStatus::Misconfigured {
                reason: "rebuild with --features cdp to enable chromiumoxide CDP backend".into(),
            },
            (Some(_path), true) => BrowserServiceStatus::Available {
                provider_id: "cdp".into(),
                capabilities: vec![BrowserCapability::Navigate],
            },
        }
    }

    pub async fn negotiate(
        &self,
        request: BrowserNegotiateRequest,
    ) -> Result<BrowserNegotiateResult, BackendError> {
        if !protocol_compatible(&request.protocol_version, BROWSER_PROVIDER_PROTOCOL_VERSION) {
            return Err(BackendError::msg("incompatible protocol version"));
        }
        let status = self.status().await;
        if !matches!(
            status,
            BrowserServiceStatus::Available { .. } | BrowserServiceStatus::Degraded { .. }
        ) {
            return Err(BackendError::msg(format!("cdp not ready: {status:?}")));
        }
        let provider = self.descriptor();
        Ok(BrowserNegotiateResult {
            protocol_version: BROWSER_PROVIDER_PROTOCOL_VERSION.into(),
            compatible: true,
            granted_capabilities: provider.capabilities.clone(),
            missing_required: Vec::new(),
            provider,
        })
    }

    pub async fn ensure_session(
        &self,
        _request: BrowserSessionEnsureRequest,
    ) -> Result<BrowserSessionHandle, BackendError> {
        let _ = &self.browser_bin;
        #[cfg(feature = "cdp")]
        {
            // Feature-gated hook for chromiumoxide launch via BROWSER_BIN only.
            // Full CDP session lifecycle lands when the optional feature is wired.
            return Err(BackendError::msg(
                "CDP session open scaffold: chromiumoxide feature present but session API not wired yet; use --mock",
            ));
        }
        #[cfg(not(feature = "cdp"))]
        {
            Err(BackendError::msg(
                "CDP session open not available; use --mock or rebuild with --features cdp and set BROWSER_BIN",
            ))
        }
    }

    pub async fn close_session(&self, _session_id: &str) -> Result<(), BackendError> {
        Err(BackendError::msg("cdp session close unavailable"))
    }

    pub async fn navigate(
        &self,
        _request: BrowserNavigateRequest,
    ) -> Result<BrowserNavigateResult, BackendError> {
        Err(BackendError::msg("cdp navigate unavailable"))
    }

    pub async fn page_state(
        &self,
        _session_id: &str,
        _page_id: Option<&str>,
    ) -> Result<BrowserPageState, BackendError> {
        Err(BackendError::msg("cdp page_state unavailable"))
    }

    pub async fn fetch_rendered(
        &self,
        _request: BrowserFetchRenderedRequest,
    ) -> Result<BrowserFetchRenderedResult, BackendError> {
        Err(BackendError::msg("cdp fetch_rendered unavailable"))
    }

    pub async fn click(
        &self,
        _request: BrowserActionRequest,
    ) -> Result<BrowserActionResult, BackendError> {
        Err(BackendError::msg("cdp click unavailable"))
    }

    pub async fn type_text(
        &self,
        _request: BrowserActionRequest,
    ) -> Result<BrowserActionResult, BackendError> {
        Err(BackendError::msg("cdp type unavailable"))
    }

    pub async fn wait(
        &self,
        _request: BrowserActionRequest,
    ) -> Result<BrowserActionResult, BackendError> {
        Err(BackendError::msg("cdp wait unavailable"))
    }

    pub async fn cancel(
        &self,
        _session_id: Option<&str>,
    ) -> Result<BrowserActionResult, BackendError> {
        Ok(BrowserActionResult {
            ok: true,
            detail: "cdp cancel no-op".into(),
        })
    }
}
