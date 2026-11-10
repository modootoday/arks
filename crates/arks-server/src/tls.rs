//! Product in-process TLS terminate helpers (operator-supplied PEM).
//!
//! Used by `arks serve` HTTP (and exported for forge cohost). Design bar:
//! [[sot:root/arks-claimed-residual-gates]] B9. Ops reverse-proxy remains valid
//! but does not by itself satisfy B9. Never spawns openssl/git/gh.

use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

/// Load a rustls `ServerConfig` from operator-supplied PEM cert + key paths.
pub fn load_server_config(
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
) -> Result<Arc<ServerConfig>, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let certs = load_certs(cert_path.as_ref())?;
    let key = load_private_key(key_path.as_ref())?;
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("tls ServerConfig: {e}"))?;
    // Prefer HTTP/1.1 ALPN so stock curl/git/ureq negotiate cleanly.
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, String> {
    let f = File::open(path).map_err(|e| format!("tls cert open {}: {e}", path.display()))?;
    let mut reader = BufReader::new(f);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("tls cert parse {}: {e}", path.display()))?;
    if certs.is_empty() {
        return Err(format!("tls cert PEM empty: {}", path.display()));
    }
    Ok(certs)
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, String> {
    let f = File::open(path).map_err(|e| format!("tls key open {}: {e}", path.display()))?;
    let mut reader = BufReader::new(f);
    rustls_pemfile::private_key(&mut reader)
        .map_err(|e| format!("tls key parse {}: {e}", path.display()))?
        .ok_or_else(|| format!("tls key PEM missing: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn testdata(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join(name)
    }

    #[test]
    fn load_server_config_rejects_missing() {
        let err = load_server_config("/no/such/cert.pem", "/no/such/key.pem").unwrap_err();
        assert!(err.contains("tls cert open"), "{err}");
    }

    #[test]
    fn load_server_config_from_testdata_pems() {
        let cfg = load_server_config(testdata("localhost.crt.pem"), testdata("localhost.key.pem"))
            .expect("load testdata PEMs");
        assert_eq!(cfg.alpn_protocols, vec![b"http/1.1".to_vec()]);
    }
}
