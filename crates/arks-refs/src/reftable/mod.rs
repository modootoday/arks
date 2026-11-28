//! Git Reftable v1/v2 backend (`arks-refs`, E5).
//!
//! Primary reference: Git `Documentation/technical/reftable.txt`.
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.6 (E5).

pub mod block;
pub mod stack;
pub mod table;
pub mod varint;

pub use block::{LogRecord, LogUpdate, RefRecord, RefValue};
pub use stack::{ReftableStack, StackVerifyReport};
pub use table::{DEFAULT_BLOCK_SIZE, TableFooter, TableHeader, TableReader, write_table};

use crate::RefsError;
use arks_oid::OidKind;
use std::fs;
use std::path::Path;

/// Sentinel content placed in `<git_dir>/HEAD` in reftable repositories so older
/// Git clients fail cleanly instead of corrupting refs (per Git reftable spec).
pub const REFTABLE_HEAD_STUB: &str = "ref: refs/heads/.invalid\n";

/// Sentinel content placed in `<git_dir>/refs/heads` (regular file, not directory)
/// in reftable repositories per Git `Documentation/technical/reftable.txt`.
pub const REFTABLE_REFS_HEADS_STUB: &str = "this repository uses the reftable format\n";

/// Returns `true` if `git_dir` has an initialized `reftable/tables.list`.
pub fn is_reftable_dir(git_dir: &Path) -> bool {
    git_dir.join("reftable").join("tables.list").is_file()
}

/// Write Git-spec backward-compatibility stubs (`HEAD` and `refs/heads` sentinel file).
pub fn write_reftable_stubs(git_dir: &Path) -> Result<(), RefsError> {
    let refs_dir = git_dir.join("refs");
    if refs_dir.exists() {
        fs::remove_dir_all(&refs_dir)
            .map_err(|e| RefsError::Io(format!("remove old refs/: {e}")))?;
    }
    fs::create_dir_all(&refs_dir).map_err(|e| RefsError::Io(format!("create refs/: {e}")))?;
    fs::write(refs_dir.join("heads"), REFTABLE_REFS_HEADS_STUB)
        .map_err(|e| RefsError::Io(format!("write refs/heads stub: {e}")))?;
    fs::write(git_dir.join("HEAD"), REFTABLE_HEAD_STUB)
        .map_err(|e| RefsError::Io(format!("write HEAD stub: {e}")))?;
    let packed = git_dir.join("packed-refs");
    if packed.is_file() {
        let _ = fs::remove_file(packed);
    }
    Ok(())
}

