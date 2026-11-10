//! Dumb HTTP client fetch/clone + WebDAV push path (additive beside smart HTTP).
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-http#_dumb_clients>
//! Fetch flow: discover refs via dumb `info/refs` body (no smart service pkt-line),
//! read `objects/info/packs`, download listed packs, fetch missing loose
//! objects by OID path, update remote-tracking refs.
//!
//! Push flow (R2): when receive-pack discovery is dumb, speak WebDAV write
//! ([git-http-push](https://git-scm.com/docs/git-http-push),
//! [RFC 4918](https://datatracker.ietf.org/doc/html/rfc4918),
//! [[adr:root/20261110190000]]).

use crate::advertise::{AdvertisedRef, Advertisement};
use crate::config::{fetch_specs_for, map_refspec};
use crate::error::RemoteError;
use crate::fetch::{FetchOptions, FetchOutcome};
use crate::push::{PushUpdate, objects_to_send, zero_oid};
use crate::transport::{HttpRequest, Transport, header, require_success};
use crate::url::join_url;
use arks_object::{
    Commit, ObjectKind, RawObject, Tree, TreeMode, loose_rel_path, zlib_compress, zlib_decompress,
};
use arks_oid::{Oid, OidKind};
use arks_repo::{Repository, ancestor_set};
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
    body.contains(&b'\t')
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
    if opts.filter.is_some() {
        return Err(RemoteError::NotImplemented(
            "clone/fetch --filter is not claimed for dumb HTTP (pack-protocol filter only)".into(),
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
                let c =
                    Commit::decode(&obj.payload).map_err(|e| RemoteError::Msg(e.to_string()))?;
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

/// Dumb HTTP push via WebDAV (git-http-push subset).
///
/// `info_body` is the dumb `info/refs` entity already returned for receive-pack
/// discovery (smart→dumb fallback; no second GET required).
pub(crate) fn push_dumb_with<T: Transport>(
    transport: &mut T,
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    info_body: &[u8],
) -> Result<(), RemoteError> {
    let ad = parse_dumb_info_refs(info_body)?;
    // Optional PROPFIND — arks server always supports exclusive write locks.
    let _ = transport.exchange(&HttpRequest {
        method: "PROPFIND".into(),
        url: join_url(url, ""),
        headers: vec![("Depth".into(), "0".into())],
        body: Vec::new(),
    });

    let mut tips = Vec::new();
    let mut bases = Vec::new();
    let mut planned: Vec<(Oid, Oid, String)> = Vec::new();
    for update in updates {
        if update.new_oid.as_bytes().iter().all(|b| *b == 0) {
            return Err(RemoteError::NotImplemented(
                "dumb HTTP push delete (-d/-D) is not claimed".into(),
            ));
        }
        let old = ad
            .refs
            .iter()
            .find(|r| r.name == update.refname)
            .map(|r| r.oid)
            .unwrap_or_else(zero_oid);
        if old == update.new_oid {
            continue;
        }
        if !old.as_bytes().iter().all(|b| *b == 0) {
            let anc =
                ancestor_set(repo, update.new_oid).map_err(|e| RemoteError::Msg(e.to_string()))?;
            if !anc.contains(&old) {
                return Err(RemoteError::Rejected(format!(
                    "{} non-fast-forward",
                    update.refname
                )));
            }
            if repo.store().exists(&old).unwrap_or(false) {
                bases.push(old);
            }
        }
        tips.push(update.new_oid);
        planned.push((old, update.new_oid, update.refname.clone()));
    }
    if planned.is_empty() {
        return Ok(());
    }

    let needed = objects_to_send(repo, &tips, &bases)?;
    for obj in &needed {
        let oid = obj
            .oid(FORMAT)
            .map_err(|e| RemoteError::Protocol(e.to_string()))?;
        upload_loose(transport, url, &oid, obj)?;
    }

    for (_old, new, refname) in &planned {
        let token = dav_lock(transport, url, refname)?;
        let ref_url = join_url(url, refname);
        let body = format!("{}\n", new.to_hex()).into_bytes();
        let put = transport.exchange(&HttpRequest {
            method: "PUT".into(),
            url: ref_url,
            headers: vec![
                ("Content-Type".into(), "text/plain".into()),
                ("If".into(), format!("(<{token}>)")),
                ("Lock-Token".into(), format!("<{token}>")),
            ],
            body,
        })?;
        if !(200..300).contains(&put.status) {
            let _ = dav_unlock(transport, url, refname, &token);
            return Err(RemoteError::Http(format!(
                "PUT {refname} -> HTTP {}",
                put.status
            )));
        }
        dav_unlock(transport, url, refname, &token)?;
    }
    Ok(())
}

fn upload_loose<T: Transport>(
    transport: &mut T,
    base: &str,
    oid: &Oid,
    obj: &RawObject,
) -> Result<(), RemoteError> {
    let rel = loose_rel_path(oid);
    let rel = rel.to_string_lossy().replace('\\', "/");
    let obj_rel = format!("objects/{rel}");
    // Skip upload when the peer already has the object.
    // Cleartext transport maps non-2xx to Err — treat HTTP 404 as missing.
    match transport.exchange(&HttpRequest {
        method: "GET".into(),
        url: join_url(base, &obj_rel),
        headers: Vec::new(),
        body: Vec::new(),
    }) {
        Ok(resp) if resp.status == 200 => return Ok(()),
        Ok(_) => {}
        Err(RemoteError::Http(msg)) if msg.starts_with("HTTP 404") => {}
        Err(e) => return Err(e),
    }
    let fanout = &oid.to_hex()[..2];
    match transport.exchange(&HttpRequest {
        method: "MKCOL".into(),
        url: join_url(base, &format!("objects/{fanout}")),
        headers: Vec::new(),
        body: Vec::new(),
    }) {
        // 201 created; 405 exists — both OK for git-http-push-class clients.
        Ok(_) => {}
        Err(RemoteError::Http(msg))
            if msg.starts_with("HTTP 405") || msg.starts_with("HTTP 409") => {}
        Err(e) => return Err(e),
    }
    let compressed = zlib_compress(&obj.encode_uncompressed())
        .map_err(|e| RemoteError::Protocol(e.to_string()))?;
    // PUT to a temp name then MOVE (git-http-push race-safety pattern).
    let tmp_rel = format!("{obj_rel}_arks_tmp");
    let put = transport.exchange(&HttpRequest {
        method: "PUT".into(),
        url: join_url(base, &tmp_rel),
        headers: vec![(
            "Content-Type".into(),
            "application/x-git-loose-object".into(),
        )],
        body: compressed,
    })?;
    if !(200..300).contains(&put.status) {
        return Err(RemoteError::Http(format!(
            "PUT {tmp_rel} -> HTTP {}",
            put.status
        )));
    }
    let dest = join_url(base, &obj_rel);
    let mv = transport.exchange(&HttpRequest {
        method: "MOVE".into(),
        url: join_url(base, &tmp_rel),
        headers: vec![("Destination".into(), dest)],
        body: Vec::new(),
    })?;
    if !(200..300).contains(&mv.status) {
        return Err(RemoteError::Http(format!(
            "MOVE {tmp_rel} -> HTTP {}",
            mv.status
        )));
    }
    Ok(())
}

fn dav_lock<T: Transport>(transport: &mut T, base: &str, rel: &str) -> Result<String, RemoteError> {
    let resp = transport.exchange(&HttpRequest {
        method: "LOCK".into(),
        url: join_url(base, rel),
        headers: vec![
            ("Content-Type".into(), "application/xml".into()),
            ("Depth".into(), "0".into()),
            ("Timeout".into(), "Second-600".into()),
        ],
        body: br#"<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
  <D:lockscope><D:exclusive/></D:lockscope>
  <D:locktype><D:write/></D:locktype>
</D:lockinfo>
"#
        .to_vec(),
    })?;
    if !(200..300).contains(&resp.status) {
        return Err(RemoteError::Http(format!(
            "LOCK {rel} -> HTTP {}",
            resp.status
        )));
    }
    if let Some(tok) = header(&resp.headers, "lock-token") {
        return Ok(tok
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_string());
    }
    // Parse locktoken href from body as fallback.
    let text = String::from_utf8_lossy(&resp.body);
    if let Some(idx) = text.find("opaquelocktoken:") {
        let rest = &text[idx..];
        let end = rest
            .find(|c: char| c == '<' || c == '>' || c.is_whitespace())
            .unwrap_or(rest.len());
        return Ok(rest[..end].to_string());
    }
    Err(RemoteError::Protocol(format!(
        "LOCK {rel} response missing lock token"
    )))
}

fn dav_unlock<T: Transport>(
    transport: &mut T,
    base: &str,
    rel: &str,
    token: &str,
) -> Result<(), RemoteError> {
    let resp = transport.exchange(&HttpRequest {
        method: "UNLOCK".into(),
        url: join_url(base, rel),
        headers: vec![("Lock-Token".into(), format!("<{token}>"))],
        body: Vec::new(),
    })?;
    if !(200..300).contains(&resp.status) {
        return Err(RemoteError::Http(format!(
            "UNLOCK {rel} -> HTTP {}",
            resp.status
        )));
    }
    Ok(())
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
