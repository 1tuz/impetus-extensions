//! Allowlist of LSP methods exposable via `lsp_request`.

pub const ALLOWED_REQUEST_METHODS: &[&str] = &[
    "textDocument/hover",
    "textDocument/definition",
    "textDocument/typeDefinition",
    "textDocument/implementation",
    "textDocument/references",
    "textDocument/documentSymbol",
    "textDocument/completion",
    "textDocument/formatting",
    "textDocument/rangeFormatting",
    "textDocument/rename",
    "textDocument/prepareRename",
    "textDocument/codeAction",
    "textDocument/diagnostic",
    "workspace/symbol",
    "workspace/diagnostic",
];

pub fn is_allowed_request(method: &str) -> bool {
    ALLOWED_REQUEST_METHODS.contains(&method)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_covers_convenience_tools() {
        for m in [
            "textDocument/hover",
            "textDocument/definition",
            "textDocument/references",
            "textDocument/documentSymbol",
            "workspace/symbol",
        ] {
            assert!(is_allowed_request(m), "{m}");
        }
        assert!(!is_allowed_request("workspace/executeCommand"));
        assert!(!is_allowed_request("initialize"));
    }
}
