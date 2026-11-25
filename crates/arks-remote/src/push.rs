//! Push: smart HTTP receive-pack, SSH / `git://` pack protocol, or dumb HTTP
//! WebDAV fallback when receive-pack advertisement is dumb.
//!
//! Smart path: update commands + undeltified pack, then `report-status` (or
//! `report-status-v2` when that is the only form advertised). Deletes require
//! peer `delete-refs`. Force-with-lease + push-options claimed (O4).
//!
//! Dumb path: WebDAV LOCK/MKCOL/PUT/(MOVE)/DELETE/UNLOCK per git-http-push +
//! RFC 4918 ([[adr:root/20261110190000]], [[adr:root/20261110112600]]).
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-pack> (push-cert / report-status),
//! <https://git-scm.com/docs/gitprotocol-capabilities> (`delete-refs`),
//! <https://git-scm.com/docs/gitprotocol-http>,
//! <https://git-scm.com/docs/git-http-push>

use crate::advertise::{
    Advertisement, cap_advertised, content_type_is, ensure_remote_object_format,
    parse_advertisement,
};
use crate::error::RemoteError;
use crate::git::{GitDaemonService, GitDaemonSession};
use crate::http::UreqTransport;
use crate::ssh::{SshService, SshSession};
use crate::transport::{HttpRequest, Transport, header, require_success};
use crate::url::{GitDaemonUrl, SshRemoteUrl, join_url};
use crate::walk::collect_closure_shallow;
use arks_object::RawObject;
use arks_oid::Oid;
use arks_pack::EncodedPack;
use arks_protocol::{Packet, decode_packet, decode_text_line, encode_packet, encode_text_line};
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashSet;

const RECEIVE_AD: &str = "application/x-git-receive-pack-advertisement";
const RECEIVE_REQ: &str = "application/x-git-receive-pack-request";
const RECEIVE_RES: &str = "application/x-git-receive-pack-result";

/// One ref update. `new_oid` is the local tip to publish under `refname`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushUpdate {
    pub refname: String,
    pub new_oid: Oid,
    /// When set (`--force-with-lease`), use as the receive-pack command `old`
    /// OID instead of the advertisement tip (O4).
    pub expected_old: Option<Oid>,
}

/// Options for a single push dialog (O4 push-options).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushOptions {
    pub push_options: Vec<String>,
}

/// Outcome of one ref of a push ([[sot:root/arks-remote]] 7d).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushRefStatus {
    /// The peer reported `ok <ref>`.
    Ok,
    /// The advertised tip already equals the new value; no command was sent.
    UpToDate,
    /// The peer reported `ng <ref> <reason>` (reason verbatim), or did not
    /// report the ref at all.
    Rejected(String),
}

/// One entry of a per-ref push report, in the order of the input updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRefResult {
    pub refname: String,
    pub status: PushRefStatus,
}

/// Legacy all-or-nothing view: fail when the peer rejected any ref.
fn all_or_nothing(report: Vec<PushRefResult>) -> Result<(), RemoteError> {
    let problems: Vec<String> = report
        .iter()
        .filter_map(|r| match &r.status {
            PushRefStatus::Rejected(why) => Some(format!("{} {why}", r.refname)),
            _ => None,
        })
        .collect();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(RemoteError::Rejected(problems.join("; ")))
    }
}

