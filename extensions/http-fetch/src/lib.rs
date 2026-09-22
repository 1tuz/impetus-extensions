//! HTTP fetch tools with SSRF-oriented guards for Impetus MCP.

#![allow(clippy::collapsible_if)]

use anyhow::{Context, Result, anyhow, bail};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{Client, Method};
use serde_json::{Value, json};
use std::time::Duration;
use tracing::info;
use url::Url;

const DEFAULT_MAX_BYTES: u64 = 1024 * 1024;
const HARD_MAX_BYTES: u64 = 8 * 1024 * 1024;
const HARD_MAX_TIMEOUT_MS: u64 = 120_000;

pub fn is_loopback_host(host: &str) -> bool {
    let h = host
        .trim()
        .trim_matches(|c| c == '[' || c == ']')
        .to_ascii_lowercase();
    matches!(h.as_str(), "localhost" | "127.0.0.1" | "::1" | "0.0.0.0")
        || h.ends_with(".localhost")
        || h.starts_with("127.")
}

pub fn validate_url(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).map_err(|e| anyhow!("invalid URL: {e}"))?;
    match url.scheme() {
        "http" | "https" => {}
        "file" => bail!("file:// URLs are blocked"),
        other => bail!("scheme `{other}` is not allowlisted (http/https only)"),
    }
    if url.username() != "" || url.password().is_some() {
        bail!("URLs with embedded credentials are blocked");
    }
    Ok(url)
}

pub fn validate_method(raw: &str) -> Result<Method> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "GET" => Ok(Method::GET),
        "HEAD" => Ok(Method::HEAD),
        "POST" => Ok(Method::POST),
        other => bail!("method `{other}` is not allowlisted (GET/HEAD/POST)"),
    }
}

pub fn require_timeout_ms(args: &Value) -> Result<u64> {
    let Some(v) = args.get("timeout_ms") else {
        bail!("timeout_ms is required");
    };
    let ms = v
        .as_u64()
        .ok_or_else(|| anyhow!("timeout_ms must be a positive integer"))?;
    if ms == 0 {
        bail!("timeout_ms must be > 0");
    }
    Ok(ms.min(HARD_MAX_TIMEOUT_MS))
}

pub fn max_bytes(args: &Value) -> u64 {
    args.get("max_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_MAX_BYTES)
        .clamp(1, HARD_MAX_BYTES)
}

pub fn allow_localhost_redirects(args: &Value) -> bool {
    args.get("allow_localhost_redirects")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Redact sensitive header values for logs / tool output metadata.
pub fn redact_header_value(name: &str, value: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if lower == "authorization"
        || lower == "proxy-authorization"
        || lower == "cookie"
        || lower == "set-cookie"
        || lower.ends_with("-api-key")
        || lower.contains("token")
        || lower.contains("secret")
    {
        return "[REDACTED]".into();
    }
    value.to_string()
}

pub fn redact_headers_map(headers: &HeaderMap) -> Value {
    let mut obj = serde_json::Map::new();
    for (name, value) in headers.iter() {
        let v = value.to_str().unwrap_or("<binary>");
        obj.insert(
            name.as_str().to_string(),
            Value::String(redact_header_value(name.as_str(), v)),
        );
    }
    Value::Object(obj)
}

fn parse_request_headers(args: &Value) -> Result<HeaderMap> {
    let mut map = HeaderMap::new();
    let Some(obj) = args.get("headers").and_then(|v| v.as_object()) else {
        return Ok(map);
    };
    for (k, v) in obj {
        let name = HeaderName::from_bytes(k.as_bytes())
            .map_err(|e| anyhow!("invalid header name `{k}`: {e}"))?;
        let val = v
            .as_str()
            .ok_or_else(|| anyhow!("header `{k}` value must be a string"))?;
        let hv = HeaderValue::from_str(val)
            .map_err(|e| anyhow!("invalid header value for `{k}`: {e}"))?;
        info!(
            header = %name,
            value = %redact_header_value(name.as_str(), val),
            "request header"
        );
        map.insert(name, hv);
    }
    Ok(map)
}

fn redirect_policy(allow_localhost: bool) -> Policy {
    Policy::custom(move |attempt| {
        let scheme = attempt.url().scheme().to_string();
        let host = attempt.url().host_str().map(str::to_string);
        let hops = attempt.previous().len();
        match scheme.as_str() {
            "http" | "https" => {}
            other => {
                return attempt.error(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("redirect scheme `{other}` blocked"),
                ));
            }
        }
        if !allow_localhost {
            if let Some(ref host) = host {
                if is_loopback_host(host) {
                    return attempt.error(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "redirect to localhost/loopback blocked (set allow_localhost_redirects=true to permit)",
                    ));
                }
            }
        }
        if hops >= 5 {
            attempt.stop()
        } else {
            attempt.follow()
        }
    })
}

