//! `packed-refs` read-only parser.

use crate::RefsError;
use arks_oid::Oid;
use std::collections::HashMap;
use std::path::Path;

/// Parse `packed-refs`. Peeled lines (`^<oid>`, the object an annotated tag
/// preceding it peels to; gitrepository-layout `packed-refs`, git
/// `refs/packed-backend.c`) are validated and omitted from the map.
pub fn read_packed_refs(path: &Path) -> Result<HashMap<String, Oid>, RefsError> {
    let data = std::fs::read_to_string(path).map_err(|e| RefsError::Io(e.to_string()))?;
    let mut map = HashMap::new();
    for line in data.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(peeled) = line.strip_prefix('^') {
            Oid::from_hex(peeled.trim())
                .map_err(|e| RefsError::Corrupt(format!("packed-refs bad peeled oid: {e}")))?;
            continue;
        }
        // Legacy arks fixture shape `<oid>^{}` (not written by stock git).
        if line.ends_with("^{}") {
            continue;
        }
        let Some((oid_hex, refname)) = line.split_once(char::is_whitespace) else {
            return Err(RefsError::Corrupt(format!(
                "packed-refs line missing refname: {line}"
            )));
        };
        let oid_hex = oid_hex.trim();
        let refname = refname.trim();
        if refname.is_empty() {
            return Err(RefsError::Corrupt("packed-refs empty refname".into()));
        }
        let oid = Oid::from_hex(oid_hex)
            .map_err(|e| RefsError::Corrupt(format!("packed-refs bad oid: {e}")));
        map.insert(refname.to_string(), oid?);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parses_header_and_peel_skipped() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            "# pack-refs with: peeled\n\
             aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa refs/heads/main\n\
             bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb refs/tags/v1\n\
             bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb^{{}}\n"
        )
        .unwrap();
        let map = read_packed_refs(f.path()).unwrap();
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("refs/heads/main"));
    }

    #[test]
    fn stock_git_peeled_lines_are_skipped() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            "# pack-refs with: peeled fully-peeled sorted \n\
             aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa refs/heads/main\n\
             bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb refs/tags/v1\n\
             ^cccccccccccccccccccccccccccccccccccccccc\n"
        )
        .unwrap();
        let map = read_packed_refs(f.path()).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map["refs/tags/v1"].to_hex(), "b".repeat(40));
    }
}
