//! `git://` daemon client (pack protocol over TCP).
//!
//! Connects to a git-daemon-compatible peer, sends the initial service
//! request pkt-line, then speaks gitprotocol-pack upload-pack / receive-pack
//! on the same duplex stream. Cite:
//! <https://git-scm.com/docs/git-daemon>,
//! <https://git-scm.com/docs/gitprotocol-pack> (Git Transport).

use crate::advertise::{Advertisement, parse_pack_advertisement};
use crate::error::RemoteError;
use crate::url::GitDaemonUrl;
use arks_protocol::{Packet, encode_packet};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Remote pack service for the initial `git://` request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitDaemonService {
    UploadPack,
    ReceivePack,
}

impl GitDaemonService {
    fn as_str(self) -> &'static str {
        match self {
            Self::UploadPack => "git-upload-pack",
            Self::ReceivePack => "git-receive-pack",
        }
    }
}

/// Live TCP session after the daemon accepted the service request.
pub struct GitDaemonSession {
    stream: TcpStream,
}

impl GitDaemonSession {
    /// Connect, send `git-upload-pack|git-receive-pack /path\0host=…\0`, return session.
    pub fn connect(url: &GitDaemonUrl, service: GitDaemonService) -> Result<Self, RemoteError> {
        let port = url.port.unwrap_or(9418);
        let dest = format!("{}:{}", url.host, port);
        let mut last_err = None;
        let mut stream = None;
        let addrs = dest
            .to_socket_addrs()
            .map_err(|e| RemoteError::Msg(format!("git:// resolve {dest}: {e}")))?;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, Duration::from_secs(30)) {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(e) => last_err = Some(e),
            }
        }
        let mut stream = stream.ok_or_else(|| {
            RemoteError::Msg(format!(
                "git:// connect {dest}: {}",
                last_err
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "no addresses".into())
            ))
        })?;
        stream
            .set_read_timeout(Some(Duration::from_secs(120)))
            .map_err(|e| RemoteError::Msg(format!("git:// set_read_timeout: {e}")))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(120)))
            .map_err(|e| RemoteError::Msg(format!("git:// set_write_timeout: {e}")))?;
        write_service_request(&mut stream, service, &url.path, &url.host)?;
        Ok(Self { stream })
    }

    /// Read the v1 pack-protocol ref advertisement (through flush-pkt).
    pub fn read_advertisement(&mut self) -> Result<Advertisement, RemoteError> {
        let body = read_until_flush(&mut self.stream)?;
        if body_starts_with_err(&body) {
            let msg = err_message_from_body(&body).unwrap_or_else(|| "git:// peer ERR".into());
            return Err(RemoteError::Msg(msg));
        }
        parse_pack_advertisement(&body)
    }

    pub fn write_all(&mut self, data: &[u8]) -> Result<(), RemoteError> {
        self.stream
            .write_all(data)
            .map_err(|e| RemoteError::Msg(format!("git:// write: {e}")))?;
        self.stream
            .flush()
            .map_err(|e| RemoteError::Msg(format!("git:// flush: {e}")))?;
        Ok(())
    }

    /// Half-close the write side (receive-pack peers that `read_to_end`).
    pub fn close_write(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Write);
    }

    /// Read remaining bytes until EOF (upload-pack exits after the pack).
    pub fn read_to_end(&mut self) -> Result<Vec<u8>, RemoteError> {
        let mut buf = Vec::new();
        self.stream
            .read_to_end(&mut buf)
            .map_err(|e| RemoteError::Msg(format!("git:// read: {e}")))?;
        Ok(buf)
    }

    /// Read pkt-lines until a flush-pkt (report-status).
    pub fn read_until_flush(&mut self) -> Result<Vec<u8>, RemoteError> {
        read_until_flush(&mut self.stream)
    }
}

fn write_service_request(
    stream: &mut TcpStream,
    service: GitDaemonService,
    path: &str,
    host: &str,
) -> Result<(), RemoteError> {
    // gitprotocol-pack Git Transport: "git-upload-pack /path\0host=hostname\0"
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let mut payload = Vec::new();
    payload.extend_from_slice(service.as_str().as_bytes());
    payload.push(b' ');
    payload.extend_from_slice(path.as_bytes());
    payload.push(0);
    payload.extend_from_slice(b"host=");
    payload.extend_from_slice(host.as_bytes());
    payload.push(0);
    let pkt = encode_packet(&Packet::Data(payload))
        .map_err(|e| RemoteError::Protocol(e.to_string()))?;
    stream
        .write_all(&pkt)
        .map_err(|e| RemoteError::Msg(format!("git:// service request: {e}")))?;
    stream
        .flush()
        .map_err(|e| RemoteError::Msg(format!("git:// service flush: {e}")))?;
    Ok(())
}

fn read_until_flush(reader: &mut impl Read) -> Result<Vec<u8>, RemoteError> {
    let mut out = Vec::new();
    loop {
        let mut len_bytes = [0u8; 4];
        reader
            .read_exact(&mut len_bytes)
            .map_err(|e| RemoteError::Protocol(format!("git:// pkt-len: {e}")))?;
        out.extend_from_slice(&len_bytes);
        let len = parse_hex4(&len_bytes)?;
        if len == 0 {
            break;
        }
        if len < 4 {
            return Err(RemoteError::Protocol(format!(
                "invalid pkt-len {len} on git:// stream"
            )));
        }
        let mut payload = vec![0u8; (len as usize) - 4];
        reader
            .read_exact(&mut payload)
            .map_err(|e| RemoteError::Protocol(format!("git:// pkt payload: {e}")))?;
        out.extend_from_slice(&payload);
    }
    Ok(out)
}

fn parse_hex4(bytes: &[u8; 4]) -> Result<u16, RemoteError> {
    let mut n = 0u16;
    for &b in bytes {
        let v = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            _ => {
                return Err(RemoteError::Protocol(
                    "pkt-len is not 4 lowercase hex digits".into(),
                ));
            }
        };
        n = (n << 4) | u16::from(v);
    }
    Ok(n)
}

fn body_starts_with_err(body: &[u8]) -> bool {
    // First data pkt after 4 hex len may be "ERR …"
    if body.len() < 8 {
        return false;
    }
    let len = match parse_hex4(body[..4].try_into().unwrap()) {
        Ok(n) => n as usize,
        Err(_) => return false,
    };
    if len < 4 || body.len() < len {
        return false;
    }
    let payload = &body[4..len];
    payload.starts_with(b"ERR ")
}

fn err_message_from_body(body: &[u8]) -> Option<String> {
    let len = parse_hex4(body.get(..4)?.try_into().ok()?).ok()? as usize;
    if len < 4 || body.len() < len {
        return None;
    }
    let payload = &body[4..len];
    let text = std::str::from_utf8(payload).ok()?.trim_end_matches('\n');
    Some(text.trim_start_matches("ERR ").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arks_protocol::decode_packet;

    #[test]
    fn service_request_roundtrips_pkt() {
        let mut payload = Vec::new();
        payload.extend_from_slice(b"git-upload-pack /repo.git");
        payload.push(0);
        payload.extend_from_slice(b"host=127.0.0.1");
        payload.push(0);
        let pkt = encode_packet(&Packet::Data(payload.clone())).unwrap();
        let (decoded, n) = decode_packet(&pkt).unwrap();
        assert_eq!(n, pkt.len());
        match decoded {
            Packet::Data(d) => assert_eq!(d, payload),
            _ => panic!("expected data"),
        }
    }
}
