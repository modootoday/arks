//! Filesystem ref store under a Git directory.

use crate::RefsError;
use crate::lock::{delete_ref_locked, write_ref_locked};
use crate::loose::{format_direct, format_symbolic, read_loose_at, ref_file_path};
use crate::packed::read_packed_refs;
use crate::refname::{is_valid_refname, is_valid_refname_allow_onelevel};
use crate::target::RefTarget;
use arks_oid::Oid;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const RESOLVE_MAX: usize = 32;

/// Refs rooted at `git_dir` (`HEAD`, `refs/`, `packed-refs`).
///
/// When `common_dir` is set (linked worktree / `$GIT_COMMON_DIR`), per-worktree
/// refs (`HEAD`, bisect/worktree rewritten namespaces) stay under `git_dir` while
/// shared refs and `packed-refs` live under `common_dir` — matching
/// gitrepository-layout.
#[derive(Debug, Clone)]
pub struct FilesystemRefStore {
    git_dir: PathBuf,
    common_dir: Option<PathBuf>,
}

impl FilesystemRefStore {
    pub fn open(git_dir: impl AsRef<Path>) -> Result<Self, RefsError> {
        Self::open_with_common(git_dir, None::<PathBuf>)
    }

    /// Open with an optional common directory for shared refs.
    pub fn open_with_common(
        git_dir: impl AsRef<Path>,
        common_dir: Option<impl AsRef<Path>>,
    ) -> Result<Self, RefsError> {
        let git_dir = git_dir.as_ref().to_path_buf();
        if !git_dir.is_dir() {
            return Err(RefsError::Io(format!(
                "git_dir is not a directory: {}",
                git_dir.display()
            )));
        }
        let common_dir = match common_dir {
            Some(p) => {
                let p = p.as_ref().to_path_buf();
                if !p.is_dir() {
                    return Err(RefsError::Io(format!(
                        "common_dir is not a directory: {}",
                        p.display()
                    )));
                }
                if p == git_dir {
                    None
                } else {
                    Some(p)
                }
            }
            None => None,
        };
        Ok(Self {
            git_dir,
            common_dir,
        })
    }

    fn require_valid(name: &str) -> Result<(), RefsError> {
        let ok = if name == "HEAD" || name.contains('/') {
            is_valid_refname(name)
        } else {
            is_valid_refname_allow_onelevel(name)
        };
        if !ok {
            return Err(RefsError::InvalidRefname(name.to_string()));
        }
        Ok(())
    }

    fn is_per_worktree_ref(name: &str) -> bool {
        matches!(
            name,
            "HEAD"
                | "FETCH_HEAD"
                | "ORIG_HEAD"
                | "MERGE_HEAD"
                | "CHERRY_PICK_HEAD"
                | "REVERT_HEAD"
                | "BISECT_LOG"
                | "AUTO_MERGE"
        ) || name.starts_with("refs/bisect/")
            || name.starts_with("refs/worktree/")
            || name.starts_with("refs/rewritten/")
    }

    fn shared_root(&self) -> &Path {
        self.common_dir.as_deref().unwrap_or(&self.git_dir)
    }

    fn ref_root_for(&self, name: &str) -> &Path {
        if Self::is_per_worktree_ref(name) {
            &self.git_dir
        } else {
            self.shared_root()
        }
    }

    fn ref_path(&self, name: &str) -> PathBuf {
        ref_file_path(self.ref_root_for(name), name)
    }

    fn packed_path(&self) -> PathBuf {
        self.shared_root().join("packed-refs")
    }

    fn read_packed_map(&self) -> Result<HashMap<String, Oid>, RefsError> {
        let path = self.packed_path();
        if path.is_file() {
            read_packed_refs(&path)
        } else {
            Ok(HashMap::new())
        }
    }

