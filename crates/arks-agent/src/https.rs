//! `https://` provider transport for `arks agent`.
//!
//! Reuses the HTTPS stack of the git remote client and `arks hub`: `ureq` +
//! `rustls` configured by [`arks_net::HttpsPolicy`] (built-in web roots plus
//! `GIT_SSL_CAINFO` / `http.sslCAInfo` / `SSL_CERT_FILE`; `http.proxy` /
//! `HTTPS_PROXY` with `NO_PROXY`). Redirects are not followed, so the API key
//! only reaches the configured base URL.
//!
//! Cite: [[sot:root/arks-agent-llm-providers]] 3c, [[sot:root/cloud-agent-dropin]] §T.

use std::io::Read;
use std::time::Duration;

/// Hint appended to TLS failures so the operator knows which anchors apply.
pub(crate) const TLS_HINT: &str = "certificate not trusted by the built-in roots or GIT_SSL_CAINFO / http.sslCAInfo / SSL_CERT_FILE";

fn agent_for(url: &str, read_timeout: Duration) -> Result<ureq::Agent, String> {
    let base = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(read_timeout)
        .timeout_write(Duration::from_secs(30))
        .user_agent("arks-agent/0.0.0");
    arks_net::HttpsPolicy::from_environment()
        .apply(base, arks_net::url_host(url))
        .map(ureq::AgentBuilder::build)
        .map_err(|e| format!("https policy: {e}"))
}

fn request(
    url: &str,
    method: &str,
    accept: &str,
    api_key: Option<&str>,
    read_timeout: Duration,
) -> Result<ureq::Request, String> {
    let agent = agent_for(url, read_timeout)?;
    let mut req = agent
        .request(method, url)
        .set("Accept", accept)
        .set("Accept-Encoding", "identity");
    if let Some(key) = api_key {
        req = req.set("Authorization", &format!("Bearer {key}"));
    }
    Ok(req)
}

/// Turn a ureq result into a response for any HTTP status, or a transport error.
fn settle(result: Result<ureq::Response, ureq::Error>) -> Result<ureq::Response, String> {
    match result {
        Ok(resp) => Ok(resp),
        Err(ureq::Error::Status(_, resp)) => Ok(resp),
        Err(ureq::Error::Transport(t)) => {
            // ureq's transport error text already names the URL.
            let msg = format!("https: {t}");
            if is_tls_message(&msg) {
                Err(format!("{msg} (TLS: {TLS_HINT})"))
            } else {
                Err(msg)
            }
        }
    }
}

fn read_body(resp: ureq::Response, max_chars: usize) -> Result<(u16, String), String> {
    let status = resp.status();
    let text = resp
        .into_string()
        .map_err(|e| format!("https body read: {e}"))?;
    Ok((status, text.chars().take(max_chars).collect()))
}

/// `GET url` with `Accept: application/json`.
pub(crate) fn get_json(
    url: &str,
    api_key: Option<&str>,
    max_chars: usize,
) -> Result<(u16, String), String> {
    let req = request(
        url,
        "GET",
        "application/json",
        api_key,
        Duration::from_secs(15),
    )?;
    read_body(settle(req.call())?, max_chars)
}

/// `POST url` with a JSON body; returns status and (capped) body text.
pub(crate) fn post_json(
    url: &str,
    body: &str,
    api_key: Option<&str>,
    max_chars: usize,
) -> Result<(u16, String), String> {
    let req = request(
        url,
        "POST",
        "application/json",
        api_key,
        Duration::from_secs(60),
    )?
    .set("Content-Type", "application/json");
    read_body(settle(req.send_string(body))?, max_chars)
}

/// `POST url` asking for `text/event-stream`; returns status and a body reader.
pub(crate) fn post_stream(
    url: &str,
    body: &str,
    api_key: Option<&str>,
) -> Result<(u16, Box<dyn Read + Send + Sync>), String> {
    let req = request(
        url,
        "POST",
        "text/event-stream",
        api_key,
        Duration::from_secs(60),
    )?
    .set("Content-Type", "application/json");
    let resp = settle(req.send_string(body))?;
    Ok((resp.status(), resp.into_reader()))
}

/// Host and port the reachability probe should dial for `https_url`: the proxy
/// when one applies to the host, else `None` (dial the origin).
pub(crate) fn probe_proxy(https_url: &str) -> Option<(String, u16)> {
    let proxy =
        arks_net::HttpsPolicy::from_environment().proxy_for(arks_net::url_host(https_url))?;
    proxy_endpoint(&proxy)
}

/// Host and port of a proxy URL such as `http://user@127.0.0.1:3128`.
fn proxy_endpoint(proxy: &str) -> Option<(String, u16)> {
    let (scheme, rest) = proxy.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let hostport = authority
        .rsplit_once('@')
        .map(|(_, h)| h)
        .unwrap_or(authority);
    let default_port = if scheme.eq_ignore_ascii_case("https") {
        443
    } else {
        80
    };
    let (host, port) = if let Some(v6) = hostport.strip_prefix('[') {
        let (h, tail) = v6.split_once(']')?;
        (h, tail.strip_prefix(':'))
    } else {
        match hostport.rsplit_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (hostport, None),
        }
    };
    let port = match port {
        Some(p) => p.parse().ok()?,
        None => default_port,
    };
    Some((host.to_string(), port))
}

/// True when a transport error message describes a TLS / certificate failure.
pub(crate) fn is_tls_message(msg: &str) -> bool {
    let e = msg.to_ascii_lowercase();
    e.contains("certificate")
        || e.contains("unknownissuer")
        || e.contains("tls")
        || e.contains("handshake")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_messages_are_recognised() {
        assert!(is_tls_message(
            "Connection Failed: tls connection init failed: invalid peer certificate: UnknownIssuer"
        ));
        assert!(!is_tls_message("connect 127.0.0.1:1: Connection refused"));
    }

    #[test]
    fn proxy_endpoint_parses_host_and_port() {
        assert_eq!(
            proxy_endpoint("http://127.0.0.1:3128"),
            Some(("127.0.0.1".into(), 3128))
        );
        assert_eq!(
            // Userinfo is dropped; `example.com` keeps the public-surface scan quiet.
            proxy_endpoint("http://u:p@example.com/"),
            Some(("example.com".into(), 80))
        );
        assert_eq!(
            proxy_endpoint("http://[::1]:8080"),
            Some(("::1".into(), 8080))
        );
    }
}
