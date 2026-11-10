//! Tree object payload encode/decode (claimed subset for M5).
//!
//! Entry form (git accepted encoding; see [[research:root/20261108071913]]):
//! `mode SP name NUL` + raw OID bytes. Directory mode is ASCII `40000` (no
//! leading zero). Entries sorted with directories compared as if name ended
//! with `/` (`base_name_compare`).

use crate::ObjectError;
use arks_oid::{Oid, OidKind};

/// Known tree entry modes written as octal ASCII (no leading zero for trees).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TreeMode {
    /// Regular non-executable file (`100644`).
    File,
    /// Executable file (`100755`).
    Executable,
    /// Symbolic link (`120000`).
    Symlink,
    /// Directory / nested tree (`40000` on disk).
    Tree,
    /// Gitlink / submodule (`160000`).
    Gitlink,
}

impl TreeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "100644",
            Self::Executable => "100755",
            Self::Symlink => "120000",
            Self::Tree => "40000",
            Self::Gitlink => "160000",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ObjectError> {
        match s {
            "100644" | "644" => Ok(Self::File),
            "100755" | "755" => Ok(Self::Executable),
            "120000" => Ok(Self::Symlink),
            "40000" | "040000" => Ok(Self::Tree),
            "160000" => Ok(Self::Gitlink),
            other => Err(ObjectError::BadTreeMode(other.to_string())),
        }
    }

    pub fn is_tree(self) -> bool {
        matches!(self, Self::Tree)
    }

    /// Index / `stat` mode bits commonly stored in the index for this kind.
    pub fn from_index_mode(mode: u32) -> Result<Self, ObjectError> {
        // Git index stores `0o100644` etc.; trees are not index entries.
        match mode & 0o170000 {
            0o100000 => {
                if mode & 0o111 != 0 {
                    Ok(Self::Executable)
                } else {
                    Ok(Self::File)
                }
            }
            0o120000 => Ok(Self::Symlink),
            0o160000 => Ok(Self::Gitlink),
            0o040000 => Ok(Self::Tree),
            _ => Err(ObjectError::BadTreeMode(format!("index mode {mode:#o}"))),
        }
    }
}

/// One tree entry (name is a single path component; no `/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub mode: TreeMode,
    pub name: String,
    pub oid: Oid,
}

/// Parsed tree payload.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new(mut entries: Vec<TreeEntry>) -> Result<Self, ObjectError> {
        sort_tree_entries(&mut entries);
        validate_entries(&entries)?;
        Ok(Self { entries })
    }

    pub fn encode(&self) -> Result<Vec<u8>, ObjectError> {
        validate_entries(&self.entries)?;
        let mut out = Vec::new();
        for e in &self.entries {
            out.extend_from_slice(e.mode.as_str().as_bytes());
            out.push(b' ');
            out.extend_from_slice(e.name.as_bytes());
            out.push(0);
            out.extend_from_slice(e.oid.as_bytes());
        }
        Ok(out)
    }

    /// Decode a tree payload. `hash_kind` selects OID width (M5 claims SHA-1).
    pub fn decode(bytes: &[u8], hash_kind: OidKind) -> Result<Self, ObjectError> {
        let oid_len = hash_kind.digest_len();
        let mut entries = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let mode_start = i;
            while i < bytes.len() && bytes[i] != b' ' {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(ObjectError::BadTreeEntry("truncated mode".into()));
            }
            let mode_str = std::str::from_utf8(&bytes[mode_start..i])
                .map_err(|_| ObjectError::BadTreeEntry("mode utf-8".into()))?;
            let mode = TreeMode::parse(mode_str)?;
            i += 1; // space
            let name_start = i;
            while i < bytes.len() && bytes[i] != 0 {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(ObjectError::BadTreeEntry("truncated name".into()));
            }
            let name = std::str::from_utf8(&bytes[name_start..i])
                .map_err(|_| ObjectError::BadTreeEntry("name utf-8".into()))?
                .to_owned();
            i += 1; // NUL
            if bytes.len() < i + oid_len {
                return Err(ObjectError::BadTreeEntry("truncated oid".into()));
            }
            let oid = Oid::from_bytes(hash_kind, &bytes[i..i + oid_len]).map_err(ObjectError::Oid)?;
            i += oid_len;
            entries.push(TreeEntry { mode, name, oid });
        }
        if !tree_entries_sorted(&entries) {
            return Err(ObjectError::UnsortedTree);
        }
        validate_entries(&entries)?;
        Ok(Self { entries })
    }
}