    /// Read one hop (does not follow symbolic refs).
    pub fn read(&self, name: &str) -> Result<RefTarget, RefsError> {
        Self::require_valid(name)?;
        let path = self.ref_path(name);
        if let Some(t) = read_loose_at(&path)? {
            return Ok(t);
        }
        let packed = self.read_packed_map()?;
        if let Some(oid) = packed.get(name) {
            return Ok(RefTarget::Direct(*oid));
        }
        Err(RefsError::NotFound(name.to_string()))
    }

    /// Follow symbolic refs to a direct OID.
    pub fn resolve(&self, name: &str) -> Result<Oid, RefsError> {
        let mut current = name.to_string();
        for depth in 0..RESOLVE_MAX {
            match self.read(&current)? {
                RefTarget::Direct(oid) => return Ok(oid),
                RefTarget::Symbolic(next) => {
                    if depth + 1 >= RESOLVE_MAX {
                        return Err(RefsError::SymbolicLoop(current));
                    }
                    current = next;
                }
            }
        }
        Err(RefsError::SymbolicLoop(name.to_string()))
    }

    fn check_old(actual: &RefTarget, old: Option<&Oid>) -> Result<(), RefsError> {
        let Some(expected) = old else {
            return Ok(());
        };
        match actual {
            RefTarget::Direct(oid) if oid == expected => Ok(()),
            RefTarget::Direct(oid) => Err(RefsError::UpdateRejected {
                expected: expected.to_hex(),
                actual: oid.to_hex(),
            }),
            RefTarget::Symbolic(sym) => Err(RefsError::UpdateRejected {
                expected: expected.to_hex(),
                actual: format!("ref: {sym}"),
            }),
        }
    }

    pub fn update(&mut self, name: &str, new: &Oid, old: Option<&Oid>) -> Result<(), RefsError> {
        Self::require_valid(name)?;
        let path = self.ref_path(name);
        // Loose or packed may supply the current value (loose wins in `read`).
        match self.read(name) {
            Ok(current) => Self::check_old(&current, old)?,
            Err(RefsError::NotFound(_)) => {
                if let Some(expected) = old {
                    return Err(RefsError::UpdateRejected {
                        expected: expected.to_hex(),
                        actual: "(missing)".into(),
                    });
                }
            }
            Err(e) => return Err(e),
        }
        // Write loose ref (overrides packed for the same name). No packed-refs rewrite in M3.
        let payload = format_direct(new);
        write_ref_locked(&path, payload.as_bytes())
    }

