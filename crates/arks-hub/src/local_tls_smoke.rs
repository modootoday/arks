//! Local HTTPS forge smoke Partial: ephemeral self-signed TLS + ureq client path.
//!
//! Exercises the remote HTTPS client against a loopback self-signed cert.
//! Design ≠ Claimed in-process TLS product / Claimed forge remote e2e.
//! Never spawns `gh` / `openssl` / reverse-proxy CLIs.

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use std::io::{Cursor, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Smoke-only localhost cert (CN=localhost, SAN DNS:localhost + IP:127.0.0.1).
/// Generated for Partial harness; not a product trust root / Claimed TLS story.
const SMOKE_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIDJTCCAg2gAwIBAgIUEczqHqDl1X1F33SWFtl/NqLeZMAwDQYJKoZIhvcNAQEL
BQAwFDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MTExMDIyMzI0NloXDTM2MTEw
NzIyMzI0NlowFDESMBAGA1UEAwwJbG9jYWxob3N0MIIBIjANBgkqhkiG9w0BAQEF
AAOCAQ8AMIIBCgKCAQEAyWo993afSmtmUkc1ZD0Dc9syIfOxV2YJzzFSJEPvjosQ
pq2+Li/MMl9GrNEe/B93F/sFJZuNNljjDXFrTEdhKddBVi8Jwi91kncYHYMpMUyU
fSPlFm7nWUzl4NBlwT2cmjc0+LnX8RED5yB0eYZwyg4ytxi2NaQ7B0N3LLzf9N3G
nqiihsu0WVykTe/rrANDoWHgFuu9C4839nSy65mvAiqhFfSID5Y5wsmR+940kadE
+nUz+6OWg488uE/f09GZcfCNnKtuSrdVzYnEqUfZ4M6poDbnqTe6GErfbD/gKeLt
TUERf9WtN1FJnD9BauImTtK0JG4HOTweKliP0aOQHwIDAQABo28wbTAdBgNVHQ4E
FgQUQO+fuX+G8WoKQCP2448uzVwubAswHwYDVR0jBBgwFoAUQO+fuX+G8WoKQCP2
448uzVwubAswDwYDVR0TAQH/BAUwAwEB/zAaBgNVHREEEzARgglsb2NhbGhvc3SH
BH8AAAEwDQYJKoZIhvcNAQELBQADggEBAJjICIBF994W75pAOc6bqeiWEXf17JqW
0pZxeOKNv2s9tFnshUwnXbYB/p2hGEhQ0M3yYCOSqoBy/g3ISqVALSGZ5xVTI5LO
vR2SJPZfpwVVSYsPKfTh2krlqv7/mnToqwfrlnN6g/uF+b81Ei72ATMnWS598x3b
ChC+GouRFUVTehLmmL9Bl0HJdEVupcoWtRqKL/IsLohZNSze5r+E18kBRSIHDeN0
ANAJUVu+lP9xlB0bN8gw7covpngRhfzr9t9D9gNq5ubhCFESumw9DRNoMZH8o+CY
Bd8It6iVgCjS6i3Qm3bqRkv8+HBBjE4jsKq4uV5Lw9NGzqNrUXTXf+I=
-----END CERTIFICATE-----
"#;

const SMOKE_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQDJaj33dp9Ka2ZS
RzVkPQNz2zIh87FXZgnPMVIkQ++OixCmrb4uL8wyX0as0R78H3cX+wUlm402WOMN
cWtMR2Ep10FWLwnCL3WSdxgdgykxTJR9I+UWbudZTOXg0GXBPZyaNzT4udfxEQPn
IHR5hnDKDjK3GLY1pDsHQ3csvN/03caeqKKGy7RZXKRN7+usA0OhYeAW670Ljzf2
dLLrma8CKqEV9IgPljnCyZH73jSRp0T6dTP7o5aDjzy4T9/T0Zlx8I2cq25Kt1XN
icSpR9ngzqmgNuepN7oYSt9sP+Ap4u1NQRF/1a03UUmcP0Fq4iZO0rQkbgc5PB4q
WI/Ro5AfAgMBAAECggEAA5LW658E9ltrCbJAdOcNqEWBao+DHBE5MJ4e53o/GH62
KCJZK/nmaaj0qv0A3fK8r2qYP/9awLF05mrNiHy4ldKIZSnZNEMGlxLjcG6cGpBR
xdnF+Wprm/fKYlit78pJs4VWhZ+CgK7PY2rG7R+g4WO9Pi/6IMkXGZiI6bOBnyl9
hJXg5QSRjYiat4I/yvF7mqGhVt22wkCALiT1nEOoFkgqvEY/WpKem1TskeoRqDo3
qp8AvgoMMUIzG+94Hhc5EPnUwUK93Vh6MM+6ayW1IbcX311RBA6NVuD6eRsRoBH/
8gwZOkJ1d5mGjtk7DWrqg8WX9ogAFduTeGwkM28bKQKBgQD2C/WsTnJMBmwMZgKZ
xrf5Ej9Mpsip+TPDAEiQwWuq9uPq6v16OnIAhQkQUCjGVXUbRSHUiwpK5y/ku+GD
tskTG1CbcOMDCUPtNrxr8kUazmZ3MpN+zFBUEsCvMgIRwteDj3NiC/oAc+LUG5VV
iiBEJ8oWrTZinBBsLYj5VvSG+QKBgQDRkBR3Te7hmC1hnIN/ZSmktKL7v3pDHxnV
F8NsGvYYoH1KvgE/YH5XCVzTozu+l27fQDW+eii9+txolBbJeRh/wK+zFybDBIxX
dojzoCIm+TI8/VSSN9jL4IOlymf6p9q8/8NvPnWgpEO6kQSWK9bUTcohzVb9gveq
QDPkIdwd1wKBgEWD04E/GKPKHLTUDWAM+OVx9lhdtsvvEGTDq7zaKOVAo9w3UC48
XQEwlQL64spGUrPFsG0wa1QGRU3Gl0xzVBQF35xLJJgzAfH/SPTmmgec0O5UZ7+n
Ufqu4amd41/JQMJ2wQ6ue4+tLiTnNiXYxIz9AOevQHJTzYACzXRE0QlhAoGAaHDc
IlFu2nPTOjCcwKMUTAfSqpDVSBp571+2f2w4djGV/+gOCx32GT6LStaxcd1a3lzL
7DBJmdUqf9w+Nw7pDNJlPmYtZUghAX4L5w9Jtg6/4PGIco7utz0fn0Lu61c4hrUZ
G5oPWY5ESvj3Rjh3CrvyH0KNNi9Na0YwFmRhpWUCgYAycaTr82FUERMsG18qzAXe
RP4j7W0zxcs/xtZfhAT64xemNhZSbPlaDyWus94KNbQCc16GCp52ogsmykkWLj2c
MXxntXPBUH1bxQ7N1CZueXO967rchyoDgLQ3Co04jx0yj8vX4znzJgPo34dW1/s7
AIpvLZDqaUMUpOcnRVa+Kw==
-----END PRIVATE KEY-----
"#;

#[derive(Debug)]
struct InsecureServerVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureServerVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn load_smoke_certs() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), String> {
    let mut cert_cursor = Cursor::new(SMOKE_CERT_PEM.as_bytes());
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_cursor)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("smoke cert parse: {e}"))?;
    if certs.is_empty() {
        return Err("smoke cert PEM empty".into());
    }
    let mut key_cursor = Cursor::new(SMOKE_KEY_PEM.as_bytes());
    let key = rustls_pemfile::private_key(&mut key_cursor)
        .map_err(|e| format!("smoke key parse: {e}"))?
        .ok_or_else(|| "smoke key PEM missing".to_string())?;
    Ok((certs, key))
}

