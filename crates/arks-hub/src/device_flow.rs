//! RFC 8628 OAuth 2.0 Device Authorization Grant (`arks auth login --web`).
//!
//! Primary sources:
//! - IETF RFC 8628: https://www.rfc-editor.org/info/rfc8628/
//! - GitHub Device Flow: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow
//!
//! Security invariants:
//! - Non-loopback endpoints MUST use `https://` (fail-closed on plain `http://`).
//! - Never prints `device_code` or `access_token` to terminal output.
//! - Never spawns a browser, `git`, or `gh`.

use serde_json::Value;
use std::io::Write;
use std::time::{Duration, Instant};

/// Default polling interval (seconds) per RFC 8628 §3.2 when omitted by the server.
pub const DEFAULT_INTERVAL_SECS: u64 = 5;
/// Mandatory backoff increment (seconds) on `slow_down` per RFC 8628 §3.5.
pub const SLOW_DOWN_INCREMENT_SECS: u64 = 5;
/// Default expiration (seconds) if omitted by the server.
pub const DEFAULT_EXPIRES_IN_SECS: u64 = 900;

/// Configuration for an RFC 8628 Device Authorization Grant session.
#[derive(Debug, Clone)]
pub struct DeviceAuthConfig {
    pub device_auth_url: String,
    pub token_url: String,
    pub client_id: String,
    pub scope: String,
}

impl DeviceAuthConfig {
    /// Resolve default endpoints for a forge hostname (`github.com` or custom HTTPS host),
    /// honoring explicit environment overrides for local fixtures or enterprise hosts.
    pub fn for_host(host: &str, client_id: String, scope: String) -> Self {
        let trimmed = host.trim().trim_end_matches('/');
        let origin = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            trimmed.to_string()
        } else {
            format!("https://{trimmed}")
        };
        let device_auth_url = std::env::var("ARKS_OAUTH_DEVICE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| format!("{origin}/login/device/code"));
        let token_url = std::env::var("ARKS_OAUTH_TOKEN_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| format!("{origin}/login/oauth/access_token"));
        Self {
            device_auth_url,
            token_url,
            client_id,
            scope,
        }
    }
}

/// Parsed response from RFC 8628 §3.2 Device Authorization Response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
}

/// Successful token issued at the end of RFC 8628 §3.5 polling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceToken {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

/// Outcome of a single RFC 8628 §3.5 token poll request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollOutcome {
    Approved(DeviceToken),
    Pending,
    SlowDown,
    Denied(String),
    Expired(String),
    Failed(String),
}

/// Enforce RFC 8628 §5.4 TLS transport policy: non-loopback OAuth endpoints MUST use HTTPS.
pub fn validate_oauth_endpoint(url: &str) -> Result<(), String> {
    if let Some(rest) = url.strip_prefix("https://") {
        if rest.is_empty() || rest.starts_with('/') {
            return Err(format!("invalid OAuth endpoint URL: {url}"));
        }
        return Ok(());
    }
    if let Some(rest) = url.strip_prefix("http://") {
        let authority = rest.split('/').next().unwrap_or("");
        let host = if authority.starts_with('[') {
            authority.split(']').next().map(|h| &h[1..]).unwrap_or("")
        } else {
            authority.split(':').next().unwrap_or("")
        };
        if matches!(host, "127.0.0.1" | "localhost" | "::1") {
            return Ok(());
        }
        return Err(format!(
            "refusing non-HTTPS OAuth endpoint {url:?} (RFC 8628 requires TLS for non-loopback hosts)"
        ));
    }
    Err(format!(
        "invalid OAuth endpoint scheme in {url:?}: expected https://"
    ))
}

/// Percent-encode a slice of `(key, value)` pairs as `application/x-www-form-urlencoded`.
pub fn encode_form(pairs: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (idx, (k, v)) in pairs.iter().enumerate() {
        if idx > 0 {
            out.push('&');
        }
        push_form_component(&mut out, k);
        out.push('=');
        push_form_component(&mut out, v);
    }
    out
}

fn push_form_component(out: &mut String, s: &str) {
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                use std::fmt::Write as _;
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
}

