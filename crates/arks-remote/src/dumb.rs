//! Dumb HTTP client fetch/clone path (additive beside smart HTTP).
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-http#_dumb_clients>
//! Flow: discover refs via dumb `info/refs` body (no smart service pkt-line),
//! read `objects/info/packs`, download listed packs, fetch missing loose
//! objects by OID path, update remote-tracking refs.
//!
//! Push over dumb HTTP is not claimed (receive-pack requires smart).

use crate::advertise::{Advertisement, AdvertisedRef};
use crate::config::{fetch_specs_for, map_refspec};
use crate::error::RemoteError;
use crate::fetch::{FetchOptions, FetchOutcome};
use crate::transport::{HttpRequest, Transport, require_success};
use crate::url::join_url;
use arks_object::{
    Commit, ObjectKind, RawObject, Tree, TreeMode, loose_rel_path, zlib_decompress,
};
use arks_oid::{Oid, OidKind};
use arks_repo::Repository;
use arks_store::{ObjectFormat, ObjectStore};
use std::collections::HashSet;
use std::path::PathBuf;

const FORMAT: ObjectFormat = ObjectFormat::Sha1;

/// Parse dumb `info/refs` body (TAB-separated; optional peeled `^{}` lines ignored).
pub(crate) fn parse_dumb_info_refs(body: &[u8]) -> Result<Advertisement, RemoteError> {
    let text = std::str::from_utf8(body).map_err(|e| RemoteError::Protocol(e.to_string()))?;
    let mut refs = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let (oid_hex, name) = line
            .split_once('\t')
            .ok_or_else(|| RemoteError::Protocol(format!("bad dumb info/refs line: {line}")))?;
        if name.ends_with("^{}") {
            // Peeled tag lines: keep direct tag entry only for claimed subset.
            continue;
        }
        let oid = Oid::from_hex(oid_hex.trim())
            .map_err(|e| RemoteError::Protocol(format!("dumb info/refs oid: {e}")))?;
        refs.push(AdvertisedRef {
            name: name.trim().to_string(),
            oid,
        });
    }
    Ok(Advertisement {
        capabilities: Vec::new(),
        refs,
    })
}

/// True when an info/refs response looks like dumb (not smart pkt-line).
pub(crate) fn looks_like_dumb_info_refs(content_type: Option<&str>, body: &[u8]) -> bool {
    if let Some(ct) = content_type {
        let lower = ct.to_ascii_lowercase();
        if lower.contains("application/x-git-") {
            return false;
        }
        if lower.contains("text/plain") {
            return true;
        }
    }
    // No/odd content-type: dumb bodies are text lines, not `# service=` pkt-line.
    if body.len() >= 5 && body[4] == b'#' {
        return false;
    }
    body.iter().any(|b| *b == b'\t')
}

pub(crate) fn fetch_dumb_with<T: Transport>(
    transport: &mut T,
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
    opts: &FetchOptions,
    ad: &Advertisement,
) -> Result<FetchOutcome, RemoteError> {
    if opts.depth.is_some() {
        return Err(RemoteError::NotImplemented(
            "clone/fetch --depth is not claimed for dumb HTTP (pack-protocol deepen only)".into(),
        ));
    }

    download_packs(transport, repo, url)?;

    let (head_branch, head_oid) = head_from_dumb(transport, url, ad)?;
    let mut wants: Vec<Oid> = Vec::new();
    for r in &ad.refs {
        if r.name.starts_with("refs/heads/") && !wants.contains(&r.oid) {
            wants.push(r.oid);
        }
    }
    if wants.is_empty() {
        if let Some(oid) = head_oid {
            wants.push(oid);
        }
    }

    for tip in &wants {
        ensure_object_closure(transport, repo, url, tip)?;
    }

    let specs = fetch_specs_for(&repo.git_dir().join("config"), remote_name);
    let mut updated = Vec::new();
    for r in &ad.refs {
        for spec in &specs {
            if let Some(dest) = map_refspec(spec, &r.name) {
                repo.refs_mut()
                    .update(&dest, &r.oid, None)
                    .map_err(|e| RemoteError::Msg(e.to_string()))?;
                updated.push((dest, r.oid));
                break;
            }
        }
    }
    if let Some(branch) = &head_branch {
        if let Some(rest) = branch.strip_prefix("refs/heads/") {
            let tracking = format!("refs/remotes/{remote_name}/{rest}");
            let sym = format!("refs/remotes/{remote_name}/HEAD");
            if let Some(oid) = head_oid {
                repo.refs_mut()
                    .update(&tracking, &oid, None)
                    .map_err(|e| RemoteError::Msg(e.to_string()))?;
            }
            repo.refs_mut()
                .set_symbolic(&sym, &tracking)
                .map_err(|e| RemoteError::Msg(e.to_string()))?;
        }
    }

    Ok(FetchOutcome {
        updated,
        head_branch,
        head_oid,
    })
}

