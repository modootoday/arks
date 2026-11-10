//! Fetch: smart HTTP (`info/refs` + `git-upload-pack`) or SSH pack protocol.
//!
//! Protocol v1. The want line lists only capabilities the peer advertised and
//! this client implements (`ofs-delta`, `no-progress`, `agent`, and `shallow`
//! when `--depth` / local shallow file requires it). Side-band is omitted so
//! upload-pack returns a contiguous PACK (no pkt-line demux on J-CLONE-LOCAL).
//! `thin-pack` is not requested, so the pack is self-contained.
//! Have lines name local tips when present; `done` ends the
//! stateless request.
//!
//! Shallow (`--depth N`): request `shallow`, send `deepen N`, parse server
//! `shallow`/`unshallow`, write `$GIT_DIR/shallow`.
//! Cite: <https://git-scm.com/docs/gitprotocol-capabilities>,
//! <https://git-scm.com/docs/gitprotocol-pack>,
//! <https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt>.
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-http>,
//! <https://git-scm.com/docs/gitprotocol-pack>

use crate::advertise::{
    cap_advertised, content_type_is, parse_advertisement, symref_target, Advertisement,
};
use crate::config::{fetch_specs_for, map_refspec};
use crate::error::RemoteError;
use crate::http::UreqTransport;
use crate::shallow::{apply_shallow_update, read_shallow, write_shallow};
use crate::ssh::{SshService, SshSession};
use crate::transport::{header, require_success, HttpRequest, Transport};
use crate::url::{join_url, SshRemoteUrl};
use arks_oid::Oid;
use arks_protocol::{decode_packet, decode_text_line, encode_packet, encode_text_line, Packet};
use arks_repo::Repository;
use arks_store::{ObjectFormat, ObjectStore};
use std::collections::HashSet;

const UPLOAD_AD: &str = "application/x-git-upload-pack-advertisement";
const UPLOAD_REQ: &str = "application/x-git-upload-pack-request";
const UPLOAD_RES: &str = "application/x-git-upload-pack-result";
const FORMAT: ObjectFormat = ObjectFormat::Sha1;

/// Options for a single fetch/clone pack negotiation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FetchOptions {
    /// Positive commit depth (`--depth N`). `None` = no deepen request.
    pub depth: Option<u32>,
    /// When true, ingest keeps pack objects in RAM and defers pack+idx disk
    /// write (`FilesystemGitStore::take_pending_pack`) so clone can overlap
    /// persist with worktree checkout. Fetch callers that set this MUST flush.
    pub defer_pack_disk: bool,
}

/// Refs updated by one fetch, plus the peer HEAD branch when advertised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchOutcome {
    pub updated: Vec<(String, Oid)>,
    pub head_branch: Option<String>,
    pub head_oid: Option<Oid>,
}

#[derive(Debug, Default, Clone)]
struct ShallowUpdate {
    shallow: Vec<Oid>,
    unshallow: Vec<Oid>,
}

pub fn fetch_http(
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
) -> Result<FetchOutcome, RemoteError> {
    fetch_http_with(repo, url, remote_name, &FetchOptions::default())
}

pub fn fetch_http_with(
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
    opts: &FetchOptions,
) -> Result<FetchOutcome, RemoteError> {
    let mut transport = UreqTransport::new();
    fetch_with(&mut transport, repo, url, remote_name, opts)
}

/// Fetch over OpenSSH (`ssh://` or SCP-like URL) speaking upload-pack pkt-line.
pub fn fetch_ssh(
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
) -> Result<FetchOutcome, RemoteError> {
    fetch_ssh_with(repo, url, remote_name, &FetchOptions::default())
}

pub fn fetch_ssh_with(
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
    opts: &FetchOptions,
) -> Result<FetchOutcome, RemoteError> {
    let ssh_url = SshRemoteUrl::parse(url)?;
    let mut session = SshSession::connect(&ssh_url, SshService::UploadPack)?;
    let ad = session.read_advertisement()?;
    let outcome = apply_fetch_from_ad(
        repo,
        remote_name,
        &ad,
        opts,
        |cap_list, haves, wants, client_shallows, depth| {
            let body = encode_fetch_request(wants, cap_list, haves, client_shallows, depth)?;
            session.write_all(&body)?;
            session.close_stdin();
            session.read_to_end()
        },
    )?;
    // Pack already applied; ssh/upload-pack exit status is advisory.
    let _ = session.finish();
    Ok(outcome)
}

