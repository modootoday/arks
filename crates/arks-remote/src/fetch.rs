//! Fetch: smart HTTP (`info/refs` + `git-upload-pack`) or SSH pack protocol.
//!
//! Protocol v1. The want line lists only capabilities the peer advertised and
//! this client implements (`ofs-delta`, `no-progress`, `agent`, `shallow` when
//! `--depth` / local shallow file requires it, and `filter` when
//! `--filter=blob:none|tree:0` is claimed). Side-band is omitted so upload-pack
//! returns a contiguous PACK (no pkt-line demux on J-CLONE-LOCAL).
//! `thin-pack` is not requested, so the pack is self-contained.
//! Have lines name local tips when present; `done` ends the
//! stateless request.
//!
//! Shallow (`--depth N`): request `shallow`, send `deepen N`, parse server
//! `shallow`/`unshallow`, write `$GIT_DIR/shallow`.
//! Partial-clone (`--filter=blob:none|tree:0`): request `filter`, send
//! `filter <spec>`, mark pack promisor, set `extensions.partialClone`.
//! Cite: <https://git-scm.com/docs/gitprotocol-capabilities>,
//! <https://git-scm.com/docs/gitprotocol-pack>,
//! <https://git-scm.com/docs/partial-clone>,
//! <https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt>.
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-http>,
//! <https://git-scm.com/docs/gitprotocol-pack>

use crate::advertise::{
    Advertisement, cap_advertised, content_type_is, ensure_remote_object_format,
    parse_advertisement, symref_target,
};
use crate::config::{fetch_specs_for, map_refspec};
use crate::error::RemoteError;
use crate::git::{GitDaemonService, GitDaemonSession};
use crate::http::UreqTransport;
use crate::shallow::{apply_shallow_update, read_shallow, write_shallow};
use crate::ssh::{SshService, SshSession};
use crate::transport::{HttpRequest, Transport, header, require_success};
use crate::url::{GitDaemonUrl, SshRemoteUrl, join_url};
use arks_oid::Oid;
use arks_protocol::{Packet, decode_packet, decode_text_line, encode_packet, encode_text_line};
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashSet;

const UPLOAD_AD: &str = "application/x-git-upload-pack-advertisement";
const UPLOAD_REQ: &str = "application/x-git-upload-pack-request";
const UPLOAD_RES: &str = "application/x-git-upload-pack-result";

/// Options for a single fetch/clone pack negotiation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FetchOptions {
    /// Positive commit depth (`--depth N`). `None` = no deepen request.
    pub depth: Option<u32>,
    /// When true, ingest keeps pack objects in RAM and defers pack+idx disk
    /// write (`FilesystemGitStore::take_pending_pack`) so clone can overlap
    /// persist with worktree checkout. Fetch callers that set this MUST flush.
    pub defer_pack_disk: bool,
    /// Claimed partial-clone filter-spec (`blob:none` / `tree:0`). `None` = full pack.
    pub filter: Option<String>,
    /// When set, want these OIDs instead of advertised branch tips (promisor backfill).
    pub want_oids: Option<Vec<Oid>>,
    /// Force `.promisor` marking even without `filter` (demand-fetch packs).
    pub mark_promisor: bool,
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

/// Owned buffer holding a PACK, plus byte offset where the PACK starts (`buf[at..]`).
type PackBufAt = Option<(Vec<u8>, usize)>;

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
        |cap_list, haves, wants, client_shallows, depth, filter| {
            let body =
                encode_fetch_request(wants, cap_list, haves, client_shallows, depth, filter)?;
            session.write_all(&body)?;
            session.close_stdin();
            session.read_to_end()
        },
    )?;
    // Pack already applied; ssh/upload-pack exit status is advisory.
    let _ = session.finish();
    Ok(outcome)
}

/// Fetch over `git://` speaking upload-pack pkt-line (git-daemon transport).
pub fn fetch_git(
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
) -> Result<FetchOutcome, RemoteError> {
    fetch_git_with(repo, url, remote_name, &FetchOptions::default())
}

