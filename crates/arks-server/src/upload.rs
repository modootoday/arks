//! upload-pack service (fetch/clone path).

use crate::advertise::{encode_http_advertisement, encode_pack_advertisement};
use crate::error::ServerError;
use crate::walk::collect_closure;
use arks_oid::Oid;
use arks_pack::EncodedPack;
use arks_protocol::{Packet, decode_packet, decode_text_line, encode_packet, encode_text_line};
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;

type RefsAndCaps = (Vec<(String, Oid)>, Vec<String>);
type RefsAndHead = (Vec<(String, Oid)>, Option<String>);

fn upload_caps(repo: &Repository) -> Result<RefsAndCaps, ServerError> {
    let (refs, head) = list_refs_and_head(repo)?;
    let mut caps = vec![
        "ofs-delta".into(),
        "side-band-64k".into(),
        "no-progress".into(),
        "include-tag".into(),
        "agent=arks/0.0.0".into(),
    ];
    if let Some(h) = &head {
        caps.insert(0, format!("symref=HEAD:{h}"));
    }
    Ok((refs, caps))
}

/// Smart-HTTP upload-pack advertisement (`# service=git-upload-pack`).
pub fn upload_advertisement(repo: &Repository) -> Result<Vec<u8>, ServerError> {
    let (refs, caps) = upload_caps(repo)?;
    encode_http_advertisement("git-upload-pack", &caps, &refs)
}

/// Pack-protocol duplex advertisement (SSH / `git://` / stdio) — no HTTP service line.
pub fn upload_pack_advertisement(repo: &Repository) -> Result<Vec<u8>, ServerError> {
    let (refs, caps) = upload_caps(repo)?;
    encode_pack_advertisement(&caps, &refs)
}

pub fn upload_pack_result(repo: &Repository, body: &[u8]) -> Result<Vec<u8>, ServerError> {
    upload_pack_result_with_nak(repo, body, true)
}

/// Like [`upload_pack_result`], but skip the leading NAK when negotiation already
/// sent one (avoids `bad band #78` when the client has entered side-band mode).
pub fn upload_pack_result_with_nak(
    repo: &Repository,
    body: &[u8],
    include_nak: bool,
) -> Result<Vec<u8>, ServerError> {
    let mut wants = parse_wants(body)?;
    let want_sideband = request_wants_sideband(body);
    if request_has_cap(body, "include-tag") {
        include_tags_for(repo, &mut wants)?;
    }
    if wants.is_empty() {
        return if want_sideband {
            sideband_pack(&[], include_nak)
        } else {
            raw_pack_result(&[], include_nak)
        };
    }
    // J-CLONE-LOCAL / bare clone: tips already live in objects/pack/*.pack.
    // Reuse the on-disk pack instead of re-walking + zlib re-encoding undeltified
    // (profile: cgi_wait ≈26ms was almost entirely pack generate).
    if let Some(pack) = try_reuse_on_disk_pack(repo, &wants)? {
        return if want_sideband {
            sideband_pack(&pack, include_nak)
        } else {
            raw_pack_result(&pack, include_nak)
        };
    }
    let get = |oid: &Oid| match repo.store().get(oid) {
        Ok(obj) => Ok(Some(obj)),
        Err(arks_store::StoreError::NotFound(_)) => Ok(None),
        Err(e) => Err(ServerError::Msg(e.to_string())),
    };
    let objects = collect_closure(&wants, get)?;
    let pack = EncodedPack::from_undeltified(repo.object_format(), &objects)
        .map_err(|e| ServerError::Protocol(format!("pack encode: {e}")))?;
    if want_sideband {
        sideband_pack(&pack.pack, include_nak)
    } else {
        raw_pack_result(&pack.pack, include_nak)
    }
}