/// Every update reported as accepted (dumb push and empty dialogs).
fn all_ok(updates: &[PushUpdate], skipped: &HashSet<String>) -> Vec<PushRefResult> {
    updates
        .iter()
        .map(|u| PushRefResult {
            refname: u.refname.clone(),
            status: if skipped.contains(&u.refname) {
                PushRefStatus::UpToDate
            } else {
                PushRefStatus::Ok
            },
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RefStatus {
    pub name: String,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReportStatus {
    pub unpack: String,
    pub refs: Vec<RefStatus>,
}

pub fn push_http(repo: &Repository, url: &str, updates: &[PushUpdate]) -> Result<(), RemoteError> {
    push_http_with(repo, url, updates, &PushOptions::default())
}

pub fn push_http_with(
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<(), RemoteError> {
    all_or_nothing(push_http_report(repo, url, updates, opts)?)
}

/// Smart/dumb HTTP push returning the per-ref report (rule 7d).
pub fn push_http_report(
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<Vec<PushRefResult>, RemoteError> {
    let mut transport = UreqTransport::new();
    push_report_with(&mut transport, repo, url, updates, opts)
}

/// Push over OpenSSH speaking receive-pack pkt-line.
pub fn push_ssh(repo: &Repository, url: &str, updates: &[PushUpdate]) -> Result<(), RemoteError> {
    push_ssh_with(repo, url, updates, &PushOptions::default())
}

pub fn push_ssh_with(
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<(), RemoteError> {
    all_or_nothing(push_ssh_report(repo, url, updates, opts)?)
}

/// SSH push returning the per-ref report (rule 7d).
pub fn push_ssh_report(
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<Vec<PushRefResult>, RemoteError> {
    validate_push_updates(updates)?;
    let ssh_url = SshRemoteUrl::parse(url)?;
    let mut session = SshSession::connect(&ssh_url, SshService::ReceivePack)?;
    let ad = session.read_advertisement()?;
    let (body, skipped) = build_push_body(repo, &ad, updates, opts)?;
    if skipped.len() == updates.len() {
        // Nothing to send; drop the session (peer may exit non-zero on empty dialog).
        drop(session);
        return Ok(all_ok(updates, &skipped));
    }
    session.write_all(&body)?;
    session.close_stdin();
    // receive-pack sends report-status then exits.
    let report_bytes = session.read_until_flush()?;
    let report = report_per_ref(&report_bytes, updates, &skipped)?;
    session.finish()?;
    Ok(report)
}

/// Push over `git://` speaking receive-pack (peer must enable receive-pack).
pub fn push_git(repo: &Repository, url: &str, updates: &[PushUpdate]) -> Result<(), RemoteError> {
    push_git_with(repo, url, updates, &PushOptions::default())
}

pub fn push_git_with(
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<(), RemoteError> {
    all_or_nothing(push_git_report(repo, url, updates, opts)?)
}

/// `git://` push returning the per-ref report (rule 7d).
pub fn push_git_report(
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<Vec<PushRefResult>, RemoteError> {
    validate_push_updates(updates)?;
    let git_url = GitDaemonUrl::parse(url)?;
    let mut session = GitDaemonSession::connect(&git_url, GitDaemonService::ReceivePack)?;
    let ad = session.read_advertisement()?;
    let (body, skipped) = build_push_body(repo, &ad, updates, opts)?;
    if skipped.len() == updates.len() {
        drop(session);
        return Ok(all_ok(updates, &skipped));
    }
    session.write_all(&body)?;
    session.close_write();
    let report_bytes = session.read_until_flush()?;
    report_per_ref(&report_bytes, updates, &skipped)
}

/// Dispatch HTTP / SSH / `git://` from the remote URL scheme.
pub fn push_remote(
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
) -> Result<(), RemoteError> {
    push_remote_with(repo, url, updates, &PushOptions::default())
}

pub fn push_remote_with(
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<(), RemoteError> {
    all_or_nothing(push_remote_report(repo, url, updates, opts)?)
}

/// Dispatch HTTP / SSH / `git://` and return the per-ref report (rule 7d):
/// `Err` only when the whole push failed (transport, protocol, unpack).
pub fn push_remote_report(
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<Vec<PushRefResult>, RemoteError> {
    if SshRemoteUrl::parse(url).is_ok() {
        push_ssh_report(repo, url, updates, opts)
    } else if GitDaemonUrl::parse(url).is_ok() {
        push_git_report(repo, url, updates, opts)
    } else {
        push_http_report(repo, url, updates, opts)
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn push_with<T: Transport>(
    transport: &mut T,
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<(), RemoteError> {
    all_or_nothing(push_report_with(transport, repo, url, updates, opts)?)
}

pub(crate) fn push_report_with<T: Transport>(
    transport: &mut T,
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<Vec<PushRefResult>, RemoteError> {
    validate_push_updates(updates)?;

    let info_url = join_url(url, "info/refs?service=git-receive-pack");
    let info = transport.exchange(&HttpRequest {
        method: "GET".into(),
        url: info_url,
        headers: vec![("Accept".into(), RECEIVE_AD.into())],
        body: Vec::new(),
    })?;
    require_success(&info, "info/refs")?;
    // Smart clients fall back to dumb when the advertisement is not smart
    // (gitprotocol-http). Dumb push uses WebDAV (git-http-push / RFC 4918).
    if crate::dumb::looks_like_dumb_info_refs(header(&info.headers, "content-type"), &info.body) {
        if !opts.push_options.is_empty() {
            return Err(RemoteError::NotImplemented(
                "push --push-option is not claimed on dumb HTTP (pack-protocol only)".into(),
            ));
        }
        // Dumb force-with-lease: CAS under exclusive WebDAV LOCK (re-GET tip
        // before PUT). Smart-pack lease remains the primary Claimed path.
        crate::dumb::push_dumb_with(transport, repo, url, updates, &info.body)?;
        return Ok(all_ok(updates, &HashSet::new()));
    }
    if !content_type_is(
        header(&info.headers, "content-type"),
        "git-receive-pack-advertisement",
    ) {
        return Err(RemoteError::Protocol(
            "info/refs was not a git-receive-pack advertisement".into(),
        ));
    }
    let ad = parse_advertisement(&info.body, "git-receive-pack")?;
    ensure_remote_object_format(repo, &ad)?;
    let (body, skipped) = build_push_body(repo, &ad, updates, opts)?;
    if skipped.len() == updates.len() {
        return Ok(all_ok(updates, &skipped));
    }

    let post_url = join_url(url, "git-receive-pack");
    let posted = transport.exchange(&HttpRequest {
        method: "POST".into(),
        url: post_url,
        headers: vec![
            ("Content-Type".into(), RECEIVE_REQ.into()),
            ("Accept".into(), RECEIVE_RES.into()),
        ],
        body,
    })?;
    require_success(&posted, "git-receive-pack")?;
    if !content_type_is(
        header(&posted.headers, "content-type"),
        "git-receive-pack-result",
    ) {
        return Err(RemoteError::Protocol(
            "git-receive-pack result had an unexpected content type".into(),
        ));
    }
    report_per_ref(&posted.body, updates, &skipped)
}

fn validate_push_updates(updates: &[PushUpdate]) -> Result<(), RemoteError> {
    if updates.is_empty() {
        return Err(RemoteError::Protocol("push with no refs".into()));
    }
    for update in updates {
        if !update.refname.starts_with("refs/") {
            return Err(RemoteError::Protocol(format!(
                "push ref must be fully qualified, got {}",
                update.refname
            )));
        }
    }
    Ok(())
}

/// Build receive-pack command+pack body plus the refs that need no command
/// (advertised tip already equals the new value). All skipped = nothing to push.
fn build_push_body(
    repo: &Repository,
    ad: &Advertisement,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<(Vec<u8>, HashSet<String>), RemoteError> {
    let want_push_options = !opts.push_options.is_empty();
    if want_push_options && !cap_advertised(&ad.capabilities, "push-options") {
        return Err(RemoteError::NotImplemented(
            "peer did not advertise push-options; --push-option not claimed against this peer"
                .into(),
        ));
    }
    let caps = push_capability_list(&ad.capabilities, want_push_options)?;

    let mut commands: Vec<(Oid, Oid, String)> = Vec::new();
    let mut skipped = HashSet::new();
    let mut any_delete = false;
    for update in updates {
        let advertised = ad
            .refs
            .iter()
            .find(|r| r.name == update.refname)
            .map(|r| r.oid)
            .unwrap_or_else(zero_oid);
        let old = if let Some(expect) = update.expected_old {
            // Fail closed early when the tip already moved (force-with-lease).
            if advertised != expect {
                return Err(RemoteError::Rejected(format!(
                    "force-with-lease failed for {}: expected {}, remote has {}",
                    update.refname,
                    expect.to_hex(),
                    advertised.to_hex()
                )));
            }
            expect
        } else {
            advertised
        };
        if old == update.new_oid {
            skipped.insert(update.refname.clone());
            continue;
        }
        if update.new_oid.as_bytes().iter().all(|b| *b == 0) {
            any_delete = true;
        }
        commands.push((old, update.new_oid, update.refname.clone()));
    }
    if commands.is_empty() {
        return Ok((Vec::new(), skipped));
    }
    if any_delete && !cap_advertised(&ad.capabilities, "delete-refs") {
        return Err(RemoteError::NotImplemented(
            "peer did not advertise delete-refs; push delete not claimed against this peer".into(),
        ));
    }

    let mut tips = Vec::new();
    let mut bases = Vec::new();
    for (old, new, _) in &commands {
        // Deletes send zero new-oid and no pack objects for that tip.
        if !new.as_bytes().iter().all(|b| *b == 0) {
            tips.push(*new);
        }
        if !old.as_bytes().iter().all(|b| *b == 0) && repo.store().exists(old).unwrap_or(false) {
            bases.push(*old);
        }
    }
    let needed = objects_to_send(repo, &tips, &bases)?;
    let pack = if needed.is_empty() {
        Vec::new()
    } else {
        EncodedPack::from_undeltified(repo.object_format(), &needed)
            .map_err(|e| RemoteError::Protocol(format!("pack encode: {e}")))?
            .pack
    };

    let mut body = Vec::new();
    for (i, (old, new, name)) in commands.iter().enumerate() {
        let cap = if i == 0 { Some(caps.as_str()) } else { None };
        body.extend(command_pkt(old, new, name, cap)?);
    }
    body.extend(encode_packet(&Packet::Flush).map_err(|e| RemoteError::Protocol(e.to_string()))?);
    if want_push_options {
        for opt in &opts.push_options {
            body.extend(
                encode_text_line(&format!("push-option {opt}"))
                    .map_err(|e| RemoteError::Protocol(e.to_string()))?,
            );
        }
        body.extend(
            encode_packet(&Packet::Flush).map_err(|e| RemoteError::Protocol(e.to_string()))?,
        );
    }
    body.extend_from_slice(&pack);
    Ok((body, skipped))
}

/// Map `report-status` onto the input updates (rule 7d). Only a failed
/// `unpack` or a report without ref lines fails the whole push.
fn report_per_ref(
    body: &[u8],
    updates: &[PushUpdate],
    skipped: &HashSet<String>,
) -> Result<Vec<PushRefResult>, RemoteError> {
    let report = parse_report_status(body)?;
    if report.unpack != "ok" {
        return Err(RemoteError::Rejected(format!("unpack {}", report.unpack)));
    }
    if report.refs.is_empty() {
        return Err(RemoteError::Protocol(
            "report-status contained no ref lines".into(),
        ));
    }
    Ok(updates
        .iter()
        .map(|u| {
            let status = if skipped.contains(&u.refname) {
                PushRefStatus::UpToDate
            } else {
                match report.refs.iter().find(|r| r.name == u.refname) {
                    Some(r) if r.ok => PushRefStatus::Ok,
                    Some(r) => PushRefStatus::Rejected(
                        r.error.clone().unwrap_or_else(|| "rejected".into()),
                    ),
                    // A ref the peer left out of report-status is not accepted.
                    None => PushRefStatus::Rejected("no report-status line for this ref".into()),
                }
            };
            PushRefResult {
                refname: u.refname.clone(),
                status,
            }
        })
        .collect())
}

fn push_capability_list(
    advertised: &[String],
    want_push_options: bool,
) -> Result<String, RemoteError> {
    let mut caps = Vec::new();
    if cap_advertised(advertised, "report-status") {
        caps.push("report-status".to_string());
    } else if cap_advertised(advertised, "report-status-v2") {
        caps.push("report-status-v2".to_string());
    } else {
        return Err(RemoteError::Protocol(
            "peer did not advertise report-status".into(),
        ));
    }
    if want_push_options && cap_advertised(advertised, "push-options") {
        caps.push("push-options".to_string());
    }
    if cap_advertised(advertised, "agent") {
        caps.push("agent=arks/0.0.0".to_string());
    }
    Ok(caps.join(" "))
}

pub(crate) fn objects_to_send(
    repo: &Repository,
    tips: &[Oid],
    bases: &[Oid],
) -> Result<Vec<RawObject>, RemoteError> {
    let get = |oid: &Oid| match repo.store().get(oid) {
        Ok(obj) => Ok(Some(obj)),
        Err(arks_store::StoreError::NotFound(_)) => Ok(None),
        Err(e) => Err(RemoteError::Msg(e.to_string())),
    };
    // [[sot:root/arks-remote]] 14a: shallow boundary commits are parentless.
    let shallow: HashSet<Oid> = std::fs::read_to_string(repo.common_dir().join("shallow"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| Oid::from_hex(l.trim()).ok())
        .collect();
    let all = collect_closure_shallow(tips, &shallow, false, get)?;
    if bases.is_empty() {
        return Ok(all);
    }
    // Bases (remote tips) may be absent locally or beyond the shallow boundary;
    // only objects we hold can be excluded.
    let have = collect_closure_shallow(bases, &shallow, true, get)?;
    let have_ids: HashSet<Oid> = have
        .iter()
        .map(|obj| obj.oid(repo.object_format()))
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|e| RemoteError::Protocol(e.to_string()))?;
    Ok(all
        .into_iter()
        .filter(|obj| match obj.oid(repo.object_format()) {
            Ok(oid) => !have_ids.contains(&oid),
            Err(_) => true,
        })
        .collect())
}

fn command_pkt(
    old: &Oid,
    new: &Oid,
    name: &str,
    caps: Option<&str>,
) -> Result<Vec<u8>, RemoteError> {
    let mut payload = format!("{} {} {name}", old.to_hex(), new.to_hex()).into_bytes();
    if let Some(caps) = caps {
        payload.push(0);
        payload.extend_from_slice(caps.as_bytes());
    }
    payload.push(b'\n');
    encode_packet(&Packet::Data(payload)).map_err(|e| RemoteError::Protocol(e.to_string()))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn encode_report_status(
    unpack: &str,
    refs: &[(&str, Result<(), &str>)],
) -> Result<Vec<u8>, RemoteError> {
    let mut out = Vec::new();
    out.extend(text_pkt(&format!("unpack {unpack}"))?);
    for (name, result) in refs {
        match result {
            Ok(()) => out.extend(text_pkt(&format!("ok {name}"))?),
            Err(err) => out.extend(text_pkt(&format!("ng {name} {err}"))?),
        }
    }
    out.extend(encode_packet(&Packet::Flush).map_err(|e| RemoteError::Protocol(e.to_string()))?);
    Ok(out)
}

pub(crate) fn parse_report_status(body: &[u8]) -> Result<ReportStatus, RemoteError> {
    let mut offset = 0;
    let mut unpack = None;
    let mut refs = Vec::new();
    if body.is_empty() {
        return Err(RemoteError::Protocol("empty report-status".into()));
    }
    while offset < body.len() {
        let (packet, n) =
            decode_packet(&body[offset..]).map_err(|e| RemoteError::Protocol(e.to_string()))?;
        offset += n;
        match packet {
            Packet::Flush => break,
            Packet::Data(payload) => {
                let text =
                    decode_text_line(&payload).map_err(|e| RemoteError::Protocol(e.to_string()))?;
                if let Some(rest) = text.strip_prefix("unpack ") {
                    unpack = Some(rest.to_string());
                } else if let Some(rest) = text.strip_prefix("ok ") {
                    refs.push(RefStatus {
                        name: rest.to_string(),
                        ok: true,
                        error: None,
                    });
                } else if let Some(rest) = text.strip_prefix("ng ") {
                    let (name, err) = match rest.split_once(' ') {
                        Some((name, err)) => (name.to_string(), err.to_string()),
                        None => (rest.to_string(), "rejected".into()),
                    };
                    refs.push(RefStatus {
                        name,
                        ok: false,
                        error: Some(err),
                    });
                }
            }
            _ => {}
        }
    }
    let unpack =
        unpack.ok_or_else(|| RemoteError::Protocol("report-status missing unpack line".into()))?;
    Ok(ReportStatus { unpack, refs })
}

fn text_pkt(text: &str) -> Result<Vec<u8>, RemoteError> {
    let mut payload = text.as_bytes().to_vec();
    if !payload.ends_with(b"\n") {
        payload.push(b'\n');
    }
    encode_packet(&Packet::Data(payload)).map_err(|e| RemoteError::Protocol(e.to_string()))
}

pub(crate) fn zero_oid() -> Oid {
    Oid::from_hex("0000000000000000000000000000000000000000").expect("sha1 zero id")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_status_roundtrip_ok_and_ng() {
        let body = encode_report_status(
            "ok",
            &[
                ("refs/heads/main", Ok(())),
                ("refs/heads/other", Err("non-fast-forward")),
            ],
        )
        .unwrap();
        let report = parse_report_status(&body).unwrap();
        assert_eq!(report.unpack, "ok");
        assert!(report.refs[0].ok);
        assert!(!report.refs[1].ok);
        assert_eq!(report.refs[1].error.as_deref(), Some("non-fast-forward"));
    }
}
