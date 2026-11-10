#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::{Index, IndexEntry, IndexError, cmp_entries, entries_sorted, sort_entries};
    use arks_oid::{Oid, OidKind};
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn empty_blob() -> Oid {
        Oid::from_hex("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").unwrap()
    }

    fn sample_entry(path: &str, stage: u16) -> IndexEntry {
        IndexEntry {
            ctime_sec: 1,
            ctime_nsec: 2,
            mtime_sec: 3,
            mtime_nsec: 4,
            dev: 5,
            ino: 6,
            mode: 0o100644,
            uid: 7,
            gid: 8,
            size: 9,
            oid: empty_blob(),
            flags: stage << 12,
            flags_extended: None,
            path: path.into(),
        }
    }

    #[test]
    fn roundtrip_empty_v2() {
        let idx = Index {
            version: 2,
            entries: vec![],
            extensions: vec![],
        };
        let bytes = idx.write(OidKind::Sha1).unwrap();
        let back = Index::read(&bytes, OidKind::Sha1).unwrap();
        assert_eq!(back.version, 2);
        assert!(back.entries.is_empty());
    }

    #[test]
    fn roundtrip_single_and_multi_sorted() {
        let mut e = vec![
            sample_entry("a.txt", 0),
            sample_entry("b.txt", 0),
            sample_entry("b.txt", 1),
        ];
        sort_entries(&mut e);
        let idx = Index {
            version: 2,
            entries: e,
            extensions: vec![],
        };
        let bytes = idx.write(OidKind::Sha1).unwrap();
        let back = Index::read(&bytes, OidKind::Sha1).unwrap();
        assert_eq!(back.entries.len(), 3);
        assert_eq!(back.entries[1].path, "b.txt");
        assert_eq!(back.entries[1].stage(), 0);
        assert_eq!(back.entries[2].stage(), 1);
        assert!(entries_sorted(&back.entries));
    }

    #[test]
    fn checksum_mismatch() {
        let idx = Index {
            version: 2,
            entries: vec![sample_entry("f", 0)],
            extensions: vec![],
        };
        let mut bytes = idx.write(OidKind::Sha1).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        assert_eq!(
            Index::read(&bytes, OidKind::Sha1),
            Err(IndexError::Checksum)
        );
    }

    #[test]
    fn truncated_and_bad_signature() {
        assert_eq!(
            Index::read(b"DIRC", OidKind::Sha1),
            Err(IndexError::Truncated)
        );
        assert_eq!(
            Index::read(b"XXXX\x00\x00\x00\x02\x00\x00\x00\x00", OidKind::Sha1),
            Err(IndexError::BadSignature)
        );
        let idx = Index {
            version: 2,
            entries: vec![],
            extensions: vec![],
        };
        let mut bytes = idx.write(OidKind::Sha1).unwrap();
        bytes.truncate(bytes.len() - 5);
        assert_eq!(
            Index::read(&bytes, OidKind::Sha1),
            Err(IndexError::Truncated)
        );
    }

    #[test]
    fn unsorted_write_rejected() {
        let idx = Index {
            version: 2,
            entries: vec![sample_entry("z", 0), sample_entry("a", 0)],
            extensions: vec![],
        };
        assert_eq!(idx.write(OidKind::Sha1), Err(IndexError::UnsortedEntries));
    }

    #[test]
    fn path_nul_rejected_on_write() {
        let mut e = sample_entry("ok", 0);
        e.path = "bad\0name".into();
        let idx = Index {
            version: 2,
            entries: vec![e],
            extensions: vec![],
        };
        assert_eq!(idx.write(OidKind::Sha1), Err(IndexError::PathNul));
    }

    #[test]
    fn stage_bits_roundtrip() {
        let e = sample_entry("x", 2);
        let mut entries = vec![e];
        sort_entries(&mut entries);
        let idx = Index {
            version: 2,
            entries,
            extensions: vec![],
        };
        let back = Index::read(&idx.write(OidKind::Sha1).unwrap(), OidKind::Sha1).unwrap();
        assert_eq!(back.entries[0].stage(), 2);
    }

    #[test]
    fn write_path_uses_lock() {
        let dir = TempDir::new().unwrap();
        let index_path = dir.path().join("index");
        let idx = Index {
            version: 2,
            entries: vec![sample_entry("locked", 0)],
            extensions: vec![],
        };
        idx.write_path(&index_path, OidKind::Sha1).unwrap();
        assert!(index_path.exists());
        assert!(!dir.path().join("index.lock").exists());
        let back = Index::read_path(&index_path, OidKind::Sha1).unwrap();
        assert_eq!(back.entries[0].path, "locked");
    }

    #[test]
    fn write_path_refuses_existing_lock() {
        let dir = TempDir::new().unwrap();
        let index_path = dir.path().join("index");
        let lock = crate::index_lock_path(&index_path);
        fs::write(&lock, b"999999\n").unwrap();
        let idx = Index {
            version: 2,
            entries: vec![sample_entry("x", 0)],
            extensions: vec![],
        };
        match idx.write_path(&index_path, OidKind::Sha1) {
            Err(IndexError::LockFailed(p)) => {
                assert!(p.contains("index.lock"), "{p}");
            }
            other => panic!("expected LockFailed, got {other:?}"),
        }
        // Must not skip the lock (lease ≠ skip Git locks).
        assert!(!index_path.exists());
        assert!(lock.exists());
    }

    #[test]
    fn memcmp_path_order() {
        let a = sample_entry("B", 0);
        let b = sample_entry("a", 0);
        assert_eq!(cmp_entries(&a, &b), std::cmp::Ordering::Less);
    }

    #[test]
    fn unsupported_version() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"DIRC");
        raw.extend_from_slice(&1u32.to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        let mut h = arks_hash::Hasher::new(arks_hash::ObjectFormat::Sha1);
        h.update(&raw);
        raw.extend_from_slice(h.finalize().as_bytes());
        assert_eq!(
            Index::read(&raw, OidKind::Sha1),
            Err(IndexError::UnsupportedVersion(1))
        );
    }

    #[test]
    fn stock_git_advisory_roundtrip() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let dir = TempDir::new().unwrap();
        let status = Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir.path())
            .status()
            .expect("git init");
        assert!(status.success());
        fs::write(dir.path().join("hello.txt"), b"hello\n").unwrap();
        let status = Command::new("git")
            .args(["add", "hello.txt"])
            .current_dir(dir.path())
            .status()
            .expect("git add");
        assert!(status.success());
        let index_path = dir.path().join(".git").join("index");
        let parsed = Index::read_path(&index_path, OidKind::Sha1).unwrap();
        assert!(
            parsed
                .entries
                .iter()
                .any(|e| e.path == "hello.txt" && e.stage() == 0)
        );

        let rewrite = Index {
            version: 2,
            entries: parsed.entries.clone(),
            extensions: vec![],
        };
        rewrite.write_path(&index_path, OidKind::Sha1).unwrap();
        let out = Command::new("git")
            .args(["ls-files", "--stage"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("hello.txt"));
    }
}
