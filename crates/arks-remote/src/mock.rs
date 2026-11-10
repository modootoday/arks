//! In-memory smart HTTP v1 peer for offline tests.
//!
//! This is not a product server. It answers `info/refs` and the two service
//! POSTs with packs produced by `arks-pack` so client tests need no network
//! and no `git` binary. Optional `prefer_dumb` serves dumb-HTTP bodies so the
//! client fallback path (`looks_like_dumb_info_refs` → `fetch_dumb_with`) can
//! be exercised offline.

use crate::advertise::encode_advertisement;
use crate::error::RemoteError;
use crate::fetch::sideband_pack;
use crate::push::{encode_report_status, zero_oid};
use crate::transport::{HttpRequest, HttpResponse, Transport};
use crate::walk::collect_closure;
use arks_object::{
    Commit, ObjectKind, RawObject, Signature, Tree, TreeEntry, TreeMode, zlib_compress,
};
use arks_oid::Oid;
use arks_pack::{DecodedPack, EncodedPack};
use arks_protocol::{Packet, decode_packet, decode_text_line, encode_packet, encode_text_line};
use arks_store::ObjectFormat;
use std::collections::{BTreeMap, HashMap};

const FORMAT: ObjectFormat = ObjectFormat::Sha1;
const UPLOAD_AD: &str = "application/x-git-upload-pack-advertisement";
const UPLOAD_RES: &str = "application/x-git-upload-pack-result";
const RECEIVE_AD: &str = "application/x-git-receive-pack-advertisement";
const RECEIVE_RES: &str = "application/x-git-receive-pack-result";

#[derive(Debug)]
pub(crate) struct MemoryPeer {
    refs: BTreeMap<String, Oid>,
    objects: HashMap<Oid, RawObject>,
    head: Option<String>,
    /// When set, the next receive-pack reports `ng` and does not move refs.
    pub reject_push: bool,
    /// When set, upload-pack `info/refs` returns a dumb body + loose object GETs
    /// (client must fall back; no smart pack POST).
    pub prefer_dumb: bool,
}

impl MemoryPeer {
    pub(crate) fn new() -> Self {
        Self {
            refs: BTreeMap::new(),
            objects: HashMap::new(),
            head: None,
            reject_push: false,
            prefer_dumb: false,
        }
    }

    pub(crate) fn ref_oid(&self, name: &str) -> Option<Oid> {
        self.refs.get(name).copied()
    }

    pub(crate) fn object(&self, oid: &Oid) -> Option<&RawObject> {
        self.objects.get(oid)
    }

    /// One-file root commit on `branch` (`refs/heads/...`).
    pub(crate) fn seed_file(
        &mut self,
        branch: &str,
        path: &str,
        bytes: &[u8],
        message: &str,
    ) -> Result<Oid, RemoteError> {
        if path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        {
            return Err(RemoteError::Protocol(format!("unsafe path {path}")));
        }
        let blob = self.insert(RawObject::new(ObjectKind::Blob, bytes.to_vec()))?;
        let mut oid = blob;
        let parts: Vec<&str> = path.split('/').collect();
        for (i, part) in parts.iter().rev().enumerate() {
            let mode = if i == 0 {
                TreeMode::File
            } else {
                TreeMode::Tree
            };
            let tree = Tree::new(vec![TreeEntry {
                mode,
                name: (*part).to_string(),
                oid,
            }])
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
            let obj = RawObject::new(
                ObjectKind::Tree,
                tree.encode().map_err(|e| RemoteError::Msg(e.to_string()))?,
            );
            oid = self.insert(obj)?;
        }
        let sig = Signature {
            name: "arks".into(),
            email: "arks@example.com".into(),
            timestamp: 1_000_000_000,
            timezone: "+0000".into(),
        };
        let message = if message.ends_with('\n') {
            message.to_string()
        } else {
            format!("{message}\n")
        };
        let commit = Commit {
            tree: oid,
            parents: Vec::new(),
            author: sig.clone(),
            committer: sig,
            gpgsig: None,
            message,
        };
        let obj = RawObject::new(
            ObjectKind::Commit,
            commit
                .encode()
                .map_err(|e| RemoteError::Msg(e.to_string()))?,
        );
        let commit_oid = self.insert(obj)?;
        self.refs.insert(branch.to_string(), commit_oid);
        self.head = Some(branch.to_string());
        Ok(commit_oid)
    }

    fn insert(&mut self, obj: RawObject) -> Result<Oid, RemoteError> {
        let oid = obj
            .oid(FORMAT)
            .map_err(|e| RemoteError::Msg(e.to_string()))?;
        self.objects.insert(oid, obj);
        Ok(oid)
    }

