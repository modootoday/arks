//! HTTPS client policy shared by `arks-remote` and `arks-hub`.
//!
//! Trust anchors: built-in web roots **plus** PEM bundles named by
//! `GIT_SSL_CAINFO` (env, wins) or `http.sslCAInfo`, and `SSL_CERT_FILE`.
//! Proxy: `http.proxy` config, else `https_proxy` / `HTTPS_PROXY` /
//! `all_proxy` / `ALL_PROXY` env; `no_proxy` / `NO_PROXY` exclusions.
//! HTTPS travels through HTTP `CONNECT`.
//!
//! Primary: <https://git-scm.com/docs/git-config#Documentation/git-config.txt-httpproxy>,
//! <https://git-scm.com/docs/git-config#Documentation/git-config.txt-httpsslCAInfo>,
//! <https://git-scm.com/docs/git#_environment_variables> (`GIT_SSL_CAINFO`).
//! Cite: `.spec/sot/cloud-agent-dropin.sot.md` §T.

#![forbid(unsafe_code)]

use arks_config::ConfigSet;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Resolved HTTPS policy for one process.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpsPolicy {
    pub ca_files: Vec<PathBuf>,
    pub proxy: Option<String>,
    pub no_proxy: Vec<String>,
}

impl HttpsPolicy {
    /// Resolve from merged config + environment.
    pub fn resolve(cfg: &ConfigSet) -> Self {
        let mut ca_files = Vec::new();
        if let Some(p) = env_nonempty("GIT_SSL_CAINFO")
            .map(PathBuf::from)
            .or_else(|| cfg.get_path("http.sslcainfo"))
        {
            ca_files.push(p);
        }
        if let Some(p) = env_nonempty("SSL_CERT_FILE").map(PathBuf::from) {
            if !ca_files.contains(&p) {
                ca_files.push(p);
            }
        }
        let proxy = cfg
            .get("http.proxy")
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                ["https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY"]
                    .iter()
                    .find_map(|k| env_nonempty(k))
            });
        let no_proxy = ["no_proxy", "NO_PROXY"]
            .iter()
            .find_map(|k| env_nonempty(k))
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_ascii_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            ca_files,
            proxy,
            no_proxy,
        }
    }

    /// Resolve from config discovered at the current directory.
    pub fn from_environment() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::resolve(&ConfigSet::load_discovered(&cwd))
    }

    pub fn add_ca_file(&mut self, p: PathBuf) {
        if !self.ca_files.contains(&p) {
            self.ca_files.push(p);
        }
    }

    /// Proxy URL to use for `host`, honoring `no_proxy`.
    pub fn proxy_for(&self, host: &str) -> Option<String> {
        let proxy = self.proxy.as_ref()?;
        let host = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_ascii_lowercase();
        let excluded = self.no_proxy.iter().any(|pat| {
            if pat == "*" {
                return true;
            }
            let pat = pat.trim_start_matches("*.").trim_start_matches('.');
            let pat = pat.split('/').next().unwrap_or(pat);
            host == pat || host.ends_with(&format!(".{pat}"))
        });
        if excluded {
            return None;
        }
        Some(if proxy.contains("://") {
            proxy.clone()
        } else {
            format!("http://{proxy}")
        })
    }

    /// Apply trust anchors and proxy for `host` onto `builder`.
    pub fn apply(
        &self,
        mut builder: ureq::AgentBuilder,
        host: &str,
    ) -> Result<ureq::AgentBuilder, String> {
        if !self.ca_files.is_empty() {
            builder = builder.tls_config(self.tls_config()?);
        }
        if let Some(p) = self.proxy_for(host) {
            let proxy = ureq::Proxy::new(&p).map_err(|e| format!("http.proxy {p}: {e}"))?;
            builder = builder.proxy(proxy);
        }
        Ok(builder)
    }

    fn tls_config(&self) -> Result<Arc<rustls::ClientConfig>, String> {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut added = 0usize;
        for f in &self.ca_files {
            added += add_pem_file(&mut roots, f)?;
        }
        if added == 0 {
            return Err("CA bundle contains no certificates".into());
        }
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let cfg = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|e| format!("tls: {e}"))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Arc::new(cfg))
    }
}

fn env_nonempty(k: &str) -> Option<String> {
    env::var(k).ok().filter(|v| !v.trim().is_empty())
}

fn add_pem_file(roots: &mut rustls::RootCertStore, path: &Path) -> Result<usize, String> {
    let f = File::open(path).map_err(|e| format!("CA file {}: {e}", path.display()))?;
    let mut rd = BufReader::new(f);
    let mut n = 0;
    for cert in rustls_pemfile::certs(&mut rd) {
        let cert = cert.map_err(|e| format!("CA file {}: {e}", path.display()))?;
        // Bundles may carry certs rustls rejects (e.g. legacy encodings); skip those.
        if roots.add(cert).is_ok() {
            n += 1;
        }
    }
    Ok(n)
}

/// Host part of an `https://` / `http://` URL (no userinfo, no port).
pub fn url_host(url: &str) -> &str {
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let hostport = authority
        .rsplit_once('@')
        .map(|(_, h)| h)
        .unwrap_or(authority);
    if hostport.starts_with('[') {
        return hostport
            .split_once(']')
            .map(|(h, _)| &h[1..])
            .unwrap_or(hostport);
    }
    hostport.split(':').next().unwrap_or(hostport)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_proxy_suffix_and_star() {
        let p = HttpsPolicy {
            ca_files: vec![],
            proxy: Some("127.0.0.1:8080".into()),
            no_proxy: vec![".svc.local".into(), "localhost".into(), "*.corp".into()],
        };
        assert_eq!(
            p.proxy_for("github.com").as_deref(),
            Some("http://127.0.0.1:8080")
        );
        assert!(p.proxy_for("a.svc.local").is_none());
        assert!(p.proxy_for("LOCALHOST").is_none());
        assert!(p.proxy_for("x.corp").is_none());
        let star = HttpsPolicy {
            no_proxy: vec!["*".into()],
            ..p
        };
        assert!(star.proxy_for("github.com").is_none());
    }

    #[test]
    fn host_extraction() {
        assert_eq!(url_host("https://u:p@github.com:443/a/b"), "github.com");
        assert_eq!(url_host("https://[::1]:8443/x"), "::1");
        assert_eq!(url_host("http://h"), "h");
    }

    #[test]
    fn empty_bundle_is_error() {
        let dir = std::env::temp_dir().join(format!("arks-net-empty-{}", std::process::id()));
        std::fs::write(&dir, "not a pem\n").unwrap();
        let p = HttpsPolicy {
            ca_files: vec![dir.clone()],
            ..Default::default()
        };
        assert!(p.apply(ureq::AgentBuilder::new(), "x").is_err());
        let _ = std::fs::remove_file(dir);
    }
}