    pub fn delete(&mut self, name: &str, old: Option<&Oid>) -> Result<(), RefsError> {
        Self::require_valid(name)?;
        let path = self.ref_path(name);
        match self.read(name) {
            Ok(current) => {
                Self::check_old(&current, old)?;
                if path.exists() {
                    // Loose delete; packed entry may remain until pack-refs rewrite (out of M3).
                    delete_ref_locked(&path)
                } else {
                    // Packed-only: cannot delete without packed-refs write (M3 out of scope).
                    Err(RefsError::Io(format!(
                        "cannot delete packed-only ref without packed-refs rewrite: {name}"
                    )))
                }
            }
            Err(RefsError::NotFound(_)) => {
                if old.is_some() {
                    Err(RefsError::NotFound(name.to_string()))
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn set_symbolic(&mut self, name: &str, target: &str) -> Result<(), RefsError> {
        Self::require_valid(name)?;
        if !is_valid_refname(target) && target != "HEAD" {
            return Err(RefsError::InvalidRefname(target.to_string()));
        }
        let path = self.ref_path(name);
        let payload = format_symbolic(target);
        write_ref_locked(&path, payload.as_bytes())
    }

    fn list_loose_under(
        &self,
        prefix: &str,
        base: &Path,
        rel: &str,
    ) -> Result<Vec<(String, Oid)>, RefsError> {
        let mut out = Vec::new();
        let read_dir = match std::fs::read_dir(base) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(RefsError::Io(e.to_string())),
        };
        for entry in read_dir {
            let entry = entry.map_err(|e| RefsError::Io(e.to_string()))?;
            let ft = entry
                .file_type()
                .map_err(|e| RefsError::Io(e.to_string()))?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if name.ends_with(".lock") {
                continue;
            }
            let full = if rel.is_empty() {
                name.to_string()
            } else {
                format!("{rel}/{name}")
            };
            if ft.is_dir() {
                out.extend(self.list_loose_under(prefix, &entry.path(), &full)?);
            } else if ft.is_file() {
                if !full.starts_with(prefix) {
                    continue;
                }
                let oid = match parse_loose_file(&entry.path())? {
                    RefTarget::Direct(oid) => oid,
                    RefTarget::Symbolic(_) => self.resolve(&full)?,
                };
                out.push((full, oid));
            }
        }
        Ok(out)
    }

    /// List resolved direct OIDs for refs matching `prefix` (loose overrides packed).
    pub fn list_refs(&self, prefix: &str) -> Result<Vec<(String, Oid)>, RefsError> {
        let mut map: HashMap<String, Oid> = self.read_packed_map()?;
        let shared_refs = self.shared_root().join("refs");
        if shared_refs.is_dir() {
            for (name, oid) in self.list_loose_under(prefix, &shared_refs, "refs")? {
                map.insert(name, oid);
            }
        }
        // Per-worktree ref namespaces under private git_dir (when linked).
        if self.common_dir.is_some() {
            let private_refs = self.git_dir.join("refs");
            if private_refs.is_dir() {
                for (name, oid) in self.list_loose_under(prefix, &private_refs, "refs")? {
                    if Self::is_per_worktree_ref(&name) {
                        map.insert(name, oid);
                    }
                }
            }
        }
        let head_path = self.git_dir.join("HEAD");
        if (prefix.is_empty() || "HEAD".starts_with(prefix)) && head_path.is_file() {
            if let Ok(oid) = self.resolve("HEAD") {
                map.insert("HEAD".into(), oid);
            }
        }
        let mut pairs: Vec<_> = map
            .into_iter()
            .filter(|(name, _)| name.starts_with(prefix))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(pairs)
    }

    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    pub fn common_dir(&self) -> &Path {
        self.shared_root()
    }

    /// Pack all `refs/**` into `packed-refs` and remove matching loose files.
    ///
    /// Cite: <https://git-scm.com/docs/git-pack-refs> (`--all`). HEAD is not packed.
    pub fn pack_refs_all(&mut self) -> Result<usize, RefsError> {
        let mut refs = self.list_refs("refs/")?;
        refs.retain(|(name, _)| name.starts_with("refs/"));
        refs.sort_by(|a, b| a.0.cmp(&b.0));
        let mut body = String::from("# pack-refs with: peeled\n");
        for (name, oid) in &refs {
            body.push_str(&format!("{} {}\n", oid.to_hex(), name));
        }
        let packed = self.packed_path();
        write_ref_locked(&packed, body.as_bytes())?;
        // Remove loose copies under shared (and per-worktree) refs trees.
        for (name, _) in &refs {
            let path = self.ref_path(name);
            if path.is_file() {
                let _ = delete_ref_locked(&path);
            }
        }
        Ok(refs.len())
    }
}

fn parse_loose_file(path: &Path) -> Result<RefTarget, RefsError> {
    let bytes = std::fs::read(path).map_err(|e| RefsError::Io(e.to_string()))?;
    crate::loose::parse_ref_contents(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn oid(n: u8) -> Oid {
        let hex = format!("{n:040x}");
        Oid::from_hex(&hex).unwrap()
    }

    fn init_repo(dir: &Path) -> FilesystemRefStore {
        std::fs::create_dir_all(dir.join("refs/heads")).unwrap();
        FilesystemRefStore::open(dir).unwrap()
    }

    #[test]
    fn update_read_delete_with_lock() {
        let tmp = TempDir::new().unwrap();
        let mut store = init_repo(tmp.path());
        let name = "refs/heads/main";
        let o1 = oid(1);
        let o2 = oid(2);
        store.update(name, &o1, None).unwrap();
        assert_eq!(store.read(name).unwrap(), RefTarget::Direct(o1));
        store.update(name, &o2, Some(&o1)).unwrap();
        assert_eq!(store.resolve(name).unwrap(), o2);
        store.delete(name, Some(&o2)).unwrap();
        assert!(matches!(store.read(name), Err(RefsError::NotFound(_))));
    }

    #[test]
    fn head_symref_resolve() {
        let tmp = TempDir::new().unwrap();
        let mut store = init_repo(tmp.path());
        let target = "refs/heads/main";
        let o = oid(9);
        store.update(target, &o, None).unwrap();
        store.set_symbolic("HEAD", target).unwrap();
        assert_eq!(
            store.read("HEAD").unwrap(),
            RefTarget::Symbolic(target.into())
        );
        assert_eq!(store.resolve("HEAD").unwrap(), o);
    }

    #[test]
    fn loose_wins_over_packed() {
        let tmp = TempDir::new().unwrap();
        let mut store = init_repo(tmp.path());
        let name = "refs/heads/main";
        let loose = oid(3);
        let packed = oid(4);
        let mut packed_file = std::fs::File::create(tmp.path().join("packed-refs")).unwrap();
        writeln!(packed_file, "{packed} {name}").unwrap();
        store.update(name, &loose, None).unwrap();
        assert_eq!(store.resolve(name).unwrap(), loose);
    }

    #[test]
    fn missing_trailing_newline_still_parses() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("refs/heads/x");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, oid(5).to_hex()).unwrap();
        let store = init_repo(tmp.path());
        assert_eq!(store.resolve("refs/heads/x").unwrap(), oid(5));
    }

    #[test]
    fn concurrent_lock_fails() {
        let tmp = TempDir::new().unwrap();
        let mut store = init_repo(tmp.path());
        let name = "refs/heads/main";
        let ref_path = tmp.path().join(name);
        std::fs::create_dir_all(ref_path.parent().unwrap()).unwrap();
        std::fs::write(ref_path.with_extension("lock"), b"").unwrap();
        assert!(matches!(
            store.update(name, &oid(1), None),
            Err(RefsError::LockFailed(_))
        ));
    }

    #[test]
    fn invalid_refname_rejected() {
        let tmp = TempDir::new().unwrap();
        let mut store = init_repo(tmp.path());
        assert!(matches!(
            store.update("refs/heads/../etc", &oid(1), None),
            Err(RefsError::InvalidRefname(_))
        ));
    }

    #[test]
    fn update_rejects_wrong_old() {
        let tmp = TempDir::new().unwrap();
        let mut store = init_repo(tmp.path());
        let name = "refs/heads/main";
        store.update(name, &oid(1), None).unwrap();
        assert!(matches!(
            store.update(name, &oid(2), Some(&oid(9))),
            Err(RefsError::UpdateRejected { .. })
        ));
    }

    #[test]
    fn update_packed_only_honors_old_and_writes_loose() {
        let tmp = TempDir::new().unwrap();
        let mut store = init_repo(tmp.path());
        let name = "refs/heads/main";
        let packed = oid(4);
        let neu = oid(5);
        let mut packed_file = std::fs::File::create(tmp.path().join("packed-refs")).unwrap();
        writeln!(packed_file, "{packed} {name}").unwrap();
        assert!(matches!(
            store.update(name, &neu, Some(&oid(9))),
            Err(RefsError::UpdateRejected { .. })
        ));
        store.update(name, &neu, Some(&packed)).unwrap();
        assert_eq!(store.resolve(name).unwrap(), neu);
        assert!(tmp.path().join(name).is_file());
    }

    #[test]
    fn delete_packed_only_refuses_without_rewrite() {
        let tmp = TempDir::new().unwrap();
        let store_init = init_repo(tmp.path());
        drop(store_init);
        let name = "refs/heads/main";
        let packed = oid(4);
        let mut packed_file = std::fs::File::create(tmp.path().join("packed-refs")).unwrap();
        writeln!(packed_file, "{packed} {name}").unwrap();
        let mut store = FilesystemRefStore::open(tmp.path()).unwrap();
        assert!(store.delete(name, Some(&packed)).is_err());
    }
}
