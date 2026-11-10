//! Loose ref file content parse/format.

use crate::RefsError;
use crate::target::RefTarget;
use arks_oid::Oid;
use std::path::{Path, PathBuf};

/// Map a validated refname to a path under `git_dir`.
pub fn ref_file_path(git_dir: &Path, name: &str) -> PathBuf {
    git_dir.join(name)
}

/// Read a single ref file into a [`RefTarget`] (no symref follow).
pub fn parse_ref_contents(bytes: &[u8]) -> Result<RefTarget, RefsError> {
    let text = std::str::from_utf8(bytes).map_err(|_| RefsError::Corrupt("non-utf8 ref".into()))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(RefsError::Corrupt("empty ref file".into()));
    }
    if let Some(rest) = trimmed.strip_prefix("ref: ") {
        let target = rest.trim();
        if target.is_empty() {
            return Err(RefsError::Corrupt("empty symref target".into()));
        }
        return Ok(RefTarget::Symbolic(target.to_string()));
    }
    let oid = Oid::from_hex(trimmed).map_err(|e| RefsError::Corrupt(format!("bad oid: {e}")))?;
    Ok(RefTarget::Direct(oid))
}

/// Read loose ref at `path` if present.
pub fn read_loose_at(path: &Path) -> Result<Option<RefTarget>, RefsError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(parse_ref_contents(&bytes)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(RefsError::Io(e.to_string())),
    }
}

pub fn format_direct(oid: &Oid) -> String {
    format!("{}\n", oid.to_hex())
}

pub fn format_symbolic(target: &str) -> String {
    format!("ref: {target}\n")
}

#[cfg(test)]
mod loose_tests {
    use super::*;
    use arks_oid::Oid;

    #[test]
    fn bare_oid_vs_symref() {
        let o = Oid::from_hex("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").unwrap();
        assert_eq!(
            parse_ref_contents(o.to_hex().as_bytes()).unwrap(),
            RefTarget::Direct(o)
        );
        assert_eq!(
            parse_ref_contents(b"ref: refs/heads/main\n").unwrap(),
            RefTarget::Symbolic("refs/heads/main".into())
        );
    }
}