/// Parse RFC 8628 §3.2 Device Authorization Response JSON.
pub fn parse_device_code_response(status: u16, body: &str) -> Result<DeviceCodeResponse, String> {
    if !(200..300).contains(&status) {
        return Err(format!(
            "device authorization request failed (HTTP {status}): {}",
            truncate_err(body)
        ));
    }
    let v: Value = serde_json::from_str(body)
        .map_err(|e| format!("invalid JSON from device authorization endpoint: {e}"))?;
    if let Some(err) = v.get("error").and_then(Value::as_str) {
        let desc = v
            .get("error_description")
            .and_then(Value::as_str)
            .unwrap_or("");
        return Err(format!("device authorization error {err}: {desc}"));
    }
    let device_code = required_str(&v, "device_code")?;
    let user_code = required_str(&v, "user_code")?;
    let verification_uri = required_str(&v, "verification_uri")?;
    let verification_uri_complete = v
        .get("verification_uri_complete")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let expires_in = v
        .get("expires_in")
        .and_then(Value::as_u64)
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_EXPIRES_IN_SECS);
    let interval = v
        .get("interval")
        .and_then(Value::as_u64)
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_INTERVAL_SECS);
    Ok(DeviceCodeResponse {
        device_code,
        user_code,
        verification_uri,
        verification_uri_complete,
        expires_in,
        interval,
    })
}

/// Parse a single RFC 8628 §3.5 token poll response (HTTP 200 or 400).
pub fn parse_token_poll_response(status: u16, body: &str) -> PollOutcome {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return PollOutcome::Failed(format!(
                "invalid JSON from token endpoint (HTTP {status}): {e}"
            ));
        }
    };
    if let Some(err) = v.get("error").and_then(Value::as_str) {
        let desc = v
            .get("error_description")
            .and_then(Value::as_str)
            .unwrap_or("");
        return match err {
            "authorization_pending" => PollOutcome::Pending,
            "slow_down" => PollOutcome::SlowDown,
            "access_denied" => PollOutcome::Denied(if desc.is_empty() {
                "authorization was denied by the user".into()
            } else {
                desc.to_string()
            }),
            "expired_token" => PollOutcome::Expired(if desc.is_empty() {
                "device code expired before authorization completed".into()
            } else {
                desc.to_string()
            }),
            other => PollOutcome::Failed(format!("OAuth error {other}: {desc}")),
        };
    }
    if !(200..300).contains(&status) {
        return PollOutcome::Failed(format!(
            "token endpoint returned HTTP {status}: {}",
            truncate_err(body)
        ));
    }
    match v.get("access_token").and_then(Value::as_str) {
        Some(tok) if !tok.is_empty() => PollOutcome::Approved(DeviceToken {
            access_token: tok.to_string(),
            token_type: v
                .get("token_type")
                .and_then(Value::as_str)
                .unwrap_or("bearer")
                .to_string(),
            scope: v
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        }),
        _ => PollOutcome::Failed("token response missing access_token".into()),
    }
}

/// Run the full RFC 8628 Device Authorization Grant over live HTTP(S).
pub fn run_device_flow(
    cfg: &DeviceAuthConfig,
    prompt_out: &mut dyn Write,
) -> Result<DeviceToken, String> {
    run_device_flow_with(
        cfg,
        prompt_out,
        crate::http_min::http_post_form_json,
        std::thread::sleep,
    )
}

