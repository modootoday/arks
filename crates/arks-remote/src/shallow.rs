//! `$GIT_DIR/shallow` read/write for the claimed `--depth` subset.
//!
//! Cite: [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout) (`shallow`),
//! [gitprotocol-pack](https://git-scm.com/docs/gitprotocol-pack) shallow-update,
//! [gitprotocol-capabilities](https://git-scm.com/docs/gitprotocol-capabilities) `shallow`.

use crate::error::RemoteError;
use arks_oid::Oid;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Apply server `shallow` / `unshallow` lines onto the client's prior shallow set.
pub fn apply_shallow_update(
    existing: &[Oid],
    added: &[Oid],
    removed: &[Oid],
) -> Vec<Oid> {
    let mut set: HashSet<Oid> = existing.iter().copied().collect();
    for oid in added {
        set.insert(*oid);
    }
    for oid in removed {
        set.remove(oid);
    }
    let mut out: Vec<Oid> = set.into_iter().collect();
    out.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    out
}

/// Read `$GIT_DIR/shallow` (one hex OID per line). Missing file → empty.
pub fn read_shallow(git_dir: &Path) -> Result<Vec<Oid>, RemoteError> {
    let path = git_dir.join("shallow");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| RemoteError::Msg(e.to_string()))?;
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let oid = Oid::from_hex(line).map_err(|e| {
            RemoteError::Protocol(format!(
                "shallow file line {}: {e}",
                lineno + 1
            ))
        })?;
        out.push(oid);
    }
    Ok(out)
}

/// Write `$GIT_DIR/shallow`. Empty set removes the file (full history locally).
pub fn write_shallow(git_dir: &Path, oids: &[Oid]) -> Result<(), RemoteError> {
    let path = git_dir.join("shallow");
    let lock = git_dir.join("shallow.lock");
    if lock.is_file() {
        return Err(RemoteError::Msg(format!(
            "Unable to create '{}' (concurrent shallow rewrite refuse)",
            lock.display()
        )));
    }
    if oids.is_empty() {
        if path.is_file() {
            fs::remove_file(&path).map_err(|e| RemoteError::Msg(e.to_string()))?;
        }
        return Ok(());
    }
    let mut body = String::new();
    for oid in oids {
        body.push_str(&oid.to_hex());
        body.push('\n');
    }
    // Best-effort lock file for SHAL-001 honesty; atomic rename for the payload.
    fs::write(&lock, b"").map_err(|e| RemoteError::Msg(e.to_string()))?;
    let tmp = git_dir.join("shallow.lock.tmp");
    let write_result = (|| -> Result<(), RemoteError> {
        fs::write(&tmp, body.as_bytes()).map_err(|e| RemoteError::Msg(e.to_string()))?;
        fs::rename(&tmp, &path).map_err(|e| RemoteError::Msg(e.to_string()))?;
        Ok(())
    })();
    let _ = fs::remove_file(&lock);
    if path.is_file() && tmp.is_file() {
        let _ = fs::remove_file(&tmp);
    }
    write_result
}
