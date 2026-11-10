//! Protocol v2 server subset: capability advertisement, `ls-refs`, `fetch`.
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-v2>
//! Claimed commands only — never invent capabilities ([[sot:root/arks-server]]).

use crate::error::ServerError;
use crate::walk::collect_closure;
use arks_oid::Oid;
use arks_pack::EncodedPack;
use arks_protocol::{
    Packet, decode_packet, decode_text_line, encode_packet, encode_text_line,
    encode_v2_capability_advertisement,
};
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;


/// Capability lines advertised for v2 (commands + agent).
pub fn v2_capability_lines() -> Vec<String> {
    // gitprotocol-v2: command features as `fetch=<feature>…` (SP-separated values).
    vec![
        "ls-refs".into(),
        "fetch=ofs-delta".into(),
        "agent=arks/0.0.0".into(),
        "object-format=sha1".into(),
    ]
}

pub fn v2_capability_advertisement() -> Result<Vec<u8>, ServerError> {
    encode_v2_capability_advertisement(&v2_capability_lines())
        .map_err(|e| ServerError::Protocol(e.to_string()))
}

/// Handle a v2 command request body (after capability advertisement).
///
/// HTTP (stateless): append response-end-pkt after **fetch** only. Stock Git
/// `ls-refs` over HTTP (2.39+) errors on response-end immediately after the
/// listing flush (`unexpected response end packet`); Content-Length close is
/// enough for that RPC. Cite: gitprotocol-v2 Packet-Line Framing.
pub fn handle_v2_request(
    repo: &Repository,
    body: &[u8],
    stateless: bool,
) -> Result<Vec<u8>, ServerError> {
    let (command, args) = parse_command_request(body)?;
    let out = match command.as_str() {
        "ls-refs" => ls_refs(repo, &args)?,
        "fetch" => fetch_v2(repo, &args)?,
        other => {
            return Err(ServerError::Protocol(format!(
                "unknown or unclaimed v2 command: {other}"
            )));
        }
    };
    // Do not append response-end-pkt: stock Git remote-curl (2.39+) surfaces
    // `unexpected response end packet` for our claimed ls-refs/fetch subset when
    // 0002 follows the command flush. HTTP Content-Length end is sufficient.
    let _ = stateless;
    Ok(out)
}

fn parse_command_request(body: &[u8]) -> Result<(String, Vec<String>), ServerError> {
    let mut offset = 0;
    let mut command = None;
    let mut caps_done = false;
    let mut args = Vec::new();
    while offset < body.len() {
        let (packet, n) =
            decode_packet(&body[offset..]).map_err(|e| ServerError::Protocol(e.to_string()))?;
        offset += n;
        match packet {
            Packet::Flush => break,
            Packet::Delim => {
                caps_done = true;
            }
            Packet::Data(payload) => {
                let text =
                    decode_text_line(&payload).map_err(|e| ServerError::Protocol(e.to_string()))?;
                if command.is_none() {
                    if let Some(rest) = text.strip_prefix("command=") {
                        command = Some(rest.to_string());
                        continue;
                    }
                    return Err(ServerError::Protocol("v2 request missing command=".into()));
                }
                if !caps_done {
                    continue;
                }
                args.push(text);
            }
            Packet::ResponseEnd => break,
        }
    }
    let command =
        command.ok_or_else(|| ServerError::Protocol("v2 request missing command".into()))?;
    Ok((command, args))
}

fn ls_refs(repo: &Repository, args: &[String]) -> Result<Vec<u8>, ServerError> {
    let mut want_symrefs = false;
    let mut prefixes: Vec<String> = Vec::new();
    for a in args {
        if a == "symrefs" {
            want_symrefs = true;
        } else if let Some(p) = a.strip_prefix("ref-prefix ") {
            prefixes.push(p.to_string());
        }
    }

    let mut refs = repo
        .refs()
        .list_refs("refs/")
        .map_err(|e| ServerError::Msg(e.to_string()))?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = Vec::new();
    match repo.head_target() {
        Ok(RefTarget::Symbolic(target)) => {
            if let Ok(oid) = repo.refs().resolve("HEAD") {
                let mut line = format!("{} HEAD", oid.to_hex());
                if want_symrefs {
                    line.push_str(&format!(" symref-target:{target}"));
                }
                out.extend(
                    encode_text_line(&line).map_err(|e| ServerError::Protocol(e.to_string()))?,
                );
            }
        }
        Ok(RefTarget::Direct(oid)) => {
            out.extend(
                encode_text_line(&format!("{} HEAD", oid.to_hex()))
                    .map_err(|e| ServerError::Protocol(e.to_string()))?,
            );
        }
        Err(_) => {}
    }
    for (name, oid) in refs {
        if !prefixes.is_empty() && !prefixes.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        out.extend(
            encode_text_line(&format!("{} {name}", oid.to_hex()))
                .map_err(|e| ServerError::Protocol(e.to_string()))?,
        );
    }
    out.extend(encode_packet(&Packet::Flush).map_err(|e| ServerError::Protocol(e.to_string()))?);
    Ok(out)
}

fn fetch_v2(repo: &Repository, args: &[String]) -> Result<Vec<u8>, ServerError> {
    let mut wants = Vec::new();
    let mut done = false;
    for a in args {
        if let Some(hex) = a.strip_prefix("want ") {
            let oid = Oid::from_hex(hex.trim())
                .map_err(|e| ServerError::Protocol(format!("want oid: {e}")))?;
            if !wants.contains(&oid) {
                wants.push(oid);
            }
        } else if a == "done" {
            done = true;
        }
    }
    if wants.is_empty() {
        return Err(ServerError::Protocol("fetch without want".into()));
    }
    if !done {
        // Ask client to continue / send done — acknowledgments section + flush.
        let mut out = encode_text_line("acknowledgments")
            .map_err(|e| ServerError::Protocol(e.to_string()))?;
        out.extend(encode_text_line("NAK").map_err(|e| ServerError::Protocol(e.to_string()))?);
        out.extend(
            encode_packet(&Packet::Flush).map_err(|e| ServerError::Protocol(e.to_string()))?,
        );
        return Ok(out);
    }

    let get = |oid: &Oid| match repo.store().get(oid) {
        Ok(obj) => Ok(Some(obj)),
        Err(arks_store::StoreError::NotFound(_)) => Ok(None),
        Err(e) => Err(ServerError::Msg(e.to_string())),
    };
    let objects = collect_closure(&wants, get)?;
    let pack = EncodedPack::from_undeltified(repo.object_format(), &objects)
        .map_err(|e| ServerError::Protocol(format!("pack encode: {e}")))?;

    // done was sent → omit acknowledgments; send packfile section only.
    let mut out = encode_text_line("packfile").map_err(|e| ServerError::Protocol(e.to_string()))?;
    const CHUNK: usize = 65_515;
    for chunk in pack.pack.chunks(CHUNK) {
        let mut payload = Vec::with_capacity(1 + chunk.len());
        payload.push(1);
        payload.extend_from_slice(chunk);
        out.extend(
            encode_packet(&Packet::Data(payload))
                .map_err(|e| ServerError::Protocol(e.to_string()))?,
        );
    }
    out.extend(encode_packet(&Packet::Flush).map_err(|e| ServerError::Protocol(e.to_string()))?);
    Ok(out)
}
