//! Mock-backend integration tests (no Chrome).

use impetus_ext_browser::{
    BROWSER_PROVIDER_PROTOCOL_VERSION, Backend, BrowserCapability, BrowserNegotiateRequest,
    build_handler, tool_defs,
};
use impetus_ext_mcp::{McpServer, ServerInfo, ToolCall, ToolHandler};
use serde_json::json;

#[tokio::test]
async fn mock_lifecycle_negotiate_navigate_fetch_cancel() {
    let backend = Backend::mock();

    let status = backend.status().await;
    let status_json = serde_json::to_value(&status).unwrap();
    assert_eq!(status_json["status"], "available");

    let negotiated = backend
        .negotiate(BrowserNegotiateRequest {
            protocol_version: BROWSER_PROVIDER_PROTOCOL_VERSION.into(),
            required_capabilities: vec![BrowserCapability::Navigate],
            optional_capabilities: vec![BrowserCapability::Click],
        })
        .await
        .unwrap();
    assert!(negotiated.compatible);
    assert!(
        negotiated
            .granted_capabilities
            .contains(&BrowserCapability::Navigate)
    );

    let session = backend
        .ensure_session(impetus_ext_browser::BrowserSessionEnsureRequest {
            client_session_id: "t1".into(),
            browser_preference: None,
        })
        .await
        .unwrap();
    assert!(session.session_id.contains("t1"));

    let nav = backend
        .navigate(impetus_ext_browser::BrowserNavigateRequest {
            session_id: session.session_id.clone(),
            url: "https://example.test/page".into(),
            page_id: None,
            new_page: false,
        })
        .await
        .unwrap();
    assert_eq!(nav.url, "https://example.test/page");

    let state = backend
        .page_state(&session.session_id, Some(&nav.page_id))
        .await
        .unwrap();
    assert!(state.ready);
    assert_eq!(state.url, "https://example.test/page");

    let fetched = backend
        .fetch_rendered(impetus_ext_browser::BrowserFetchRenderedRequest {
            session_id: session.session_id.clone(),
            page_id: Some(nav.page_id.clone()),
            url: None,
        })
        .await
        .unwrap();
    assert!(fetched.text.contains("Mock rendered"));
    assert!(fetched.text.contains("example.test"));

    let click = backend
        .click(impetus_ext_browser::BrowserActionRequest {
            session_id: session.session_id.clone(),
            page_id: Some(nav.page_id.clone()),
            selector: Some("#btn".into()),
            text: None,
            timeout_ms: None,
        })
        .await
        .unwrap();
    assert!(click.ok);

    let typed = backend
        .type_text(impetus_ext_browser::BrowserActionRequest {
            session_id: session.session_id.clone(),
            page_id: None,
            selector: Some("input".into()),
            text: Some("hi".into()),
            timeout_ms: None,
        })
        .await
        .unwrap();
    assert!(typed.ok);

    let waited = backend
        .wait(impetus_ext_browser::BrowserActionRequest {
            session_id: session.session_id.clone(),
            page_id: None,
            selector: None,
            text: None,
            timeout_ms: Some(10),
        })
        .await
        .unwrap();
    assert!(waited.ok);

    let cancelled = backend.cancel(Some(&session.session_id)).await.unwrap();
    assert!(cancelled.ok);

    let closed = backend.close_session(&session.session_id).await;
    assert!(closed.is_err());
}

#[tokio::test]
async fn mcp_tools_list_and_status_call() {
    let backend = Backend::mock();
    let server = McpServer::new(
        ServerInfo {
            name: "browser".into(),
            version: "0.1.0".into(),
        },
        tool_defs(),
        build_handler(backend),
    );

    // tools/list + tools/call via handler surface
    assert!(tool_defs().iter().any(|t| t.name == "browser_status"));
    assert!(tool_defs().iter().any(|t| t.name == "browser_negotiate"));
    assert_eq!(tool_defs().len(), 11);

    let handler = build_handler(Backend::mock());
    let result = handler
        .call(ToolCall {
            name: "browser_status",
            arguments: json!({}),
        })
        .await
        .unwrap();
    assert!(!result.is_error);
    assert!(result.text.contains("available"));

    let nego = handler
        .call(ToolCall {
            name: "browser_negotiate",
            arguments: json!({}),
        })
        .await
        .unwrap();
    assert!(!nego.is_error);
    assert!(nego.text.contains("compatible"));

    let _ = server;
}

#[tokio::test]
async fn reject_incompatible_protocol_major() {
    let backend = Backend::mock();
    let err = backend
        .negotiate(BrowserNegotiateRequest {
            protocol_version: "1.0".into(),
            required_capabilities: vec![],
            optional_capabilities: vec![],
        })
        .await
        .unwrap_err();
    assert!(err.to_string().contains("incompatible"));
}

#[test]
fn extension_manifest_loads() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = impetus_ext_support::load_extension_dir(root).unwrap();
    assert_eq!(manifest.id.as_str(), "browser");
    assert!(matches!(
        manifest.entrypoint,
        impetus_ext_support::ExtensionEntrypoint::McpBridge { .. }
    ));
    assert!(
        manifest
            .permissions
            .contains(&impetus_ext_support::ExtensionPermission::Network)
    );
    assert!(
        !manifest
            .permissions
            .contains(&impetus_ext_support::ExtensionPermission::FilesystemRead)
    );
    assert!(
        !manifest
            .permissions
            .contains(&impetus_ext_support::ExtensionPermission::SecretsProvider)
    );
}