    pub(crate) fn handle(&mut self, request: &HttpRequest) -> Result<HttpResponse, RemoteError> {
        let target = request_target(&request.url);
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        if path.ends_with("/info/refs") {
            let service = query_param(query, "service").unwrap_or("");
            if self.prefer_dumb && (service == "git-upload-pack" || service == "git-receive-pack") {
                return Ok(text_response(200, &self.dumb_info_refs_body()));
            }
            let (ct, body) = if service == "git-upload-pack" {
                (UPLOAD_AD, self.advertisement("git-upload-pack")?)
            } else if service == "git-receive-pack" {
                (RECEIVE_AD, self.advertisement("git-receive-pack")?)
            } else {
                return Ok(text_response(403, b"unknown service"));
            };
            return Ok(typed_response(200, ct, body));
        }
        if self.prefer_dumb {
            if path.ends_with("/HEAD") {
                if let Some(head) = &self.head {
                    let body = format!("ref: {head}\n");
                    return Ok(text_response(200, body.as_bytes()));
                }
                return Ok(text_response(404, b"no HEAD"));
            }
            if path.ends_with("/objects/info/packs") {
                return Ok(text_response(404, b""));
            }
            if let Some(oid_hex) = dumb_loose_oid_hex(path) {
                let oid = Oid::from_hex(&oid_hex)
                    .map_err(|e| RemoteError::Protocol(format!("dumb loose oid: {e}")))?;
                let obj = self
                    .objects
                    .get(&oid)
                    .ok_or_else(|| RemoteError::Protocol(format!("missing loose {oid_hex}")))?;
                let compressed = zlib_compress(&obj.encode_uncompressed())
                    .map_err(|e| RemoteError::Protocol(e.to_string()))?;
                return Ok(typed_response(200, "application/octet-stream", compressed));
            }
        }
        if request.method == "POST" && path.ends_with("/git-upload-pack") {
            let body = self.upload_result(&request.body)?;
            return Ok(typed_response(200, UPLOAD_RES, body));
        }
        if request.method == "POST" && path.ends_with("/git-receive-pack") {
            let body = self.receive(&request.body)?;
            return Ok(typed_response(200, RECEIVE_RES, body));
        }
        Ok(text_response(404, b"not found"))
    }

    fn dumb_info_refs_body(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, oid) in &self.refs {
            out.extend_from_slice(oid.to_hex().as_bytes());
            out.push(b'\t');
            out.extend_from_slice(name.as_bytes());
            out.push(b'\n');
        }
        out
    }

    fn advertisement(&self, service: &str) -> Result<Vec<u8>, RemoteError> {
        let mut caps = Vec::new();
        if service == "git-upload-pack" {
            caps.push("ofs-delta".into());
            caps.push("side-band-64k".into());
            caps.push("no-progress".into());
            caps.push("shallow".into());
            if let Some(head) = &self.head {
                caps.push(format!("symref=HEAD:{head}"));
            }
            caps.push("agent=arks/0.0.0".into());
        } else {
            caps.push("report-status".into());
            caps.push("report-status-v2".into());
            caps.push("delete-refs".into());
            caps.push("agent=arks/0.0.0".into());
        }
        let refs: Vec<(String, Oid)> = self
            .refs
            .iter()
            .map(|(name, oid)| (name.clone(), *oid))
            .collect();
        encode_advertisement(service, &caps, &refs)
    }

    fn upload_result(&self, request_body: &[u8]) -> Result<Vec<u8>, RemoteError> {
        let tips: Vec<Oid> = self.refs.values().copied().collect();
        let want_sideband = request_wants_sideband(request_body);
        if tips.is_empty() {
            return if want_sideband {
                sideband_pack(&[])
            } else {
                raw_pack_result(&[])
            };
        }
        let deepen = parse_deepen(request_body);
        let objects = collect_closure(&tips, |oid| Ok(self.objects.get(oid).cloned()))?;
        let pack = EncodedPack::from_undeltified(FORMAT, &objects)
            .map_err(|e| RemoteError::Protocol(format!("pack encode: {e}")))?;
        let mut out = Vec::new();
        // Minimal honest shallow-update: when deepen is requested, mark branch tips shallow.
        // (Does not truncate pack history — host/stock-Git peers prove real deepen.)
        if deepen.is_some() {
            for tip in &tips {
                out.extend(
                    encode_text_line(&format!("shallow {}", tip.to_hex()))
                        .map_err(|e| RemoteError::Protocol(e.to_string()))?,
                );
            }
            out.extend(
                encode_packet(&Packet::Flush).map_err(|e| RemoteError::Protocol(e.to_string()))?,
            );
        }
        if want_sideband {
            out.extend(sideband_pack(&pack.pack)?);
        } else {
            out.extend(raw_pack_result(&pack.pack)?);
        }
        Ok(out)
    }

    fn receive(&mut self, body: &[u8]) -> Result<Vec<u8>, RemoteError> {
        let (commands, pack) = split_commands(body)?;
        if self.reject_push {
            let refs: Vec<(&str, Result<(), &str>)> = commands
                .iter()
                .map(|cmd| (cmd.name.as_str(), Err("rejected")))
                .collect();
            return encode_report_status("ok", &refs);
        }
        if !pack.is_empty() {
            if !pack.starts_with(b"PACK") {
                return encode_report_status("pack corrupt", &[]);
            }
            let decoded = match DecodedPack::decode(&pack, FORMAT) {
                Ok(d) => d,
                Err(e) => return encode_report_status(&e.to_string(), &[]),
            };
            for object in decoded.objects {
                self.objects.insert(object.oid, object.object);
            }
        }
        let mut refs = Vec::new();
        for cmd in &commands {
            if cmd.new.as_bytes().iter().all(|b| *b == 0) {
                self.refs.remove(&cmd.name);
                refs.push((cmd.name.as_str(), Ok(())));
                continue;
            }
            self.refs.insert(cmd.name.clone(), cmd.new);
            refs.push((cmd.name.as_str(), Ok(())));
        }
        let borrowed: Vec<(&str, Result<(), &str>)> = refs
            .iter()
            .map(|(n, r)| {
                (
                    *n,
                    match r {
                        Ok(()) => Ok(()),
                        Err(e) => Err(*e),
                    },
                )
            })
            .collect();
        encode_report_status("ok", &borrowed)
    }
}