/// Dispatch HTTP vs SSH from the remote URL scheme.
pub fn fetch_remote(
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
) -> Result<FetchOutcome, RemoteError> {
    fetch_remote_with(repo, url, remote_name, &FetchOptions::default())
}

pub fn fetch_remote_with(
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
    opts: &FetchOptions,
) -> Result<FetchOutcome, RemoteError> {
    if SshRemoteUrl::parse(url).is_ok() {
        fetch_ssh_with(repo, url, remote_name, opts)
    } else {
        fetch_http_with(repo, url, remote_name, opts)
    }
}

pub(crate) fn fetch_with<T: Transport>(
    transport: &mut T,
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
    opts: &FetchOptions,
) -> Result<FetchOutcome, RemoteError> {
    let info_url = join_url(url, "info/refs?service=git-upload-pack");
    let info = transport.exchange(&HttpRequest {
        method: "GET".into(),
        url: info_url,
        headers: vec![("Accept".into(), UPLOAD_AD.into())],
        body: Vec::new(),
    })?;
    require_success(&info, "info/refs")?;
    let ct = header(&info.headers, "content-type");
    // Smart clients MUST fall back to dumb when the peer returns a non-smart
    // Content-Type / dumb body (gitprotocol-http Smart Clients).
    if crate::dumb::looks_like_dumb_info_refs(ct, &info.body) {
        let ad = crate::dumb::parse_dumb_info_refs(&info.body)?;
        return crate::dumb::fetch_dumb_with(transport, repo, url, remote_name, opts, &ad);
    }
    if !content_type_is(ct, "git-upload-pack-advertisement") {
        return Err(RemoteError::Protocol(
            "info/refs was not a git-upload-pack advertisement".into(),
        ));
    }
    let ad = parse_advertisement(&info.body, "git-upload-pack")?;
    apply_fetch_from_ad(
        repo,
        remote_name,
        &ad,
        opts,
        |cap_list, haves, wants, client_shallows, depth| {
            let body = encode_fetch_request(wants, cap_list, haves, client_shallows, depth)?;
            let post_url = join_url(url, "git-upload-pack");
            let posted = transport.exchange(&HttpRequest {
                method: "POST".into(),
                url: post_url,
                headers: vec![
                    ("Content-Type".into(), UPLOAD_REQ.into()),
                    ("Accept".into(), UPLOAD_RES.into()),
                ],
                body,
            })?;
            require_success(&posted, "git-upload-pack")?;
            if !content_type_is(
                header(&posted.headers, "content-type"),
                "git-upload-pack-result",
            ) {
                return Err(RemoteError::Protocol(
                    "git-upload-pack result had an unexpected content type".into(),
                ));
            }
            Ok(posted.body)
        },
    )
}