fn forge_like_response(path: &str) -> (u16, &'static str, String) {
    let body = if path.starts_with("/user") {
        r#"{"login":"arks-local-tls-smoke","id":1,"partial":true,"claimed":false}"#.to_string()
    } else {
        format!(r#"{{"ok":true,"path":{path:?},"partial":true}}"#)
    };
    (200, "application/json", body)
}

fn handle_tls_client(stream: TcpStream, config: Arc<rustls::ServerConfig>) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let conn = match rustls::ServerConnection::new(config) {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut tls = rustls::StreamOwned::new(conn, stream);
    // Complete handshake before HTTP read (Partial smoke stub).
    if tls.conn.complete_io(&mut tls.sock).is_err() {
        return;
    }
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        match tls.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                if buf.len() > 16_384 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        return;
    }
    let req = String::from_utf8_lossy(&buf);
    let path = req
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/user");
    let (status, ctype, body) = forge_like_response(path);
    let resp = format!(
        "HTTP/1.1 {status} OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\nX-Arks-Hub-Tls-Smoke: partial\r\n\r\n{body}",
        body.len()
    );
    let _ = tls.write_all(resp.as_bytes());
    let _ = tls.flush();
}

/// ureq HTTPS GET that accepts the smoke self-signed cert (Partial only).
pub fn https_get_insecure(url: &str, token: Option<&str>) -> Result<(u16, String), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureServerVerifier))
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(10))
        .timeout_write(Duration::from_secs(10))
        .redirects(0)
        .user_agent("arks-hub/0.0.0 (+Partial local-tls smoke; design≠Claimed)")
        .tls_config(Arc::new(config))
        .build();
    let mut req = agent.get(url).set("Accept", "application/json");
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    let resp = req
        .call()
        .map_err(|e| format!("local-tls https Partial: {e}"))?;
    let status = resp.status();
    let body = resp
        .into_string()
        .map_err(|e| format!("local-tls body: {e}"))?;
    Ok((status, body))
}

/// Spawn ephemeral self-signed HTTPS stub; hit it via insecure ureq client path.
pub fn run_local_https_smoke(path: &str) -> Result<(SocketAddr, u16, String), String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let (certs, key) = load_smoke_certs()?;
    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("smoke ServerConfig: {e}"))?;
    let server_config = Arc::new(server_config);

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let _ = listener.set_nonblocking(false);
    let ready = Arc::new(AtomicBool::new(false));
    let ready2 = ready.clone();
    let cfg = server_config.clone();
    // One-shot accept for smoke: avoid racing stop against the client handshake.
    let handle = thread::spawn(move || {
        ready2.store(true, Ordering::SeqCst);
        match listener.accept() {
            Ok((stream, _)) => handle_tls_client(stream, cfg),
            Err(_) => {}
        }
    });

    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let url = format!("https://127.0.0.1:{}{path}", addr.port());
    while !ready.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(5));
    }
    thread::sleep(Duration::from_millis(20));
    let result = https_get_insecure(&url, Some("local-tls-smoke-token"));
    let _ = handle.join();
    let (status, body) = result?;
    Ok((addr, status, body))
}
