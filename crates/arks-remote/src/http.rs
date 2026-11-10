//! HTTP client for claimed smart HTTP (`http://` cleartext and `https://` via rustls).
//!
//! `ureq` is the minimal TLS-capable client already declared on the workspace.
//! Protocol bytes stay in this crate; the library only moves request and response bodies.
//!
//! On HTTP 401 without an Authorization header, fill credentials via helper/askpass
//! and retry once ([[sot:root/credential-helper-askpass]]).

use crate::credential::{CredentialSource, resolve_authorization};
use crate::error::RemoteError;
use crate::transport::{HttpRequest, HttpResponse, Transport};
use crate::url::prepare_url_with_source;
use std::io::Read;
use std::time::Duration;

pub struct UreqTransport {
    agent: ureq::Agent,
    source: CredentialSource,
}

impl UreqTransport {
    pub fn new() -> Self {
        Self::with_source(CredentialSource::discover(&[]))
    }

    pub fn with_source(source: CredentialSource) -> Self {
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout_connect(Duration::from_secs(15))
            .timeout_read(Duration::from_secs(120))
            .timeout_write(Duration::from_secs(120))
            .user_agent("arks/0.0.0")
            .build();
        Self { agent, source }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for UreqTransport {
    fn exchange(&mut self, request: &HttpRequest) -> Result<HttpResponse, RemoteError> {
        let prepared = prepare_url_with_source(&request.url, &self.source)?;
        match self.exchange_once(request, &prepared.url, prepared.authorization.as_deref()) {
            Ok(response) => Ok(response),
            Err(RemoteError::Http(msg))
                if msg.starts_with("HTTP 401") && prepared.authorization.is_none() =>
            {
                // Retry once after force-fill (askpass / helper may only run on challenge).
                let auth = resolve_authorization(&request.url, &self.source)?;
                if auth.is_none() {
                    return Err(RemoteError::Http(msg));
                }
                self.exchange_once(request, &prepared.url, auth.as_deref())
            }
            Err(err) => Err(err),
        }
    }
}

impl UreqTransport {
    fn exchange_once(
        &mut self,
        request: &HttpRequest,
        url: &str,
        authorization: Option<&str>,
    ) -> Result<HttpResponse, RemoteError> {
        let mut call = match request.method.as_str() {
            "GET" => self.agent.get(url),
            "POST" => self.agent.post(url),
            other => {
                return Err(RemoteError::Http(format!(
                    "unsupported HTTP method {other}"
                )));
            }
        };
        for (name, value) in &request.headers {
            call = call.set(name, value);
        }
        if let Some(auth) = authorization {
            call = call.set("Authorization", auth);
        }
        let result = if request.method == "POST" {
            call.send_bytes(&request.body)
        } else {
            call.call()
        };
        match result {
            Ok(response) => read_response(response),
            Err(ureq::Error::Status(code, response)) => {
                let mut snippet = String::new();
                let _ = response
                    .into_reader()
                    .take(512)
                    .read_to_string(&mut snippet);
                Err(RemoteError::Http(format!("HTTP {code}: {snippet}")))
            }
            Err(err) => Err(RemoteError::Http(err.to_string())),
        }
    }
}

fn read_response(response: ureq::Response) -> Result<HttpResponse, RemoteError> {
    let status = response.status();
    let mut headers = Vec::new();
    if let Some(ct) = response.header("content-type") {
        headers.push(("content-type".into(), ct.to_string()));
    }
    let mut body = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut body)
        .map_err(|e| RemoteError::Http(e.to_string()))?;
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}