fn apply_fetch_from_ad<F>(
    repo: &mut Repository,
    remote_name: &str,
    ad: &Advertisement,
    opts: &FetchOptions,
    mut request_pack: F,
) -> Result<FetchOutcome, RemoteError>
where
    F: FnMut(&str, &[Oid], &[Oid], &[Oid], Option<u32>) -> Result<Vec<u8>, RemoteError>,
{
    if let Some(depth) = opts.depth {
        if depth == 0 {
            return Err(RemoteError::Msg(
                "fetch --depth 0 is not a deepen request; omit --depth".into(),
            ));
        }
    }

    let (head_branch, head_oid) = head_from_ad(ad);
    let specs = fetch_specs_for(&repo.git_dir().join("config"), remote_name);
    let mut wants: Vec<Oid> = Vec::new();
    for r in &ad.refs {
        if r.name.starts_with("refs/heads/") && !wants.contains(&r.oid) {
            wants.push(r.oid);
        }
    }
    if wants.is_empty() {
        return Ok(FetchOutcome {
            updated: Vec::new(),
            head_branch,
            head_oid,
        });
    }

    let client_shallows = read_shallow(repo.git_dir())?;
    let need_shallow_cap = opts.depth.is_some() || !client_shallows.is_empty();
    if need_shallow_cap && !cap_advertised(&ad.capabilities, "shallow") {
        return Err(RemoteError::NotImplemented(
            "peer did not advertise shallow; --depth / shallow fetch not claimed against this peer"
                .into(),
        ));
    }

    let (cap_list, sideband) = negotiate_fetch_caps(&ad.capabilities, need_shallow_cap);
    let haves = local_haves(repo);
    let response_body = request_pack(&cap_list, &haves, &wants, &client_shallows, opts.depth)?;

    let (pack_opt, update) = extract_pack_and_shallow(response_body, sideband)?;
    let mut ingested_pack = false;
    match pack_opt {
        Some((buf, pack_at)) => {
            // Keep the received pack as pack+idx (Git layout) instead of
            // exploding every object to loose — fetch/clone hot path for M14.
            // Clone defers disk via owned move; pack_at skips NAK prefix copy.
            if opts.defer_pack_disk {
                repo.store_mut()
                    .ingest_pack_bytes_deferred_at(buf, pack_at, FORMAT)
                    .map_err(|e| RemoteError::Protocol(format!("pack: {e}")))?;
            } else {
                repo.store_mut()
                    .ingest_pack_bytes(&buf[pack_at..], FORMAT)
                    .map_err(|e| RemoteError::Protocol(format!("pack: {e}")))?;
            }
            ingested_pack = true;
        }
        None => {
            for oid in &wants {
                let present = repo
                    .store()
                    .exists(oid)
                    .map_err(|e| RemoteError::Msg(e.to_string()))?;
                if !present {
                    return Err(RemoteError::Protocol(
                        "peer sent no pack and wanted objects are not local".into(),
                    ));
                }
            }
        }
    }

    if need_shallow_cap || !update.shallow.is_empty() || !update.unshallow.is_empty() {
        let next = apply_shallow_update(&client_shallows, &update.shallow, &update.unshallow);
        write_shallow(repo.git_dir(), &next)?;
    }

    // Tips must resolve before refs move. After a successful pack ingest the
    // decoder already reconstructed tip OIDs into the pack cache — skip the
    // redundant exists() walk on that hot path.
    if !ingested_pack {
        for oid in &wants {
            let present = repo
                .store()
                .exists(oid)
                .map_err(|e| RemoteError::Msg(e.to_string()))?;
            if !present {
                return Err(RemoteError::Protocol(format!(
                    "missing tip object {}",
                    oid.to_hex()
                )));
            }
        }
    }

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

fn head_from_ad(ad: &Advertisement) -> (Option<String>, Option<Oid>) {
    if let Some(target) = symref_target(&ad.capabilities, "HEAD") {
        let oid = ad.refs.iter().find(|r| r.name == target).map(|r| r.oid);
        return (Some(target), oid);
    }
    if let Some(r) = ad
        .refs
        .iter()
        .find(|r| r.name == "refs/heads/main")
        .or_else(|| ad.refs.iter().find(|r| r.name.starts_with("refs/heads/")))
    {
        return (Some(r.name.clone()), Some(r.oid));
    }
    (None, None)
}

fn negotiate_fetch_caps(advertised: &[String], want_shallow: bool) -> (String, bool) {
    let mut caps = Vec::new();
    if cap_advertised(advertised, "ofs-delta") {
        caps.push("ofs-delta");
    }
    // Skip side-band / side-band-64k: progress is unused (no-progress) and a raw
    // PACK after NAK avoids pkt-line demux copies on J-CLONE-LOCAL. Stock
    // http-backend and MemoryPeer/arks-server emit contiguous pack when omitted.
    let sideband = false;
    if cap_advertised(advertised, "no-progress") {
        caps.push("no-progress");
    }
    if want_shallow && cap_advertised(advertised, "shallow") {
        caps.push("shallow");
    }
    if cap_advertised(advertised, "agent") {
        caps.push("agent=arks/0.0.0");
    }
    (caps.join(" "), sideband)
}

fn local_haves(repo: &Repository) -> Vec<Oid> {
    let Ok(refs) = repo.refs().list_refs("") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (_name, oid) in refs {
        if !seen.insert(oid) {
            continue;
        }
        if repo.store().exists(&oid).unwrap_or(false) {
            out.push(oid);
        }
        if out.len() == 32 {
            break;
        }
    }
    out
}

fn encode_fetch_request(
    wants: &[Oid],
    caps: &str,
    haves: &[Oid],
    client_shallows: &[Oid],
    depth: Option<u32>,
) -> Result<Vec<u8>, RemoteError> {
    let mut body = Vec::new();
    for (i, oid) in wants.iter().enumerate() {
        let line = if i == 0 && !caps.is_empty() {
            format!("want {} {caps}", oid.to_hex())
        } else {
            format!("want {}", oid.to_hex())
        };
        body.extend(encode_text_line(&line).map_err(|e| RemoteError::Protocol(e.to_string()))?);
    }
    for oid in client_shallows {
        body.extend(
            encode_text_line(&format!("shallow {}", oid.to_hex()))
                .map_err(|e| RemoteError::Protocol(e.to_string()))?,
        );
    }
    if let Some(depth) = depth {
        body.extend(
            encode_text_line(&format!("deepen {depth}"))
                .map_err(|e| RemoteError::Protocol(e.to_string()))?,
        );
    }
    body.extend(encode_packet(&Packet::Flush).map_err(|e| RemoteError::Protocol(e.to_string()))?);
    for oid in haves {
        if wants.contains(oid) {
            body.extend(
                encode_text_line(&format!("have {}", oid.to_hex()))
                    .map_err(|e| RemoteError::Protocol(e.to_string()))?,
            );
        }
    }
    body.extend(encode_text_line("done").map_err(|e| RemoteError::Protocol(e.to_string()))?);
    Ok(body)
}

/// Demux a v1 upload-pack result, collecting optional shallow-update lines.
///
/// With side-band, band 1 is pack data, band 2 is progress, band 3 is fatal.
/// Without side-band, pkt-line `NAK`/`ACK` is followed by a raw pack.
/// `None` pack means the peer finished negotiation without a pack (already up to date).
#[allow(dead_code)]
pub(crate) fn extract_pack(body: Vec<u8>, sideband: bool) -> Result<Option<Vec<u8>>, RemoteError> {
    let (pack, _) = extract_pack_and_shallow(body, sideband)?;
    Ok(pack.map(|(buf, at)| if at == 0 { buf } else { buf[at..].to_vec() }))
}

/// Returns `(buf, pack_at)` where `buf[pack_at..]` is the raw PACK (or a
/// side-band-demuxed pack with `pack_at == 0`).
fn extract_pack_and_shallow(
    body: Vec<u8>,
    sideband: bool,
) -> Result<(Option<(Vec<u8>, usize)>, ShallowUpdate), RemoteError> {
    let mut offset = 0;
    // Side-band demux still accumulates; raw PACK keeps offset into `body`.
    let mut pack = Vec::new();
    let mut saw_status = false;
    let mut update = ShallowUpdate::default();
    let mut raw_pack_at: Option<usize> = None;
    while offset < body.len() {
        if !sideband && body[offset..].starts_with(b"PACK") {
            raw_pack_at = Some(offset);
            break;
        }
        if body.len() - offset < 4 {
            break;
        }
        let (packet, n) = match decode_packet(&body[offset..]) {
            Ok(v) => v,
            Err(_) if !pack.is_empty() => break,
            Err(e) => return Err(RemoteError::Protocol(e.to_string())),
        };
        offset += n;
        match packet {
            Packet::Flush => {
                if saw_status || !pack.is_empty() {
                    break;
                }
            }
            Packet::Data(payload) => {
                if payload.is_empty() {
                    continue;
                }
                if sideband && matches!(payload[0], 1..=3) {
                    match payload[0] {
                        1 => pack.extend_from_slice(&payload[1..]),
                        2 => {}
                        3 => {
                            let msg = String::from_utf8_lossy(&payload[1..]);
                            return Err(RemoteError::Protocol(format!("remote: {msg}")));
                        }
                        _ => {}
                    }
                    saw_status = true;
                } else {
                    let text = decode_text_line(&payload).unwrap_or_default();
                    if text.starts_with("ERR ") || text.starts_with("error") {
                        return Err(RemoteError::Protocol(text));
                    }
                    if let Some(hex) = text.strip_prefix("shallow ") {
                        let oid = Oid::from_hex(hex.trim())
                            .map_err(|e| RemoteError::Protocol(format!("shallow oid: {e}")))?;
                        update.shallow.push(oid);
                        continue;
                    }
                    if let Some(hex) = text.strip_prefix("unshallow ") {
                        let oid = Oid::from_hex(hex.trim())
                            .map_err(|e| RemoteError::Protocol(format!("unshallow oid: {e}")))?;
                        update.unshallow.push(oid);
                        continue;
                    }
                    if text.starts_with("NAK") || text.starts_with("ACK") {
                        saw_status = true;
                    }
                }
            }
            Packet::Delim | Packet::ResponseEnd => {
                return Err(RemoteError::NotImplemented(
                    "protocol v2 pack response is not claimed".into(),
                ));
            }
        }
    }
    let extracted = if let Some(at) = raw_pack_at {
        if at >= body.len() || !body[at..].starts_with(b"PACK") {
            return Err(RemoteError::Protocol(
                "demultiplexed payload is not a pack".into(),
            ));
        }
        Some((body, at))
    } else if pack.is_empty() {
        if saw_status {
            None
        } else {
            return Err(RemoteError::Protocol(
                "upload-pack response did not contain a pack".into(),
            ));
        }
    } else if !pack.starts_with(b"PACK") {
        return Err(RemoteError::Protocol(
            "demultiplexed payload is not a pack".into(),
        ));
    } else {
        Some((pack, 0))
    };
    Ok((extracted, update))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn sideband_pack(pack: &[u8]) -> Result<Vec<u8>, RemoteError> {
    let mut out = encode_text_line("NAK").map_err(|e| RemoteError::Protocol(e.to_string()))?;
    // gitprotocol-common max data is 65516; one byte is the sideband designator.
    const CHUNK: usize = 65_515;
    for chunk in pack.chunks(CHUNK) {
        let mut payload = Vec::with_capacity(1 + chunk.len());
        payload.push(1);
        payload.extend_from_slice(chunk);
        out.extend(
            encode_packet(&Packet::Data(payload))
                .map_err(|e| RemoteError::Protocol(e.to_string()))?,
        );
    }
    out.extend(encode_packet(&Packet::Flush).map_err(|e| RemoteError::Protocol(e.to_string()))?);
    Ok(out)
}

#[cfg(test)]
mod shallow_wire_tests {
    use super::*;
    use arks_protocol::encode_text_line;

    #[test]
    fn encode_includes_deepen_and_shallow_cap() {
        let oid = Oid::from_hex("0123456789abcdef0123456789abcdef01234567").unwrap();
        let body = encode_fetch_request(&[oid], "ofs-delta shallow", &[], &[], Some(1)).unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("want 0123456789abcdef0123456789abcdef01234567 ofs-delta shallow"));
        assert!(text.contains("deepen 1"));
    }

    #[test]
    fn extract_parses_shallow_before_nak() {
        let tip = Oid::from_hex("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let mut body = encode_text_line(&format!("shallow {}", tip.to_hex())).unwrap();
        body.extend(encode_packet(&Packet::Flush).unwrap());
        body.extend(encode_text_line("NAK").unwrap());
        let mut pack = b"PACK".to_vec();
        pack.extend_from_slice(&[0u8; 8]);
        body.extend_from_slice(&pack);
        let (got, update) = extract_pack_and_shallow(body, false).unwrap();
        assert_eq!(update.shallow, vec![tip]);
        let (buf, at) = got.unwrap();
        assert!(buf[at..].starts_with(b"PACK"));
    }
}