async fn execute(method: Method, args: Value) -> Result<String> {
    let url_raw = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("url is required"))?;
    let url = validate_url(url_raw)?;
    let timeout_ms = require_timeout_ms(&args)?;
    let max = max_bytes(&args);
    let allow_local = allow_localhost_redirects(&args);
    let headers = parse_request_headers(&args)?;
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if (method == Method::GET || method == Method::HEAD) && body.is_some() {
        bail!("body is not allowed for {method}");
    }

    let client = Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .redirect(redirect_policy(allow_local))
        .build()
        .context("build HTTP client")?;

    let mut builder = client.request(method.clone(), url.clone()).headers(headers);
    if let Some(b) = body {
        if b.len() as u64 > max {
            bail!("request body exceeds max_bytes");
        }
        builder = builder.body(b);
    }

    info!(
        method = %method,
        url = %url,
        timeout_ms,
        allow_localhost_redirects = allow_local,
        "http request"
    );

    let response = builder.send().await.context("HTTP request failed")?;
    let status = response.status();
    let resp_headers = redact_headers_map(response.headers());
    let final_url = response.url().clone();
    if !allow_local {
        if let Some(host) = final_url.host_str() {
            if is_loopback_host(host) {
                bail!(
                    "final URL resolved to localhost/loopback and allow_localhost_redirects is false"
                );
            }
        }
    }

    let bytes = if method == Method::HEAD {
        Vec::new()
    } else {
        let raw = response.bytes().await.context("read body")?;
        if raw.len() as u64 > max {
            bail!("response body {} bytes exceeds max_bytes {max}", raw.len());
        }
        raw.to_vec()
    };

    let lossy = String::from_utf8_lossy(&bytes);
    let utf8_lossy = lossy.as_bytes() != bytes.as_slice();
    let out = json!({
        "status": status.as_u16(),
        "url": final_url.as_str(),
        "headers": resp_headers,
        "body": lossy.as_ref(),
        "utf8_lossy": utf8_lossy,
        "bytes": bytes.len(),
    });
    Ok(serde_json::to_string_pretty(&out)?)
}

pub async fn http_get(args: Value) -> Result<String> {
    execute(Method::GET, args).await
}

pub async fn http_request(args: Value) -> Result<String> {
    let method_raw = args
        .get("method")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("method is required"))?;
    let method = validate_method(method_raw)?;
    execute(method, args).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

    #[test]
    fn blocks_file_scheme() {
        assert!(
            validate_url("file:///etc/passwd")
                .unwrap_err()
                .to_string()
                .contains("file://")
        );
    }

    #[test]
    fn allows_http_https() {
        assert!(validate_url("https://example.com/a").is_ok());
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn method_allowlist() {
        assert!(validate_method("GET").is_ok());
        assert!(validate_method("post").is_ok());
        assert!(validate_method("DELETE").is_err());
        assert!(validate_method("PUT").is_err());
    }

    #[test]
    fn timeout_required() {
        assert!(require_timeout_ms(&json!({})).is_err());
        assert_eq!(
            require_timeout_ms(&json!({"timeout_ms": 1000})).unwrap(),
            1000
        );
        assert!(require_timeout_ms(&json!({"timeout_ms": 0})).is_err());
    }

    #[test]
    fn loopback_detection() {
        assert!(is_loopback_host("localhost"));
        assert!(is_loopback_host("127.0.0.1"));
        assert!(is_loopback_host("::1"));
        assert!(is_loopback_host("127.1.2.3"));
        assert!(!is_loopback_host("example.com"));
        assert!(!is_loopback_host("10.0.0.1"));
    }

    #[test]
    fn redacts_authorization() {
        assert_eq!(
            redact_header_value("Authorization", "Bearer secret"),
            "[REDACTED]"
        );
        assert_eq!(
            redact_header_value("Content-Type", "text/plain"),
            "text/plain"
        );
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer xyz"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let redacted = redact_headers_map(&headers);
        assert_eq!(redacted["authorization"], "[REDACTED]");
        assert_eq!(redacted["content-type"], "application/json");
    }

    #[test]
    fn localhost_redirect_flag_defaults_off() {
        assert!(!allow_localhost_redirects(&json!({})));
        assert!(allow_localhost_redirects(
            &json!({"allow_localhost_redirects": true})
        ));
    }
}