/// Deterministic, injectable RFC 8628 state machine driver (used by unit and fixture tests).
pub fn run_device_flow_with<F, S>(
    cfg: &DeviceAuthConfig,
    prompt_out: &mut dyn Write,
    mut post_form: F,
    mut sleep_fn: S,
) -> Result<DeviceToken, String>
where
    F: FnMut(&str, &str) -> Result<(u16, String), String>,
    S: FnMut(Duration),
{
    if cfg.client_id.trim().is_empty() {
        return Err("OAuth device login requires a client_id (set ARKS_OAUTH_CLIENT_ID)".into());
    }
    validate_oauth_endpoint(&cfg.device_auth_url)?;
    validate_oauth_endpoint(&cfg.token_url)?;

    let mut auth_params: Vec<(&str, &str)> = vec![("client_id", cfg.client_id.as_str())];
    if !cfg.scope.trim().is_empty() {
        auth_params.push(("scope", cfg.scope.as_str()));
    }
    let auth_body = encode_form(&auth_params);
    let (status, resp_body) = post_form(&cfg.device_auth_url, &auth_body)?;
    let code_resp = parse_device_code_response(status, &resp_body)?;

    // Print verification URI and user_code only — NEVER print device_code.
    let _ = writeln!(
        prompt_out,
        "! First copy your one-time code: {}",
        code_resp.user_code
    );
    let _ = writeln!(
        prompt_out,
        "Open this URL in your browser to authorize arks:"
    );
    let _ = writeln!(prompt_out, "  {}", code_resp.verification_uri);
    if let Some(complete) = &code_resp.verification_uri_complete {
        let _ = writeln!(prompt_out, "Direct link: {complete}");
    }
    let _ = writeln!(
        prompt_out,
        "Security notice (RFC 8628): confirm the code shown in your browser matches {} before approving.",
        code_resp.user_code
    );
    let _ = prompt_out.flush();

    let poll_body = encode_form(&[
        ("client_id", cfg.client_id.as_str()),
        ("device_code", code_resp.device_code.as_str()),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
    ]);

    let deadline = Instant::now() + Duration::from_secs(code_resp.expires_in);
    let mut interval_secs = code_resp.interval;

    loop {
        if Instant::now() >= deadline {
            return Err("device authorization timed out before approval".into());
        }
        sleep_fn(Duration::from_secs(interval_secs));
        if Instant::now() >= deadline {
            return Err("device authorization timed out before approval".into());
        }
        let (st, body) = post_form(&cfg.token_url, &poll_body)?;
        match parse_token_poll_response(st, &body) {
            PollOutcome::Approved(tok) => return Ok(tok),
            PollOutcome::Pending => continue,
            PollOutcome::SlowDown => {
                interval_secs = interval_secs.saturating_add(SLOW_DOWN_INCREMENT_SECS);
                continue;
            }
            PollOutcome::Denied(msg) => return Err(format!("device authorization denied: {msg}")),
            PollOutcome::Expired(msg) => {
                return Err(format!("device authorization expired: {msg}"));
            }
            PollOutcome::Failed(msg) => return Err(msg),
        }
    }
}

fn required_str(v: &Value, key: &str) -> Result<String, String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("device authorization response missing required field {key:?}"))
}

