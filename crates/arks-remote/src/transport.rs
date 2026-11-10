//! HTTP exchange abstraction and an offline recorded script.

use crate::error::RemoteError;

/// One smart-HTTP request (absolute URL or origin-form target).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// One smart-HTTP response. `body` is the entity body, not the header block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Client transport. Production uses [`crate::http::UreqTransport`]; tests use a script or mock.
pub trait Transport {
    fn exchange(&mut self, request: &HttpRequest) -> Result<HttpResponse, RemoteError>;
}

/// One scripted response. The next exchange must match method and `url_contains`.
#[derive(Debug, Clone)]
pub struct RecordedExchange {
    pub method: String,
    pub url_contains: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Offline transport that replays a script and records the requests it saw.
#[derive(Debug)]
pub struct RecordedTransport {
    script: Vec<RecordedExchange>,
    index: usize,
    pub requests: Vec<HttpRequest>,
}

impl RecordedTransport {
    pub fn new(script: Vec<RecordedExchange>) -> Self {
        Self {
            script,
            index: 0,
            requests: Vec::new(),
        }
    }
}

impl Transport for RecordedTransport {
    fn exchange(&mut self, request: &HttpRequest) -> Result<HttpResponse, RemoteError> {
        let step = self.script.get(self.index).ok_or_else(|| {
            RemoteError::Protocol(format!(
                "recorded script ended before {} {}",
                request.method, request.url
            ))
        })?;
        if request.method != step.method || !request.url.contains(&step.url_contains) {
            return Err(RemoteError::Protocol(format!(
                "recorded script expected {} containing '{}', got {} {}",
                step.method, step.url_contains, request.method, request.url
            )));
        }
        let response = HttpResponse {
            status: step.status,
            headers: step.headers.clone(),
            body: step.body.clone(),
        };
        self.index += 1;
        self.requests.push(request.clone());
        Ok(response)
    }
}

pub(crate) fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

pub(crate) fn require_success(response: &HttpResponse, what: &str) -> Result<(), RemoteError> {
    if (200..300).contains(&response.status) {
        Ok(())
    } else {
        Err(RemoteError::Http(format!(
            "{what} returned HTTP {}",
            response.status
        )))
    }
}
