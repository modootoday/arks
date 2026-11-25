//! `git://` initial request: `git-<service> <path>\0[host=<host>[:<port>]\0][\0<extra>\0...]`.
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-pack> (Git Transport) and
//! <https://git-scm.com/docs/gitprotocol-v2> (extra parameters after a second NUL).

use super::log::DaemonLog;
use super::path::PathStyle;
use crate::error::ServerError;
use std::io::Read;

/// Host attribute after sanitizing (safe to splice into a filesystem path).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct HostAttr {
    /// `%H`: lower-cased, sanitized host name.
    pub hostname: String,
    /// `%P`: sanitized port (empty when the client sent none).
    pub port: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ServiceRequest {
    /// Command word (`git-upload-pack`).
    pub command: String,
    /// Requested path, verbatim.
    pub path: String,
    /// Whole first field (`git-upload-pack /x.git`) for protocol-error logs.
    pub line: String,
    pub host: Option<HostAttr>,
    /// Extra parameters (`version=2`).
    pub extra: Vec<String>,
}

pub(super) fn read_service_request<R: Read>(
    stream: &mut R,
    log: &DaemonLog,
) -> Result<ServiceRequest, ServerError> {
    let mut lenbuf = [0u8; 4];
    stream.read_exact(&mut lenbuf).map_err(hung_up)?;
    let pkt_len = parse_pkt_len(&lenbuf)?;
    if !(5..=65_520).contains(&pkt_len) {
        return Err(ServerError::Protocol("bad service request length".into()));
    }
    let mut payload = vec![0u8; pkt_len - 4];
    stream.read_exact(&mut payload).map_err(hung_up)?;
    parse_service_payload(&payload, log)
}

/// git's fatal for a peer that closed before its request packet was complete.
pub(super) const HUNG_UP: &str = "the remote end hung up unexpectedly";

fn hung_up(e: std::io::Error) -> ServerError {
    if e.kind() == std::io::ErrorKind::UnexpectedEof {
        ServerError::Msg(HUNG_UP.into())
    } else {
        e.into()
    }
}

pub(super) fn parse_pkt_len(lenbuf: &[u8; 4]) -> Result<usize, ServerError> {
    let mut n = 0usize;
    for &b in lenbuf {
        n = (n << 4)
            + match b {
                b'0'..=b'9' => (b - b'0') as usize,
                b'a'..=b'f' => (b - b'a' + 10) as usize,
                b'A'..=b'F' => (b - b'A' + 10) as usize,
                _ => return Err(ServerError::Protocol("bad pkt-len hex".into())),
            };
    }
    Ok(n)
}

pub(super) fn parse_service_payload(
    payload: &[u8],
    log: &DaemonLog,
) -> Result<ServiceRequest, ServerError> {
    let (head, rest) = match payload.iter().position(|b| *b == 0) {
        Some(i) => (&payload[..i], &payload[i + 1..]),
        None => (payload, &[][..]),
    };
    let line = String::from_utf8_lossy(head)
        .trim_end_matches('\n')
        .to_string();
    let (command, path) = match line.split_once(' ') {
        Some((c, p)) => (c.to_string(), p.to_string()),
        None => (line.clone(), String::new()),
    };
    let mut host = None;
    let mut extra = Vec::new();
    // Host attribute(s) up to an empty field, then extra parameters.
    let mut fields = rest.split(|b| *b == 0);
    for field in fields.by_ref() {
        if field.is_empty() {
            break;
        }
        let s = String::from_utf8_lossy(field);
        if let Some(h) = s.strip_prefix("host=") {
            if host.is_some() {
                log.error("fatal: Invalid request");
                return Err(ServerError::Protocol("Invalid request".into()));
            }
            log.info(&format!("Extended attribute \"host\": {h}"));
            host = Some(parse_host_attr(h));
        }
    }
    for field in fields {
        if field.is_empty() {
            continue;
        }
        let s = String::from_utf8_lossy(field).to_string();
        log.info(&format!("Extended attribute \"protocol\": {s}"));
        extra.push(s);
    }
    Ok(ServiceRequest {
        command,
        path,
        line,
        host,
        extra,
    })
}

/// Split `host[:port]` (or `[v6]:port`), then sanitize and lower-case the host.
pub(super) fn parse_host_attr(value: &str) -> HostAttr {
    parse_host_attr_in(value, PathStyle::HOST)
}