fn truncate_err(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.len() <= 200 {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..200])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    fn test_cfg() -> DeviceAuthConfig {
        DeviceAuthConfig {
            device_auth_url: "https://example.test/login/device/code".into(),
            token_url: "https://example.test/login/oauth/access_token".into(),
            client_id: "arks-test-client".into(),
            scope: "repo read:org".into(),
        }
    }

    #[test]
    fn validates_https_and_loopback_only() {
        assert!(validate_oauth_endpoint("https://github.com/login/device/code").is_ok());
        assert!(validate_oauth_endpoint("http://127.0.0.1:8080/device").is_ok());
        assert!(validate_oauth_endpoint("http://localhost:9000/token").is_ok());
        assert!(validate_oauth_endpoint("http://[::1]:9000/token").is_ok());
        assert!(validate_oauth_endpoint("http://github.com/login/device/code").is_err());
        assert!(validate_oauth_endpoint("ftp://github.com/login").is_err());
    }

    #[test]
    fn form_encoding_is_rfc3986_safe() {
        let encoded = encode_form(&[
            ("client_id", "id+1"),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("scope", "repo read:org"),
        ]);
        assert_eq!(
            encoded,
            "client_id=id%2B1&grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code&scope=repo+read%3Aorg"
        );
    }

    #[test]
    fn state_machine_handles_pending_slow_down_and_approval() {
        let cfg = test_cfg();
        let mut prompt = Vec::new();
        let mut sleeps = Vec::new();
        let mut step = 0usize;

        let tok = run_device_flow_with(
            &cfg,
            &mut prompt,
            |url, body| {
                step += 1;
                match step {
                    1 => {
                        assert_eq!(url, cfg.device_auth_url);
                        assert!(body.contains("client_id=arks-test-client"));
                        assert!(body.contains("scope=repo+read%3Aorg"));
                        Ok((
                            200,
                            r#"{"device_code":"SECRET_DEV_CODE","user_code":"WDJB-MJHT","verification_uri":"https://example.test/device","expires_in":600,"interval":5}"#.into(),
                        ))
                    }
                    2 => {
                        assert_eq!(url, cfg.token_url);
                        assert!(body.contains("device_code=SECRET_DEV_CODE"));
                        Ok((400, r#"{"error":"authorization_pending"}"#.into()))
                    }
                    3 => Ok((400, r#"{"error":"slow_down"}"#.into())),
                    4 => Ok((
                        200,
                        r#"{"access_token":"gho_test_token_123","token_type":"bearer","scope":"repo"}"#.into(),
                    )),
                    _ => unreachable!("unexpected poll {step}"),
                }
            },
            |d| sleeps.push(d.as_secs()),
        )
        .expect("device flow should succeed");

        assert_eq!(tok.access_token, "gho_test_token_123");
        assert_eq!(tok.token_type, "bearer");
        // Initial interval 5s -> pending (5s) -> slow_down (+5s => 10s) -> approved.
        assert_eq!(sleeps, vec![5, 5, 10]);

        let prompt_str = String::from_utf8(prompt).unwrap();
        assert!(prompt_str.contains("WDJB-MJHT"));
        assert!(prompt_str.contains("https://example.test/device"));
        assert!(
            !prompt_str.contains("SECRET_DEV_CODE"),
            "device_code must never be printed"
        );
        assert!(
            !prompt_str.contains("gho_test_token_123"),
            "access_token must never be printed"
        );
    }

    #[test]
    fn state_machine_fails_closed_on_denied_and_expired() {
        for (err_code, expect_substr) in [("access_denied", "denied"), ("expired_token", "expired")]
        {
            let cfg = test_cfg();
            let mut prompt = Vec::new();
            let mut step = 0usize;
            let err = run_device_flow_with(
                &cfg,
                &mut prompt,
                |_, _| {
                    step += 1;
                    if step == 1 {
                        Ok((
                            200,
                            r#"{"device_code":"d","user_code":"ABCD-EFGH","verification_uri":"https://example.test/device","expires_in":60,"interval":1}"#.into(),
                        ))
                    } else {
                        Ok((400, format!(r#"{{"error":"{err_code}"}}"#)))
                    }
                },
                |_| {},
            )
            .expect_err("must fail closed");
            assert!(err.contains(expect_substr), "unexpected err: {err}");
        }
    }

    #[test]
    fn loopback_http_fixture_end_to_end() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let polls = Arc::new(Mutex::new(0usize));
        let polls_srv = Arc::clone(&polls);

        let srv = std::thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..n]);
                let (status, reason, body) = if req.starts_with("POST /login/device/code ") {
                    (
                        200,
                        "OK",
                        r#"{"device_code":"dev_1","user_code":"1234-5678","verification_uri":"http://127.0.0.1/verify","expires_in":60,"interval":1}"#,
                    )
                } else {
                    let mut p = polls_srv.lock().unwrap();
                    *p += 1;
                    if *p == 1 {
                        (400, "Bad Request", r#"{"error":"authorization_pending"}"#)
                    } else {
                        (
                            200,
                            "OK",
                            r#"{"access_token":"tok_loopback_ok","token_type":"bearer","scope":"repo"}"#,
                        )
                    }
                };
                let resp = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(resp.as_bytes()).unwrap();
            }
        });

        let cfg = DeviceAuthConfig {
            device_auth_url: format!("http://{addr}/login/device/code"),
            token_url: format!("http://{addr}/login/oauth/access_token"),
            client_id: "arks-local-test".into(),
            scope: "repo".into(),
        };
        let mut prompt = Vec::new();
        let tok = run_device_flow_with(
            &cfg,
            &mut prompt,
            crate::http_min::http_post_form_json,
            |_| {},
        )
        .expect("loopback fixture device flow should succeed");
        srv.join().unwrap();
        assert_eq!(tok.access_token, "tok_loopback_ok");
        assert_eq!(*polls.lock().unwrap(), 2);
    }
}
