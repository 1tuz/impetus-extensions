use impetus_ext_lsp::session::{LspSession, StartConfig};
use impetus_ext_lsp::{allowlist, parse_args_list};
use std::path::PathBuf;

#[tokio::test]
async fn mock_session_initialize_diagnostics_and_hover() {
    let session = LspSession::new(true, None, vec![], Some(PathBuf::from(".")));
    let started = session
        .start(StartConfig {
            command: String::new(),
            args: vec![],
            workspace: PathBuf::from("."),
            mock: true,
        })
        .await
        .expect("start mock");
    assert_eq!(started["started"], true);
    assert_eq!(started["mock"], true);
    assert!(
        started["initialize"]["capabilities"]["hoverProvider"]
            .as_bool()
            .unwrap_or(false)
    );

    let diags = session.diagnostics(None).await.expect("diags");
    let map = diags["diagnostics"].as_object().expect("diag map");
    assert!(!map.is_empty(), "expected mock publishDiagnostics");

    let hover = session
        .hover("file:///tmp/x.rs", 0, 0)
        .await
        .expect("hover");
    assert!(hover.get("contents").is_some());

    let def = session
        .definition("file:///tmp/x.rs", 0, 0)
        .await
        .expect("definition");
    assert!(def.get("uri").is_some() || def.as_array().is_some());

    let refs = session
        .references("file:///tmp/x.rs", 0, 0)
        .await
        .expect("references");
    assert!(refs.as_array().is_some());

    let syms = session
        .symbols(Some("file:///tmp/x.rs"), None)
        .await
        .expect("symbols");
    assert!(syms.as_array().is_some());

    let cancelled = session.cancel(99).await.expect("cancel");
    assert_eq!(cancelled["cancelled"], 99);

    let stopped = session.stop().await.expect("stop");
    assert_eq!(stopped["stopped"], true);
}

#[tokio::test]
async fn restart_reuses_prior_config() {
    let session = LspSession::new(true, None, vec![], Some(PathBuf::from(".")));
    session
        .start(StartConfig {
            command: String::new(),
            args: vec![],
            workspace: PathBuf::from("."),
            mock: true,
        })
        .await
        .unwrap();
    let again = session.restart(None).await.expect("restart");
    assert_eq!(again["started"], true);
    let _ = session.stop().await;
}

#[tokio::test]
async fn allowlist_rejects_dangerous_methods() {
    let session = LspSession::new(true, None, vec![], Some(PathBuf::from(".")));
    session
        .start(StartConfig {
            command: String::new(),
            args: vec![],
            workspace: PathBuf::from("."),
            mock: true,
        })
        .await
        .unwrap();
    let err = session
        .request("workspace/executeCommand", serde_json::json!({}))
        .await
        .expect_err("must reject");
    assert!(err.to_string().contains("allowlist"));
    assert!(!allowlist::is_allowed_request("initialize"));
    let _ = session.stop().await;
}

#[test]
fn parse_args_json_and_whitespace() {
    assert_eq!(
        parse_args_list(Some(r#"["--stdio","--foo"]"#)),
        vec!["--stdio".to_string(), "--foo".to_string()]
    );
    assert_eq!(
        parse_args_list(Some("--stdio --foo")),
        vec!["--stdio".to_string(), "--foo".to_string()]
    );
}

#[test]
fn package_meta_and_mcp_json_shape() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let meta =
        impetus_ext_support::load_package_meta(&root.join("package.toml")).expect("package.toml");
    assert_eq!(meta.id, "lsp");
    assert_eq!(meta.compatibility.impetus_tag, "v0.1.2");
    assert_eq!(meta.compatibility.extension_api_version, "0.1.0-skill-mcp");
    assert!(!meta.permissions.contains(&"network".to_string()));
    assert!(!meta.permissions.contains(&"secrets_provider".to_string()));
    assert!(meta.permissions.contains(&"process_spawn".to_string()));
    assert!(meta.permissions.contains(&"filesystem_read".to_string()));

    let mcp: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("mcp.json")).unwrap()).unwrap();
    assert_eq!(mcp["schema_version"], 1);
    assert_eq!(mcp["id"], "lsp");
    assert_eq!(mcp["transport"], "stdio");
    assert_eq!(mcp["command"], "impetus-ext-lsp");
    assert!(mcp["args"].is_array());
    assert_eq!(mcp["capabilities"]["tools"], true);

    let preset: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("presets/rust-analyzer.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(preset["command"], "rust-analyzer");
}