impl Transport for MemoryPeer {
    fn exchange(&mut self, request: &HttpRequest) -> Result<HttpResponse, RemoteError> {
        self.handle(request)
    }
}

struct ReceiveCommand {
    new: Oid,
    name: String,
}

fn split_commands(body: &[u8]) -> Result<(Vec<ReceiveCommand>, Vec<u8>), RemoteError> {
    let mut offset = 0;
    let mut commands = Vec::new();
    while offset < body.len() {
        let (packet, n) =
            decode_packet(&body[offset..]).map_err(|e| RemoteError::Protocol(e.to_string()))?;
        offset += n;
        match packet {
            Packet::Flush => break,
            Packet::Data(payload) => {
                let text =
                    decode_text_line(&payload).map_err(|e| RemoteError::Protocol(e.to_string()))?;
                let cmd = text.split('\0').next().unwrap_or("").trim();
                if cmd.is_empty() {
                    continue;
                }
                let mut parts = cmd.split_whitespace();
                let old = parts.next().unwrap_or("");
                let new = parts.next().unwrap_or("");
                let name = parts.next().unwrap_or("");
                if name.is_empty() {
                    return Err(RemoteError::Protocol(format!("bad push command: {cmd}")));
                }
                let _old = Oid::from_hex(old).unwrap_or_else(|_| zero_oid());
                let new = Oid::from_hex(new).unwrap_or_else(|_| zero_oid());
                commands.push(ReceiveCommand {
                    new,
                    name: name.to_string(),
                });
            }
            _ => {}
        }
    }
    Ok((commands, body[offset..].to_vec()))
}

fn parse_deepen(body: &[u8]) -> Option<u32> {
    let mut offset = 0;
    while offset < body.len() {
        let Ok((packet, n)) = decode_packet(&body[offset..]) else {
            break;
        };
        offset += n;
        match packet {
            Packet::Flush => break,
            Packet::Data(payload) => {
                let text = decode_text_line(&payload).unwrap_or_default();
                if let Some(rest) = text.strip_prefix("deepen ") {
                    if let Ok(n) = rest.trim().parse::<u32>() {
                        return Some(n);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// True when the first `want` line lists `side-band` / `side-band-64k`.
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
                let text = decode_text_line(&payload).unwrap_or_default();
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

/// NAK + raw PACK (no side-band), matching stock upload-pack when the client
/// omits side-band capabilities.
fn raw_pack_result(pack: &[u8]) -> Result<Vec<u8>, RemoteError> {
    let mut out = encode_text_line("NAK").map_err(|e| RemoteError::Protocol(e.to_string()))?;
    out.extend_from_slice(pack);
    Ok(out)
}

fn request_target(url: &str) -> &str {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    match rest {
        Some(rest) => rest.find('/').map(|i| &rest[i..]).unwrap_or("/"),
        None => url,
    }
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then_some(v)
    })
}

fn typed_response(status: u16, content_type: &str, body: Vec<u8>) -> HttpResponse {
    HttpResponse {
        status,
        headers: vec![("content-type".into(), content_type.into())],
        body,
    }
}

fn text_response(status: u16, body: &[u8]) -> HttpResponse {
    HttpResponse {
        status,
        headers: vec![("content-type".into(), "text/plain".into())],
        body: body.to_vec(),
    }
}

/// Parse `.../objects/aa/bbbb…` from a request path into a 40-hex OID string.
fn dumb_loose_oid_hex(path: &str) -> Option<String> {
    let idx = path.find("/objects/")?;
    let rest = &path[idx + "/objects/".len()..];
    let (dir, file) = rest.split_once('/')?;
    if dir.len() != 2 || file.len() != 38 {
        return None;
    }
    if !dir.chars().all(|c| c.is_ascii_hexdigit()) || !file.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("{dir}{file}"))
}
