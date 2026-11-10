//! Dumb HTTP static paths + `update-server-info`.
//!
//! Primary: <https://git-scm.com/docs/gitprotocol-http> (dumb clients / dumb server),
//! <https://git-scm.com/docs/git-update-server-info>.
//! Dumb-HTTP-only as the sole claimed server path remains forbidden ([[sot:root/arks-server]]).

use crate::error::ServerError;
use arks_object::loose_rel_path;
use arks_oid::Oid;
use arks_repo::Repository;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Write `info/refs` and `objects/info/packs` for dumb HTTP clients.
pub fn update_server_info(repo: &Repository) -> Result<(), ServerError> {
    let git_dir = repo.git_dir();
    let info_dir = git_dir.join("info");
    fs::create_dir_all(&info_dir)?;
    let refs_body = dumb_info_refs_body(repo)?;
    atomic_write(&info_dir.join("refs"), &refs_body)?;

    let objects_info = git_dir.join("objects/info");
    fs::create_dir_all(&objects_info)?;
    let packs_body = dumb_packs_list(git_dir)?;
    atomic_write(&objects_info.join("packs"), &packs_body)?;
    Ok(())
}

/// Build dumb `info/refs` body (TAB-separated; no HEAD; C-locale sort).
pub fn dumb_info_refs_body(repo: &Repository) -> Result<Vec<u8>, ServerError> {
    let mut refs = repo
        .refs()
        .list_refs("refs/")
        .map_err(|e| ServerError::Msg(e.to_string()))?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::new();
    for (name, oid) in refs {
        // Peeled tags (`name^{}`) not claimed in first dumb ship — direct refs only.
        writeln!(out, "{}\t{name}", oid.to_hex()).map_err(|e| ServerError::Io(e.to_string()))?;
    }
    Ok(out)
}

fn dumb_packs_list(git_dir: &Path) -> Result<Vec<u8>, ServerError> {
    let pack_dir = git_dir.join("objects/pack");
    let mut names = Vec::new();
    if pack_dir.is_dir() {
        for entry in fs::read_dir(&pack_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("pack-") && name.ends_with(".pack") {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    let mut out = Vec::new();
    for name in names {
        // git-update-server-info: "P pack-<hash>.pack"
        writeln!(out, "P {name}").map_err(|e| ServerError::Io(e.to_string()))?;
    }
    Ok(out)
}

/// Resolve a dumb HTTP GET path under the repository to bytes + content-type.
///
/// Paths are relative to `$GIT_URL` (e.g. `info/refs`, `objects/ab/cd…`,
/// `objects/info/packs`, `objects/pack/pack-….pack`).
pub fn serve_dumb_path(
    repo: &Repository,
    rel: &str,
) -> Result<Option<(String, Vec<u8>)>, ServerError> {
    let rel = rel.trim_start_matches('/');
    let git_dir = repo.git_dir();
    // Dumb clients also GET `$GIT_URL/HEAD` (plain text) for default branch checkout.
    if rel == "HEAD" || rel.ends_with("/HEAD") {
        let path = git_dir.join("HEAD");
        if path.is_file() {
            return Ok(Some(("text/plain".into(), fs::read(path)?)));
        }
        return Ok(None);
    }
    if rel == "info/refs" || rel.ends_with("/info/refs") {
        // Prefer on-disk file (after update-server-info); else generate.
        let path = git_dir.join("info/refs");
        let body = if path.is_file() {
            fs::read(&path)?
        } else {
            dumb_info_refs_body(repo)?
        };
        return Ok(Some(("text/plain; charset=utf-8".into(), body)));
    }
    if rel == "objects/info/packs" || rel.ends_with("/objects/info/packs") {
        let path = git_dir.join("objects/info/packs");
        let body = if path.is_file() {
            fs::read(&path)?
        } else {
            dumb_packs_list(git_dir)?
        };
        return Ok(Some(("text/plain; charset=utf-8".into(), body)));
    }
    if let Some(rest) = strip_objects_prefix(rel) {
        if rest.starts_with("pack/") {
            let name = &rest["pack/".len()..];
            if !is_safe_pack_name(name) {
                return Ok(None);
            }
            let path = git_dir.join("objects/pack").join(name);
            if path.is_file() {
                let ct = if name.ends_with(".idx") {
                    "application/x-git-packed-idx"
                } else {
                    "application/x-git-packed-objects"
                };
                return Ok(Some((ct.into(), fs::read(path)?)));
            }
            return Ok(None);
        }
        // Loose object: XX/rest (40 hex total).
        if let Some(oid) = parse_loose_rel(rest) {
            let path = git_dir.join("objects").join(loose_rel_path(&oid));
            if path.is_file() {
                return Ok(Some((
                    "application/x-git-loose-object".into(),
                    fs::read(path)?,
                )));
            }
        }
    }
    Ok(None)
}

fn strip_objects_prefix(rel: &str) -> Option<&str> {
    if let Some(rest) = rel.strip_prefix("objects/") {
        return Some(rest);
    }
    // Allow "/repo.git/objects/…" style by finding `/objects/`.
    if let Some(idx) = rel.find("/objects/") {
        return Some(&rel[idx + "/objects/".len()..]);
    }
    None
}

fn parse_loose_rel(rest: &str) -> Option<Oid> {
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() != 2 {
        return None;
    }
    let hex = format!("{}{}", parts[0], parts[1]);
    if hex.len() != 40 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Oid::from_hex(&hex).ok()
}

fn is_safe_pack_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
        && (name.ends_with(".pack") || name.ends_with(".idx"))
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<(), ServerError> {
    let tmp = PathBuf::from(format!("{}.tmp-arks", path.display()));
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