/// When every `want` is present in a single on-disk pack+idx, return that pack's
/// bytes. Extra objects in the pack are fine (client did not request thin-pack).
fn try_reuse_on_disk_pack(
    repo: &Repository,
    wants: &[Oid],
) -> Result<Option<Vec<u8>>, ServerError> {
    let pack_dir = repo.store().objects_dir().join("pack");
    if !pack_dir.is_dir() {
        return Ok(None);
    }
    let entries = std::fs::read_dir(&pack_dir).map_err(|e| ServerError::Msg(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| ServerError::Msg(e.to_string()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".pack") {
            continue;
        }
        let pack_path = entry.path();
        let idx_path = pack_path.with_extension("idx");
        if !idx_path.is_file() {
            continue;
        }
        let idx_bytes = std::fs::read(&idx_path).map_err(|e| ServerError::Msg(e.to_string()))?;
        let idx = match arks_pack::PackIndex::decode(&idx_bytes, repo.object_format()) {
            Ok(i) => i,
            Err(_) => continue,
        };
        if wants.iter().all(|w| idx.lookup(w).is_some()) {
            let pack = std::fs::read(&pack_path).map_err(|e| ServerError::Msg(e.to_string()))?;
            if std::env::var_os("ARKS_SERVER_PROFILE").is_some() {
                eprintln!(
                    "UPLOAD_PACK_REUSE path={} wants={} bytes={}",
                    pack_path.display(),
                    wants.len(),
                    pack.len()
                );
            }
            return Ok(Some(pack));
        }
    }
    Ok(None)
}

fn list_refs_and_head(repo: &Repository) -> Result<RefsAndHead, ServerError> {
    let mut refs = repo
        .refs()
        .list_refs("refs/")
        .map_err(|e| ServerError::Msg(e.to_string()))?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    let head = match repo.head_target() {
        Ok(RefTarget::Symbolic(t)) => {
            if let Ok(oid) = repo.refs().resolve("HEAD") {
                if !refs.iter().any(|(n, _)| n == "HEAD") {
                    refs.insert(0, ("HEAD".into(), oid));
                }
            }
            Some(t)
        }
        Ok(RefTarget::Direct(oid)) => {
            if !refs.iter().any(|(n, _)| n == "HEAD") {
                refs.insert(0, ("HEAD".into(), oid));
            }
            None
        }
        Err(_) => None,
    };
    Ok((refs, head))
}

fn parse_wants(body: &[u8]) -> Result<Vec<Oid>, ServerError> {
    let mut offset = 0;
    let mut wants = Vec::new();
    while offset < body.len() {
        let (packet, n) =
            decode_packet(&body[offset..]).map_err(|e| ServerError::Protocol(e.to_string()))?;
        offset += n;
        match packet {
            Packet::Flush => {
                // continue for have/done after flush
            }
            Packet::Data(payload) => {
                let text =
                    decode_text_line(&payload).map_err(|e| ServerError::Protocol(e.to_string()))?;
                if let Some(rest) = text.strip_prefix("want ") {
                    let hex = rest.split_whitespace().next().unwrap_or("");
                    let oid = Oid::from_hex(hex)
                        .map_err(|e| ServerError::Protocol(format!("want oid: {e}")))?;
                    if !wants.contains(&oid) {
                        wants.push(oid);
                    }
                } else if text.starts_with("have ") || text == "done" {
                    // ignore negotiation detail for claimed undeltified full pack
                }
            }
            _ => {}
        }
    }
    Ok(wants)
}

/// True when the first `want` line lists `side-band` / `side-band-64k`.
/// gitprotocol-capabilities `include-tag`: send every annotated tag (and the
/// tags it points to) whose peeled object is a commit the pack already holds,
/// so a client that auto-follows tags gets the tag objects in the same pack.
fn include_tags_for(repo: &Repository, wants: &mut Vec<Oid>) -> Result<(), ServerError> {
    let tags = repo
        .refs()
        .list_refs("refs/tags/")
        .map_err(|e| ServerError::Msg(e.to_string()))?;
    let mut annotated: Vec<(Oid, Oid)> = Vec::new();
    for (_, oid) in &tags {
        if let Some(peeled) = peel_annotated(repo, oid)? {
            annotated.push((*oid, peeled));
        }
    }
    if annotated.is_empty() {
        return Ok(());
    }
    let reachable = reachable_commits(repo, wants)?;
    for (tag, peeled) in annotated {
        if reachable.contains(&peeled) && !wants.contains(&tag) {
            wants.push(tag);
        }
    }
    Ok(())
}

/// The non-tag object an annotated tag (chain) points to; `None` when `oid`
/// is not a tag object (a lightweight tag needs no extra object).
fn peel_annotated(repo: &Repository, oid: &Oid) -> Result<Option<Oid>, ServerError> {
    let mut cur = *oid;
    let mut seen_tag = false;
    for _ in 0..32 {
        let obj = match repo.store().get(&cur) {
            Ok(o) => o,
            Err(arks_store::StoreError::NotFound(_)) => return Ok(None),
            Err(e) => return Err(ServerError::Msg(e.to_string())),
        };
        if obj.kind != arks_object::ObjectKind::Tag {
            return Ok(seen_tag.then_some(cur));
        }
        seen_tag = true;
        let target = std::str::from_utf8(&obj.payload)
            .ok()
            .and_then(|t| t.lines().find_map(|l| l.strip_prefix("object ")))
            .and_then(|h| Oid::from_hex(h.trim()).ok());
        match target {
            Some(t) => cur = t,
            None => return Ok(None),
        }
    }
    Ok(None)
}

/// Commits reachable from `tips` (tag tips peeled), walking parents only.
fn reachable_commits(
    repo: &Repository,
    tips: &[Oid],
) -> Result<std::collections::HashSet<Oid>, ServerError> {
    let mut seen = std::collections::HashSet::new();
    let mut stack: Vec<Oid> = Vec::new();
    for tip in tips {
        stack.push(peel_annotated(repo, tip)?.unwrap_or(*tip));
    }
    while let Some(oid) = stack.pop() {
        if !seen.insert(oid) {
            continue;
        }
        let obj = match repo.store().get(&oid) {
            Ok(o) => o,
            Err(arks_store::StoreError::NotFound(_)) => continue,
            Err(e) => return Err(ServerError::Msg(e.to_string())),
        };
        if obj.kind != arks_object::ObjectKind::Commit {
            continue;
        }
        let commit = arks_object::Commit::decode(&obj.payload)
            .map_err(|e| ServerError::Protocol(e.to_string()))?;
        stack.extend(commit.parents);
    }
    Ok(seen)
}

/// Whether the first `want` line of the request lists capability `cap`.
fn request_has_cap(body: &[u8], cap: &str) -> bool {
    let mut offset = 0;
    while offset < body.len() {
        let Ok((packet, n)) = decode_packet(&body[offset..]) else {
            break;
        };
        offset += n;
        match packet {
            Packet::Flush => break,
            Packet::Data(payload) => {
                let Ok(text) = decode_text_line(&payload) else {
                    continue;
                };
                if let Some(rest) = text.strip_prefix("want ") {
                    return rest.split_whitespace().any(|c| c == cap);
                }
            }
            _ => {}
        }
    }
    false
}

fn request_wants_sideband(body: &[u8]) -> bool {
    let mut offset = 0;
    while offset < body.len() {
        let Ok((packet, n)) = decode_packet(&body[offset..]) else {
            break;
        };
        offset += n;
        match packet {
            Packet::Flush => break,
            Packet::Data(payload) => {
                let Ok(text) = decode_text_line(&payload) else {
                    continue;
                };
                if let Some(rest) = text.strip_prefix("want ") {
                    return rest
                        .split_whitespace()
                        .any(|c| c == "side-band" || c == "side-band-64k");
                }
            }
            _ => {}
        }
    }
    false
}

fn sideband_pack(pack: &[u8], include_nak: bool) -> Result<Vec<u8>, ServerError> {
    // Optional final NAK then side-band-64k pack (gitprotocol-pack).
    let mut out = Vec::new();
    if include_nak {
        out.extend(encode_text_line("NAK").map_err(|e| ServerError::Protocol(e.to_string()))?);
    }
    const CHUNK: usize = 65_514; // 1 band byte + payload ≤ 65516
    for chunk in pack.chunks(CHUNK) {
        let mut payload = Vec::with_capacity(1 + chunk.len());
        payload.push(1); // side-band stream 1 = pack data
        payload.extend_from_slice(chunk);
        out.extend(
            encode_packet(&Packet::Data(payload))
                .map_err(|e| ServerError::Protocol(e.to_string()))?,
        );
    }
    out.extend(encode_packet(&Packet::Flush).map_err(|e| ServerError::Protocol(e.to_string()))?);
    Ok(out)
}

/// NAK (optional) + raw PACK when the client omitted side-band caps.
fn raw_pack_result(pack: &[u8], include_nak: bool) -> Result<Vec<u8>, ServerError> {
    let mut out = Vec::with_capacity(pack.len() + 8);
    if include_nak {
        out.extend(encode_text_line("NAK").map_err(|e| ServerError::Protocol(e.to_string()))?);
    }
    out.extend_from_slice(pack);
    Ok(out)
}
