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

use crate::advertise::{Advertisement, cap_advertised, content_type_is, parse_advertisement};
use crate::error::RemoteError;
use crate::git::{GitDaemonService, GitDaemonSession};
use crate::http::UreqTransport;
use crate::ssh::{SshService, SshSession};
use crate::transport::{HttpRequest, Transport, header, require_success};
use crate::url::{GitDaemonUrl, SshRemoteUrl, join_url};
use crate::walk::collect_closure;
use arks_object::RawObject;
use arks_oid::Oid;
use arks_pack::EncodedPack;
use arks_protocol::{Packet, decode_packet, decode_text_line, encode_packet, encode_text_line};
use arks_repo::Repository;
use arks_store::{ObjectFormat, ObjectStore};
use std::collections::HashSet;

const RECEIVE_AD: &str = "application/x-git-receive-pack-advertisement";
const RECEIVE_REQ: &str = "application/x-git-receive-pack-request";
const RECEIVE_RES: &str = "application/x-git-receive-pack-result";
const FORMAT: ObjectFormat = ObjectFormat::Sha1;

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
    let mut transport = UreqTransport::new();
    push_with(&mut transport, repo, url, updates, opts)
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
    validate_push_updates(updates)?;
    let ssh_url = SshRemoteUrl::parse(url)?;
    let mut session = SshSession::connect(&ssh_url, SshService::ReceivePack)?;
    let ad = session.read_advertisement()?;
    let (body, skip) = build_push_body(repo, &ad, updates, opts)?;
    if skip {
        // Nothing to send; drop the session (peer may exit non-zero on empty dialog).
        drop(session);
        return Ok(());
    }
    session.write_all(&body)?;
    session.close_stdin();
    // receive-pack sends report-status then exits.
    let report_bytes = session.read_until_flush()?;
    apply_report_status(&report_bytes)?;
    session.finish()?;
    Ok(())
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
    validate_push_updates(updates)?;
    let git_url = GitDaemonUrl::parse(url)?;
    let mut session = GitDaemonSession::connect(&git_url, GitDaemonService::ReceivePack)?;
    let ad = session.read_advertisement()?;
    let (body, skip) = build_push_body(repo, &ad, updates, opts)?;
    if skip {
        drop(session);
        return Ok(());
    }
    session.write_all(&body)?;
    session.close_write();
    let report_bytes = session.read_until_flush()?;
    apply_report_status(&report_bytes)?;
    Ok(())
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
    if SshRemoteUrl::parse(url).is_ok() {
        push_ssh_with(repo, url, updates, opts)
    } else if GitDaemonUrl::parse(url).is_ok() {
        push_git_with(repo, url, updates, opts)
    } else {
        push_http_with(repo, url, updates, opts)
    }
}

pub(crate) fn push_with<T: Transport>(
    transport: &mut T,
    repo: &Repository,
    url: &str,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<(), RemoteError> {
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
        if updates.iter().any(|u| u.expected_old.is_some()) {
            return Err(RemoteError::NotImplemented(
                "push --force-with-lease is not claimed on dumb HTTP (smart pack only)".into(),
            ));
        }
        return crate::dumb::push_dumb_with(transport, repo, url, updates, &info.body);
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
    let (body, skip) = build_push_body(repo, &ad, updates, opts)?;
    if skip {
        return Ok(());
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
    apply_report_status(&posted.body)
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

/// Build receive-pack command+pack body. `skip == true` means nothing to push.
fn build_push_body(
    repo: &Repository,
    ad: &Advertisement,
    updates: &[PushUpdate],
    opts: &PushOptions,
) -> Result<(Vec<u8>, bool), RemoteError> {
    let want_push_options = !opts.push_options.is_empty();
    if want_push_options && !cap_advertised(&ad.capabilities, "push-options") {
        return Err(RemoteError::NotImplemented(
            "peer did not advertise push-options; --push-option not claimed against this peer"
                .into(),
        ));
    }
    let caps = push_capability_list(&ad.capabilities, want_push_options)?;

    let mut commands: Vec<(Oid, Oid, String)> = Vec::new();
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
            continue;
        }
        if update.new_oid.as_bytes().iter().all(|b| *b == 0) {
            any_delete = true;
        }
        commands.push((old, update.new_oid, update.refname.clone()));
    }
    if commands.is_empty() {
        return Ok((Vec::new(), true));
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
        EncodedPack::from_undeltified(FORMAT, &needed)
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
    Ok((body, false))
}

fn apply_report_status(body: &[u8]) -> Result<(), RemoteError> {
    let report = parse_report_status(body)?;
    if report.unpack != "ok" {
        return Err(RemoteError::Rejected(format!("unpack {}", report.unpack)));
    }
    let mut problems = Vec::new();
    for r in &report.refs {
        if !r.ok {
            problems.push(format!(
                "{} {}",
                r.name,
                r.error.as_deref().unwrap_or("rejected")
            ));
        }
    }
    if report.refs.is_empty() {
        return Err(RemoteError::Protocol(
            "report-status contained no ref lines".into(),
        ));
    }
    if !problems.is_empty() {
        return Err(RemoteError::Rejected(problems.join("; ")));
    }
    Ok(())
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
    let all = collect_closure(tips, get)?;
    if bases.is_empty() {
        return Ok(all);
    }
    let have = collect_closure(bases, get)?;
    let have_ids: HashSet<Oid> = have
        .iter()
        .map(|obj| obj.oid(FORMAT))
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|e| RemoteError::Protocol(e.to_string()))?;
    Ok(all
        .into_iter()
        .filter(|obj| match obj.oid(FORMAT) {
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
