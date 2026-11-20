//! Line-oriented client connections for the mail bridges (`imap-send`,
//! `send-email`): plain TCP, TLS from the first byte (`imaps://`, SMTPS),
//! STARTTLS upgrades, or a tunnel command's stdin/stdout (`imap.tunnel`).
//! TLS uses rustls with the built-in web roots; certificate checks can be
//! turned off where git documents a `sslVerify` switch.

use crate::error::{CliError, CliResult};
use rustls::pki_types::ServerName;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::Arc;

type Tls = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

enum Inner {
    Plain(TcpStream),
    Tls(Box<Tls>),
    Tunnel(Child, ChildStdin, ChildStdout),
    Closed,
}

impl Read for Inner {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Inner::Plain(s) => s.read(buf),
            Inner::Tls(s) => s.read(buf),
            Inner::Tunnel(_, _, o) => o.read(buf),
            Inner::Closed => Ok(0),
        }
    }
}

impl Write for Inner {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Inner::Plain(s) => s.write(buf),
            Inner::Tls(s) => s.write(buf),
            Inner::Tunnel(_, i, _) => i.write(buf),
            Inner::Closed => Err(std::io::Error::other("connection closed")),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Inner::Plain(s) => s.flush(),
            Inner::Tls(s) => s.flush(),
            Inner::Tunnel(_, i, _) => i.flush(),
            Inner::Closed => Ok(()),
        }
    }
}

/// A buffered client connection.
pub struct Conn {
    r: BufReader<Inner>,
    host: String,
    verify: bool,
    /// Echo the conversation to stderr (`--smtp-debug`, `imap-send -v`).
    pub debug: bool,
}

fn net_err(what: &str, e: impl std::fmt::Display) -> CliError {
    CliError::Exit(1, format!("fatal: {what}: {e}"))
}

impl Conn {
    pub fn tcp(host: &str, port: u16, tls: bool, verify: bool) -> CliResult<Self> {
        let s = TcpStream::connect((host, port))
            .map_err(|e| net_err(&format!("unable to connect to {host}:{port}"), e))?;
        let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(120)));
        let inner = if tls {
            Inner::Tls(Box::new(tls_wrap(s, host, verify)?))
        } else {
            Inner::Plain(s)
        };
        Ok(Self {
            r: BufReader::new(inner),
            host: host.to_string(),
            verify,
            debug: false,
        })
    }

    /// Talk through `sh -c <command>` (its stdin/stdout carry the protocol).
    pub fn tunnel(command: &str) -> CliResult<Self> {
        let mut c = super::proc::shell(command, "sh", &[]);
        c.stdin(Stdio::piped()).stdout(Stdio::piped());
        let mut child = c
            .spawn()
            .map_err(|e| net_err(&format!("cannot start proxy {command}"), e))?;
        let i = child
            .stdin
            .take()
            .ok_or_else(|| net_err("tunnel", "no stdin"))?;
        let o = child
            .stdout
            .take()
            .ok_or_else(|| net_err("tunnel", "no stdout"))?;
        Ok(Self {
            r: BufReader::new(Inner::Tunnel(child, i, o)),
            host: String::new(),
            verify: true,
            debug: false,
        })
    }

    /// Switch a plain connection to TLS (STARTTLS).
    pub fn starttls(&mut self) -> CliResult<()> {
        let inner = std::mem::replace(self.r.get_mut(), Inner::Closed);
        let Inner::Plain(s) = inner else {
            *self.r.get_mut() = inner;
            return Ok(());
        };
        let tls = tls_wrap(s, &self.host, self.verify)?;
        self.r = BufReader::new(Inner::Tls(Box::new(tls)));
        Ok(())
    }

    pub fn is_tls(&self) -> bool {
        matches!(self.r.get_ref(), Inner::Tls(_))
    }

    /// Read one line without its CRLF; `None` at end of stream.
    pub fn line(&mut self) -> CliResult<Option<String>> {
        let mut buf = Vec::new();
        let n = self
            .r
            .read_until(b'\n', &mut buf)
            .map_err(|e| net_err("read", e))?;
        if n == 0 {
            return Ok(None);
        }
        while buf.last().is_some_and(|b| *b == b'\n' || *b == b'\r') {
            buf.pop();
        }
        let s = String::from_utf8_lossy(&buf).into_owned();
        if self.debug {
            eprintln!("S: {s}");
        }
        Ok(Some(s))
    }

    /// Send `data` as is.
    pub fn send(&mut self, data: &[u8]) -> CliResult<()> {
        let w = self.r.get_mut();
        w.write_all(data).map_err(|e| net_err("write", e))?;
        w.flush().map_err(|e| net_err("write", e))
    }

    /// Send one command line (CRLF appended); `shown` replaces it in debug output.
    pub fn cmd(&mut self, line: &str, shown: Option<&str>) -> CliResult<()> {
        if self.debug {
            eprintln!("C: {}", shown.unwrap_or(line));
        }
        self.send(format!("{line}\r\n").as_bytes())
    }

    pub fn close(mut self) {
        if let Inner::Tunnel(mut child, i, _) = std::mem::replace(self.r.get_mut(), Inner::Closed) {
            drop(i);
            let _ = child.wait();
        }
    }
}

fn tls_wrap(s: TcpStream, host: &str, verify: bool) -> CliResult<Tls> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| net_err("tls", e))?;
    let cfg = if verify {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        builder.with_root_certificates(roots).with_no_client_auth()
    } else {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerify(provider)))
            .with_no_client_auth()
    };
    let name = ServerName::try_from(host.to_string()).map_err(|e| net_err("tls server name", e))?;
    let conn = rustls::ClientConnection::new(Arc::new(cfg), name).map_err(|e| net_err("tls", e))?;
    let mut tls = rustls::StreamOwned::new(conn, s);
    tls.conn
        .complete_io(&mut tls.sock)
        .map_err(|e| net_err("tls handshake", e))?;
    Ok(tls)
}

/// `sslVerify = false`: accept any certificate (signatures still checked).
#[derive(Debug)]
struct NoVerify(Arc<rustls::crypto::CryptoProvider>);

impl rustls::client::danger::ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

/// HMAC-MD5 (RFC 2104 over RFC 1321) for CRAM-MD5 (RFC 2195).
pub fn hmac_md5(key: &[u8], msg: &[u8]) -> [u8; 16] {
    let mut k = if key.len() > 64 {
        md5(key).to_vec()
    } else {
        key.to_vec()
    };
    k.resize(64, 0);
    let mut inner: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    inner.extend_from_slice(msg);
    let ih = md5(&inner);
    let mut outer: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    outer.extend_from_slice(&ih);
    md5(&outer)
}

/// MD5 (RFC 1321).
pub fn md5(data: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64)
        .map(|i| ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32)
        .collect();
    let (mut a0, mut b0, mut c0, mut d0) =
        (0x67452301u32, 0xefcdab89u32, 0x98badcfeu32, 0x10325476u32);
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_le_bytes());
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk
            .chunks(4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    out[..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..].copy_from_slice(&d0.to_le_bytes());
    out
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_vectors() {
        assert_eq!(hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(&md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn cram_md5_rfc2195_example() {
        let challenge = b"<1896.697170952@postoffice.reston.mci.net>";
        assert_eq!(
            hex(&hmac_md5(b"tanstaaftanstaaf", challenge)),
            "b913a602c7eda7a495b4e6e7334d3890"
        );
    }
}