fn head_from_dumb<T: Transport>(
    transport: &mut T,
    url: &str,
    ad: &Advertisement,
) -> Result<(Option<String>, Option<Oid>), RemoteError> {
    let head_url = join_url(url, "HEAD");
    if let Ok(resp) = transport.exchange(&HttpRequest {
        method: "GET".into(),
        url: head_url,
        headers: Vec::new(),
        body: Vec::new(),
    }) {
        if resp.status == 200 {
            if let Ok(text) = std::str::from_utf8(&resp.body) {
                let text = text.trim();
                if let Some(target) = text.strip_prefix("ref: ") {
                    let name = target.trim().to_string();
                    let oid = ad.refs.iter().find(|r| r.name == name).map(|r| r.oid);
                    return Ok((Some(name), oid));
                }
                if let Ok(oid) = Oid::from_hex(text) {
                    return Ok((None, Some(oid)));
                }
            }
        }
    }
    if let Some(r) = ad.refs.iter().find(|r| r.name.starts_with("refs/heads/")) {
        return Ok((Some(r.name.clone()), Some(r.oid)));
    }
    Ok((None, None))
}

fn download_packs<T: Transport>(
    transport: &mut T,
    repo: &mut Repository,
    url: &str,
) -> Result<(), RemoteError> {
    let packs_url = join_url(url, "objects/info/packs");
    let resp = transport.exchange(&HttpRequest {
        method: "GET".into(),
        url: packs_url,
        headers: Vec::new(),
        body: Vec::new(),
    })?;
    if resp.status == 404 {
        return Ok(());
    }
    require_success(&resp, "objects/info/packs")?;
    let text = std::str::from_utf8(&resp.body).map_err(|e| RemoteError::Protocol(e.to_string()))?;
    for line in text.lines() {
        let line = line.trim();
        let Some(name) = line.strip_prefix("P ") else {
            continue;
        };
        let name = name.trim();
        if !(name.starts_with("pack-") && name.ends_with(".pack")) {
            continue;
        }
        let pack_bytes = get_bytes(transport, url, &format!("objects/pack/{name}"))?;
        let idx_name = name.replacen(".pack", ".idx", 1);
        let _ = get_bytes(transport, url, &format!("objects/pack/{idx_name}"));
        repo.store_mut()
            .ingest_pack_bytes(&pack_bytes, FORMAT)
            .map_err(|e| RemoteError::Protocol(format!("dumb pack ingest: {e}")))?;
    }
    Ok(())
}

fn ensure_object_closure<T: Transport>(
    transport: &mut T,
    repo: &mut Repository,
    url: &str,
    tip: &Oid,
) -> Result<(), RemoteError> {
    let mut frontier = vec![*tip];
    let mut seen = HashSet::new();
    while let Some(oid) = frontier.pop() {
        if !seen.insert(oid) {
            continue;
        }
        let present = repo
            .store()
            .exists(&oid)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        if !present {
            fetch_loose(transport, repo, url, &oid)?;
        }
        let obj = repo
            .store()
            .get(&oid)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        match obj.kind {
            ObjectKind::Commit => {
                let c = Commit::decode(&obj.payload).map_err(|e| RemoteError::Msg(e.to_string()))?;
                frontier.push(c.tree);
                frontier.extend(c.parents);
            }
            ObjectKind::Tree => {
                let t = Tree::decode(&obj.payload, OidKind::Sha1)
                    .map_err(|e| RemoteError::Msg(e.to_string()))?;
                for e in t.entries {
                    if e.mode == TreeMode::Gitlink {
                        continue;
                    }
                    frontier.push(e.oid);
                }
            }
            ObjectKind::Tag | ObjectKind::Blob => {}
        }
    }
    Ok(())
}

fn fetch_loose<T: Transport>(
    transport: &mut T,
    repo: &mut Repository,
    url: &str,
    oid: &Oid,
) -> Result<(), RemoteError> {
    let rel: PathBuf = loose_rel_path(oid);
    let rel = rel.to_string_lossy().replace('\\', "/");
    let bytes = get_bytes(transport, url, &format!("objects/{rel}"))?;
    let plain = zlib_decompress(&bytes).map_err(|e| RemoteError::Protocol(e.to_string()))?;
    let obj =
        RawObject::decode_uncompressed(&plain).map_err(|e| RemoteError::Protocol(e.to_string()))?;
    repo.store_mut()
        .put(FORMAT, &obj)
        .map_err(|e| RemoteError::Msg(e.to_string()))?;
    Ok(())
}

fn get_bytes<T: Transport>(
    transport: &mut T,
    base: &str,
    rel: &str,
) -> Result<Vec<u8>, RemoteError> {
    let url = join_url(base, rel);
    let resp = transport.exchange(&HttpRequest {
        method: "GET".into(),
        url,
        headers: Vec::new(),
        body: Vec::new(),
    })?;
    require_success(&resp, rel)?;
    Ok(resp.body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dumb_refs_tab_separated() {
        let body = b"\
95dcfa3633004da0049d3d0fa03f80589cbcaf31\trefs/heads/maint\n\
d049f6c27a2244e12041955e262a404c7faba355\trefs/heads/master\n\
2cb58b79488a98d2721cea644875a8dd0026b115\trefs/tags/v1.0\n\
a3c2e2402b99163d1d59756e5f207ae21cccba4c\trefs/tags/v1.0^{}\n";
        let ad = parse_dumb_info_refs(body).unwrap();
        assert_eq!(ad.refs.len(), 3);
        assert_eq!(ad.refs[1].name, "refs/heads/master");
        assert!(looks_like_dumb_info_refs(Some("text/plain"), body));
        assert!(!looks_like_dumb_info_refs(
            Some("application/x-git-upload-pack-advertisement"),
            body
        ));
    }
}