fn parse_host_attr_in(value: &str, style: PathStyle) -> HostAttr {
    let (host, port) = if let Some(inner) = value.strip_prefix('[') {
        match inner.split_once(']') {
            Some((h, after)) => (h, after.strip_prefix(':').unwrap_or("")),
            None => (value, ""),
        }
    } else {
        match value.rsplit_once(':') {
            Some((h, p)) => (h, p),
            None => (value, ""),
        }
    };
    HostAttr {
        hostname: sanitize_client(host, style).to_ascii_lowercase(),
        port: sanitize_client(port, style),
    }
}

/// Make a client string safe inside a path: drop `/`, collapse runs of `.` and
/// strip leading/trailing dots (observed stock `git daemon` behaviour). Windows
/// also drops `\` and `:` (matrix SoT §Windows rule 5).
pub(super) fn sanitize_client(s: &str, style: PathStyle) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if style.is_unsafe_in_host(c) {
            continue;
        }
        if c == '.' && out.ends_with('.') {
            continue;
        }
        out.push(c);
    }
    out.trim_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(bytes: &[u8]) -> ServiceRequest {
        parse_service_payload(bytes, &DaemonLog::memory(true)).unwrap()
    }

    #[test]
    fn parses_command_path_host_and_extra() {
        let r = parse(b"git-upload-pack /a.git\0host=Example.COM:777\0\0version=2\0");
        assert_eq!(r.command, "git-upload-pack");
        assert_eq!(r.path, "/a.git");
        let h = r.host.unwrap();
        assert_eq!(h.hostname, "example.com");
        assert_eq!(h.port, "777");
        assert_eq!(r.extra, vec!["version=2".to_string()]);
    }

    #[test]
    fn host_sanitizing_matches_stock_git_daemon() {
        // Observed with stock git daemon: `../X/y...z..:12/3` → `xy.z` port `123`.
        let h = parse_host_attr_in("../X/y...z..:12/3", PathStyle::Posix);
        assert_eq!(h.hostname, "xy.z");
        assert_eq!(h.port, "123");
        assert_eq!(parse_host_attr_in(".a/b.", PathStyle::Posix).hostname, "ab");
        let v6 = parse_host_attr_in("[::1]:55", PathStyle::Posix);
        assert_eq!((v6.hostname.as_str(), v6.port.as_str()), ("::1", "55"));
        let multi = parse_host_attr_in("a:b:c", PathStyle::Posix);
        assert_eq!((multi.hostname.as_str(), multi.port.as_str()), ("a:b", "c"));
        assert_eq!(parse_host_attr_in("", PathStyle::Posix).hostname, "");
    }

    #[test]
    fn windows_host_sanitizing_drops_drive_and_unc_characters() {
        let w = PathStyle::Windows;
        let unc = parse_host_attr_in("\\\\evil\\share", w);
        assert_eq!(unc.hostname, "evilshare");
        let drive = parse_host_attr_in("[c:]:7", w);
        assert_eq!((drive.hostname.as_str(), drive.port.as_str()), ("c", "7"));
        assert_eq!(parse_host_attr_in("..\\..\\x", w).hostname, "x");
        assert_eq!(parse_host_attr_in("[::1]:55", w).hostname, "1");
        // Posix keeps both characters, as before.
        let posix = parse_host_attr_in("[c:]:7", PathStyle::Posix);
        assert_eq!(posix.hostname, "c:");
    }

    #[test]
    fn duplicate_host_is_invalid_request() {
        let log = DaemonLog::memory(false);
        assert!(parse_service_payload(b"git-upload-pack /a\0host=a\0host=b\0", &log).is_err());
        assert_eq!(log.lines(), vec!["fatal: Invalid request".to_string()]);
    }

    #[test]
    fn verbose_logs_extended_attributes() {
        let log = DaemonLog::memory(true);
        parse_service_payload(b"git-upload-pack /a\0host=h\0\0version=2\0", &log).unwrap();
        assert_eq!(
            log.lines(),
            vec![
                "Extended attribute \"host\": h".to_string(),
                "Extended attribute \"protocol\": version=2".to_string()
            ]
        );
    }

    #[test]
    fn request_without_nul_still_parses() {
        let r = parse(b"git-upload-pack /a.git\n");
        assert_eq!(r.path, "/a.git");
        assert!(r.host.is_none());
    }
}