fn validate_entries(entries: &[TreeEntry]) -> Result<(), ObjectError> {
    for e in entries {
        if e.name.is_empty() {
            return Err(ObjectError::BadTreeEntry("empty name".into()));
        }
        if e.name.contains('/') || e.name.contains('\0') {
            return Err(ObjectError::BadTreeEntry(format!("bad name {}", e.name)));
        }
        if e.name == "." || e.name == ".." || e.name.eq_ignore_ascii_case(".git") {
            return Err(ObjectError::BadTreeEntry(format!("forbidden name {}", e.name)));
        }
    }
    Ok(())
}

/// Sort key: name bytes, with directories treated as if followed by `/`.
pub fn tree_sort_key(mode: TreeMode, name: &str) -> Vec<u8> {
    let mut key = name.as_bytes().to_vec();
    if mode.is_tree() {
        key.push(b'/');
    }
    key
}

pub fn cmp_tree_entries(a: &TreeEntry, b: &TreeEntry) -> std::cmp::Ordering {
    tree_sort_key(a.mode, &a.name).cmp(&tree_sort_key(b.mode, &b.name))
}

pub fn sort_tree_entries(entries: &mut [TreeEntry]) {
    entries.sort_by(cmp_tree_entries);
}

pub fn tree_entries_sorted(entries: &[TreeEntry]) -> bool {
    entries
        .windows(2)
        .all(|w| cmp_tree_entries(&w[0], &w[1]) != std::cmp::Ordering::Greater)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arks_hash::ObjectFormat;
    use crate::{ObjectKind, RawObject};

    fn oid(n: u8) -> Oid {
        Oid::from_hex(&format!("{n:040x}")).unwrap()
    }

    #[test]
    fn encode_decode_single_file() {
        let tree = Tree::new(vec![TreeEntry {
            mode: TreeMode::File,
            name: "hello.txt".into(),
            oid: oid(0x3b),
        }])
        .unwrap();
        let bytes = tree.encode().unwrap();
        assert_eq!(Tree::decode(&bytes, OidKind::Sha1).unwrap(), tree);
    }

    #[test]
    fn directory_mode_is_40000_not_040000() {
        let tree = Tree::new(vec![TreeEntry {
            mode: TreeMode::Tree,
            name: "src".into(),
            oid: oid(1),
        }])
        .unwrap();
        let bytes = tree.encode().unwrap();
        assert!(bytes.starts_with(b"40000 src\0"));
        assert!(!bytes.starts_with(b"040000"));
    }

    #[test]
    fn sort_directory_as_trailing_slash() {
        // Order: foo-bar, foo.txt, foo (dir), foo0
        let mut entries = vec![
            TreeEntry {
                mode: TreeMode::Tree,
                name: "foo".into(),
                oid: oid(1),
            },
            TreeEntry {
                mode: TreeMode::File,
                name: "foo0".into(),
                oid: oid(2),
            },
            TreeEntry {
                mode: TreeMode::File,
                name: "foo-bar".into(),
                oid: oid(3),
            },
            TreeEntry {
                mode: TreeMode::File,
                name: "foo.txt".into(),
                oid: oid(4),
            },
        ];
        sort_tree_entries(&mut entries);
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["foo-bar", "foo.txt", "foo", "foo0"]);
    }

    #[test]
    fn known_hello_txt_tree_oid() {
        // From public docs: hello.txt → blob 3b18e512dba79e4c8300dd08aeb37f8e728b8dad
        // tree OID 68aba62e560c0ebc3396e8ae9335232cd93a3f60
        let blob = Oid::from_hex("3b18e512dba79e4c8300dd08aeb37f8e728b8dad").unwrap();
        let tree = Tree::new(vec![TreeEntry {
            mode: TreeMode::File,
            name: "hello.txt".into(),
            oid: blob,
        }])
        .unwrap();
        let obj = RawObject::new(ObjectKind::Tree, tree.encode().unwrap());
        assert_eq!(
            obj.oid(ObjectFormat::Sha1).unwrap().to_hex(),
            "68aba62e560c0ebc3396e8ae9335232cd93a3f60"
        );
    }
}