pub fn fetch_git_with(
    repo: &mut Repository,
    url: &str,
    remote_name: &str,
    opts: &FetchOptions,
) -> Result<FetchOutcome, RemoteError> {
    let git_url = GitDaemonUrl::parse(url)?;
    let mut session = GitDaemonSession::connect(&git_url, GitDaemonService::UploadPack)?;
    let ad = session.read_advertisement()?;
    apply_fetch_from_ad(
        repo,
        remote_name,
        &ad,
        opts,
        |cap_list, haves, wants, client_shallows, depth, filter| {
            let body =
                encode_fetch_request(wants, cap_list, haves, client_shallows, depth, filter)?;
            session.write_all(&body)?;
            session.read_to_end()
        },
    )
}

/// Dispatch HTTP / SSH / `git://` from the remote URL scheme.
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
    } else if GitDaemonUrl::parse(url).is_ok() {
        fetch_git_with(repo, url, remote_name, opts)
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
        |cap_list, haves, wants, client_shallows, depth, filter| {
            let body =
                encode_fetch_request(wants, cap_list, haves, client_shallows, depth, filter)?;
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
    F: FnMut(
        &str,
        &[Oid],
        &[Oid],
        &[Oid],
        Option<u32>,
        Option<&str>,
    ) -> Result<Vec<u8>, RemoteError>,
{
    if let Some(depth) = opts.depth {
        if depth == 0 {
            return Err(RemoteError::Msg(
                "fetch --depth 0 is not a deepen request; omit --depth".into(),
            ));
        }
    }
    if let Some(filter) = opts.filter.as_deref() {
        validate_claimed_filter(filter)?;
    }
    ensure_remote_object_format(repo, ad)?;

    let (head_branch, head_oid) = head_from_ad(ad);
    let specs = fetch_specs_for(&repo.git_dir().join("config"), remote_name);
    let wants: Vec<Oid> = if let Some(extra) = &opts.want_oids {
        extra.clone()
    } else {
        let mut tips = Vec::new();
        for r in &ad.refs {
            if r.name.starts_with("refs/heads/") && !tips.contains(&r.oid) {
                tips.push(r.oid);
            }
        }
        tips
    };
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
    let need_filter_cap = opts.filter.is_some();
    if need_filter_cap && !cap_advertised(&ad.capabilities, "filter") {
        return Err(RemoteError::NotImplemented(
            "peer did not advertise filter; --filter partial-clone not claimed against this peer"
                .into(),
        ));
    }

    let (cap_list, sideband) =
        negotiate_fetch_caps(&ad.capabilities, need_shallow_cap, need_filter_cap);
    let haves = if opts.want_oids.is_some() {
        Vec::new()
    } else {
        local_haves(repo)
    };
    let response_body = request_pack(
        &cap_list,
        &haves,
        &wants,
        &client_shallows,
        opts.depth,
        opts.filter.as_deref(),
    )?;

    let (pack_opt, update) = extract_pack_and_shallow(response_body, sideband)?;
    let mark_promisor = opts.mark_promisor || opts.filter.is_some();
    let mut ingested_pack = false;
    match pack_opt {
        Some((buf, pack_at)) => {
            // Keep the received pack as pack+idx (Git layout) instead of
            // exploding every object to loose — fetch/clone hot path for M14.
            // Clone defers disk via owned move; pack_at skips NAK prefix copy.
            if mark_promisor {
                repo.store_mut().mark_next_pack_promisor(true);
            }
            if opts.defer_pack_disk {
                let format = repo.object_format();
                repo.store_mut()
                    .ingest_pack_bytes_deferred_at(buf, pack_at, format)
                    .map_err(|e| RemoteError::Protocol(format!("pack: {e}")))?;
            } else {
                let format = repo.object_format();
                repo.store_mut()
                    .ingest_pack_bytes(&buf[pack_at..], format)
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

    if opts.filter.is_some() {
        crate::config::enable_partial_clone(repo.git_dir(), remote_name)?;
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

    // Demand-fetch (want_oids) only fills missing objects; do not rewrite tracking refs.
    if opts.want_oids.is_some() {
        return Ok(FetchOutcome {
            updated: Vec::new(),
            head_branch,
            head_oid,
        });
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

/// Claimed filter-spec gate (S1/O2 + FS4 + G6 + sparse:oid).
///
/// Claimed: `blob:none`, `tree:0`, `tree:<depth>` (non-neg int), `blob:limit=<n>`
/// (`n` bytes, optional `k`/`m`/`g` suffix), `object:type=(blob|tree|commit|tag)`,
/// `sparse:oid=<hex>` (40/64-char OID of a sparse-checkout blob), and `combine:`
/// of already-Claimed children joined by `+`. Multi-`--filter` remains refuse.
/// Cite: git-rev-list `--filter`.
pub fn validate_claimed_filter(spec: &str) -> Result<(), RemoteError> {
    validate_claimed_filter_inner(spec, 0)
}

fn validate_claimed_filter_inner(spec: &str, depth: u8) -> Result<(), RemoteError> {
    if depth > 8 {
        return Err(RemoteError::NotImplemented(
            "clone/fetch --filter=combine: nesting too deep".into(),
        ));
    }
    if spec == "blob:none" || spec == "tree:0" {
        return Ok(());
    }
    if let Some(rest) = spec.strip_prefix("tree:") {
        if rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty() {
            return Ok(());
        }
    }
    if let Some(rest) = spec.strip_prefix("blob:limit=") {
        if parse_blob_limit(rest).is_some() {
            return Ok(());
        }
    }
    if let Some("blob" | "tree" | "commit" | "tag") = spec.strip_prefix("object:type=") {
        return Ok(());
    }
    if let Some(rest) = spec.strip_prefix("sparse:oid=") {
        // Hex OID of the sparse-checkout blob (sha1 40 / sha256 64). Blob-ish
        // names beyond raw hex remain refuse (peer resolves object id).
        if (rest.len() == 40 || rest.len() == 64)
            && rest.chars().all(|c| c.is_ascii_hexdigit())
            && Oid::from_hex(rest).is_ok()
        {
            return Ok(());
        }
        return Err(RemoteError::NotImplemented(
            "clone/fetch --filter=sparse:oid=<hex> requires a 40- or 64-char object id".into(),
        ));
    }
    if let Some(rest) = spec.strip_prefix("combine:") {
        if rest.is_empty() {
            return Err(RemoteError::NotImplemented(
                "clone/fetch --filter=combine: requires claimed children".into(),
            ));
        }
        for part in rest.split('+') {
            validate_claimed_filter_inner(part, depth + 1)?;
        }
        return Ok(());
    }
    Err(RemoteError::NotImplemented(format!(
        "clone/fetch --filter={spec} not claimed; claimed: blob:none, tree:0, tree:<depth>, blob:limit=<n>, object:type=<blob|tree|commit|tag>, sparse:oid=<hex>, combine:<claimed>+…"
    )))
}

fn parse_blob_limit(raw: &str) -> Option<u64> {
    if raw.is_empty() {
        return None;
    }
    let (num, mul) = match raw.as_bytes().last().map(|b| b.to_ascii_lowercase()) {
        Some(b @ (b'k' | b'm' | b'g')) => {
            let n = raw[..raw.len() - 1].parse::<u64>().ok()?;
            let m = match b {
                b'k' => 1024u64,
                b'm' => 1024 * 1024,
                b'g' => 1024 * 1024 * 1024,
                _ => 1,
            };
            (n, m)
        }
        _ => (raw.parse::<u64>().ok()?, 1),
    };
    num.checked_mul(mul)
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

fn negotiate_fetch_caps(
    advertised: &[String],
    want_shallow: bool,
    want_filter: bool,
) -> (String, bool) {
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
    if want_filter && cap_advertised(advertised, "filter") {
        caps.push("filter");
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
    filter: Option<&str>,
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
    if let Some(spec) = filter {
        body.extend(
            encode_text_line(&format!("filter {spec}"))
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
) -> Result<(PackBufAt, ShallowUpdate), RemoteError> {
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
        let body =
            encode_fetch_request(&[oid], "ofs-delta shallow", &[], &[], Some(1), None).unwrap();
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
