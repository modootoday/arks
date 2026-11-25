//! `arks agent provider smoke` / `review --smoke` over `https://` (queue unit Q22;
//! transport contract in [[sot:root/arks-agent-llm-providers]] 3c).
//!
//! A loopback rustls server presents a leaf signed by a test CA generated at run
//! time (rcgen; no PEM fixture tracked). The smoke must succeed with that CA as a
//! trust anchor, succeed through a loopback HTTP `CONNECT` proxy, and fail with the
//! TLS error class and exit 1 without it. Host runs are advisory.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

/// Loopback TLS chat fixture; returns its port and the PEM of the CA that signed it.
fn spawn_tls_fixture() -> (u16, String) {
    let ca_key = KeyPair::generate().unwrap();
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "arks agent test CA");
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca = ca_params.self_signed(&ca_key).unwrap();

    let leaf_key = KeyPair::generate().unwrap();
    let mut leaf_params = CertificateParams::new(vec!["127.0.0.1".to_string()]).unwrap();
    leaf_params
        .distinguished_name
        .push(DnType::CommonName, "127.0.0.1");
    leaf_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let leaf = leaf_params.signed_by(&leaf_key, &ca, &ca_key).unwrap();

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der())),
        )
        .unwrap();
    let config = Arc::new(config);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for tcp in listener.incoming().flatten() {
            let config = Arc::clone(&config);
            thread::spawn(move || serve_tls(config, tcp));
        }
    });
    (port, ca.pem())
}

fn serve_tls(config: Arc<rustls::ServerConfig>, tcp: TcpStream) {
    let _ = tcp.set_read_timeout(Some(Duration::from_secs(10)));
    let Ok(conn) = rustls::ServerConnection::new(config) else {
        return;
    };
    let mut tls = rustls::StreamOwned::new(conn, tcp);
    let Some(request_line) = read_request(&mut tls) else {
        return; // handshake refused by the client, or a short read
    };
    let body = if request_line.starts_with("GET ") {
        r#"{"object":"list","data":[{"id":"arks-tls-fixture"}]}"#
    } else {
        r#"{"choices":[{"message":{"role":"assistant","content":"tls fixture ok"}}]}"#
    };
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = tls.write_all(resp.as_bytes());
    let _ = tls.flush();
    tls.conn.send_close_notify();
    let _ = tls.flush();
}

/// Read one HTTP/1.1 request (headers + Content-Length body); return its request line.
fn read_request(stream: &mut impl Read) -> Option<String> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let head_end = loop {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let len: usize = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse().ok())?
        })
        .unwrap_or(0);
    while buf.len() < head_end + len {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    head.lines().next().map(str::to_string)
}

/// Loopback HTTP `CONNECT` proxy; records each tunnel target.
fn spawn_connect_proxy() -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    thread::spawn(move || {
        for mut client in listener.incoming().flatten() {
            let log = Arc::clone(&log);
            thread::spawn(move || {
                let Some(line) = read_request(&mut client) else {
                    return;
                };
                let mut parts = line.split_whitespace();
                if parts.next() != Some("CONNECT") {
                    let _ = client.write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n");
                    return;
                }
                let target = parts.next().unwrap_or_default().to_string();
                log.lock().unwrap().push(target.clone());
                let Ok(upstream) = TcpStream::connect(&target) else {
                    let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n");
                    return;
                };
                if client
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .is_err()
                {
                    return;
                }
                let (mut c_rd, mut u_wr) =
                    (client.try_clone().unwrap(), upstream.try_clone().unwrap());
                let up = thread::spawn(move || {
                    let _ = std::io::copy(&mut c_rd, &mut u_wr);
                    let _ = u_wr.shutdown(std::net::Shutdown::Write);
                });
                let (mut u_rd, mut c_wr) = (upstream, client);
                let _ = std::io::copy(&mut u_rd, &mut c_wr);
                let _ = c_wr.shutdown(std::net::Shutdown::Write);
                let _ = up.join();
            });
        }
    });
    (port, seen)
}

/// Run `arks agent …` with a hermetic trust / proxy environment: no inherited CA
/// bundle, proxy or git config; `ca` (if any) becomes `GIT_SSL_CAINFO`.
fn agent(args: &[&str], ca: Option<&Path>, proxy: Option<u16>) -> Output {
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_arks"));
    cmd.arg("agent")
        .args(args)
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("GIT_CONFIG_GLOBAL")
        .env_remove("ARKS_AGENT_PROVIDER")
        .env_remove("ARKS_AGENT_BASE_URL")
        .env_remove("ARKS_AGENT_API_KEY")
        .env_remove("OPENAI_API_KEY");
    for k in [
        "GIT_SSL_CAINFO",
        "SSL_CERT_FILE",
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
        "no_proxy",
        "NO_PROXY",
    ] {
        cmd.env_remove(k);
    }
    if let Some(ca) = ca {
        cmd.env("GIT_SSL_CAINFO", ca);
    }
    if let Some(port) = proxy {
        cmd.env("HTTPS_PROXY", format!("http://127.0.0.1:{port}"));
    }
    cmd.output().unwrap()
}

fn write_ca(pem: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test-ca.pem");
    std::fs::write(&path, pem).unwrap();
    (dir, path)
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn provider_smoke_https_with_test_ca_answers() {
    let (port, ca_pem) = spawn_tls_fixture();
    let (_d, ca) = write_ca(&ca_pem);
    let url = format!("https://127.0.0.1:{port}/v1");
    let out = agent(
        &["provider", "smoke", "openai-compat", "--base-url", &url],
        Some(&ca),
        None,
    );
    let text = stdout(&out);
    assert!(out.status.success(), "{out:?}");
    assert!(text.contains("models_http=200"), "{text}");
    assert!(text.contains("chat_http=200"), "{text}");
    assert!(text.contains("tls fixture ok"), "{text}");
    assert!(text.contains("smoke_result=ok"), "{text}");
}

#[test]
fn provider_smoke_https_without_ca_is_a_tls_failure() {
    let (port, _ca_pem) = spawn_tls_fixture();
    let url = format!("https://127.0.0.1:{port}/v1");
    let out = agent(
        &["provider", "smoke", "openai-compat", "--base-url", &url],
        None,
        None,
    );
    let text = stdout(&out);
    eprintln!("{text}");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(text.contains("smoke_result=fail"), "{text}");
    assert!(text.contains("error_class=err_class=tls"), "{text}");
    assert!(text.contains("reason=TLS failure"), "{text}");
}

#[test]
fn review_smoke_https_through_connect_proxy() {
    let (port, ca_pem) = spawn_tls_fixture();
    let (proxy_port, seen) = spawn_connect_proxy();
    let (_d, ca) = write_ca(&ca_pem);
    let url = format!("https://127.0.0.1:{port}/v1");
    let out = agent(
        &["review", "--smoke", "--base-url", &url],
        Some(&ca),
        Some(proxy_port),
    );
    let text = stdout(&out);
    assert!(out.status.success(), "{out:?}");
    assert!(text.contains("review_smoke=ok"), "{text}");
    let seen = seen.lock().unwrap();
    assert!(
        seen.iter().any(|t| t == &format!("127.0.0.1:{port}")),
        "proxy saw {seen:?}"
    );
}

#[test]
fn review_smoke_https_without_ca_exits_1() {
    let (port, _ca_pem) = spawn_tls_fixture();
    let url = format!("https://127.0.0.1:{port}/v1");
    let out = agent(&["review", "--smoke", "--base-url", &url], None, None);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(text.contains("review_smoke=fail"), "{text}");
    assert!(text.contains("reason=TLS failure"), "{text}");
}