/// Detect the hash format (`Sha1` or `Sha256`) from `<git_dir>/config` (`extensions.objectFormat`).
pub fn detect_oid_kind(git_dir: &Path) -> OidKind {
    let Ok(text) = fs::read_to_string(git_dir.join("config")) else {
        return OidKind::Sha1;
    };
    let mut in_ext = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_ext = line[1..line.len() - 1]
                .trim()
                .eq_ignore_ascii_case("extensions");
            continue;
        }
        if in_ext {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim().eq_ignore_ascii_case("objectformat")
                    && v.trim().trim_matches('"').eq_ignore_ascii_case("sha256")
                {
                    return OidKind::Sha256;
                }
            }
        }
    }
    OidKind::Sha1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::RefTarget;
    use arks_oid::Oid;
    use tempfile::TempDir;

    fn sha1(n: u32) -> Oid {
        Oid::from_hex(&format!("{n:040x}")).unwrap()
    }

    fn sha256(n: u32) -> Oid {
        Oid::from_hex(&format!("{n:064x}")).unwrap()
    }

    #[test]
    fn table_v1_multi_block_index_and_restart_binary_search() {
        // Write 150 refs (>64 per block -> triggers multi-block + 'i' index block + restart tables).
        let mut refs = Vec::new();
        for i in 0..150u32 {
            refs.push(RefRecord {
                refname: format!("refs/heads/branch-{i:04}"),
                update_index: 1,
                value: RefValue::Direct(sha1(i + 1)),
            });
        }
        refs.push(RefRecord {
            refname: "refs/tags/v1.0".into(),
            update_index: 1,
            value: RefValue::Peeled {
                value: sha1(900),
                target: sha1(901),
            },
        });
        refs.sort_by(|a, b| a.refname.cmp(&b.refname));

        let logs = vec![LogRecord {
            refname: "refs/heads/branch-0000".into(),
            update_index: 1,
            update: Some(LogUpdate {
                old_oid: sha1(0),
                new_oid: sha1(1),
                name: "Test User".into(),
                email: "test@example.com".into(),
                time: 1_700_000_000,
                tz_offset: 0,
                message: "initial commit\n".into(),
            }),
        }];

        let bytes = write_table(&refs, &logs, 1, 1, OidKind::Sha1, DEFAULT_BLOCK_SIZE).unwrap();
        let reader = TableReader::from_bytes(bytes.clone()).unwrap();
        assert_eq!(reader.header().version, 1);
        assert!(reader.footer().ref_index_position > 0);
        assert!(reader.footer().log_position > 0);
        reader.verify().unwrap();

        // Seek boundary, middle, and missing refs.
        let r0 = reader.seek_ref("refs/heads/branch-0000").unwrap().unwrap();
        assert_eq!(r0.value, RefValue::Direct(sha1(1)));
        let r99 = reader.seek_ref("refs/heads/branch-0099").unwrap().unwrap();
        assert_eq!(r99.value, RefValue::Direct(sha1(100)));
        let tag = reader.seek_ref("refs/tags/v1.0").unwrap().unwrap();
        assert_eq!(
            tag.value,
            RefValue::Peeled {
                value: sha1(900),
                target: sha1(901)
            }
        );
        assert!(
            reader
                .seek_ref("refs/heads/does-not-exist")
                .unwrap()
                .is_none()
        );

        // Decoded log block matches.
        let dec_logs = reader.all_logs().unwrap();
        assert_eq!(dec_logs, logs);

        // Corrupt a single byte in the footer -> CRC-32 mismatch must fail closed.
        let mut bad = bytes;
        let last = bad.len() - 1;
        bad[last] ^= 0xff;
        assert!(TableReader::from_bytes(bad).is_err());
    }

    #[test]
    fn table_v2_sha256_roundtrip() {
        let refs = vec![
            RefRecord {
                refname: "HEAD".into(),
                update_index: 1,
                value: RefValue::Symbolic("refs/heads/main".into()),
            },
            RefRecord {
                refname: "refs/heads/main".into(),
                update_index: 1,
                value: RefValue::Direct(sha256(42)),
            },
        ];
        let bytes = write_table(&refs, &[], 1, 1, OidKind::Sha256, DEFAULT_BLOCK_SIZE).unwrap();
        let reader = TableReader::from_bytes(bytes).unwrap();
        assert_eq!(reader.header().version, 2);
        assert_eq!(reader.header().oid_kind, OidKind::Sha256);
        assert_eq!(
            reader.seek_ref("HEAD").unwrap().unwrap().value,
            RefValue::Symbolic("refs/heads/main".into())
        );
        assert_eq!(
            reader.seek_ref("refs/heads/main").unwrap().unwrap().value,
            RefValue::Direct(sha256(42))
        );
    }

    #[test]
    fn stack_transactions_tombstones_and_geometric_compaction() {
        let tmp = TempDir::new().unwrap();
        let stack = ReftableStack::open(tmp.path().join("reftable"), OidKind::Sha1);
        stack.init().unwrap();

        // Append 12 consecutive transactions -> geometric factor-2 auto-compaction keeps table count small!
        for i in 1..=12u32 {
            stack
                .append_transaction(
                    vec![
                        ("HEAD".into(), RefValue::Symbolic("refs/heads/main".into())),
                        ("refs/heads/main".into(), RefValue::Direct(sha1(i))),
                        (
                            format!("refs/heads/feat-{i}"),
                            RefValue::Direct(sha1(100 + i)),
                        ),
                    ],
                    vec![],
                )
                .unwrap();
        }
        // Delete feat-3 via tombstone.
        stack
            .append_transaction(
                vec![("refs/heads/feat-3".into(), RefValue::Deletion)],
                vec![],
            )
            .unwrap();

        assert_eq!(
            stack.read_ref("HEAD").unwrap(),
            RefTarget::Symbolic("refs/heads/main".into())
        );
        assert_eq!(
            stack.read_ref("refs/heads/main").unwrap(),
            RefTarget::Direct(sha1(12))
        );
        assert!(matches!(
            stack.read_ref("refs/heads/feat-3"),
            Err(RefsError::NotFound(_))
        ));
        assert_eq!(
            stack.read_ref("refs/heads/feat-12").unwrap(),
            RefTarget::Direct(sha1(112))
        );

        let report = stack.verify().unwrap();
        assert!(
            report.table_count <= 4,
            "geometric compaction should keep stack compact, got {}",
            report.table_count
        );
        assert_eq!(report.max_update_index, 13);

        // Full compaction collapses to 1 table.
        stack.compact_all().unwrap();
        assert_eq!(stack.table_names().unwrap().len(), 1);
        assert_eq!(
            stack.read_ref("refs/heads/main").unwrap(),
            RefTarget::Direct(sha1(12))
        );
    }
}
