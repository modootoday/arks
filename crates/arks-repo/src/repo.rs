//! Composed repository handle (store + refs).

use crate::discover::{discover, resolve_common_dir, GitDirKind};
use crate::RepoError;
use arks_object::RawObject;
use arks_oid::Oid;
use arks_refs::{FilesystemRefStore, RefTarget};
use arks_store::{FilesystemGitStore, ObjectStore};
use std::fs;
use std::path::{Path, PathBuf};

/// Open repository: filesystem object store + ref store under `$GIT_DIR` /
/// `$GIT_COMMON_DIR` (linked worktrees).
#[derive(Debug)]
pub struct Repository {
    git_dir: PathBuf,
    common_dir: PathBuf,
    work_tree: Option<PathBuf>,
    kind: GitDirKind,
    store: FilesystemGitStore,
    refs: FilesystemRefStore,
}

impl Repository {
    /// Discover from `start` and open store + refs.
    pub fn discover(start: impl AsRef<Path>) -> Result<Self, RepoError> {
        let d = discover(start)?;
        // Prefer discovery-walk worktree (covers absorbed submodule `.git` files
        // that lack a linked-worktree `gitdir` reverse pointer). Fall back to
        // inferring from `$GIT_DIR/gitdir` for open-by-git-dir callers.
        let work_tree = match d.work_tree {
            Some(wt) => Some(wt),
            None => infer_work_tree(&d.git_dir, d.kind)?,
        };
        Self::open_git_dir(d.git_dir, d.kind, work_tree)
    }

    /// Open a known `$GIT_DIR` (must look like a Git directory).
    pub fn open(git_dir: impl AsRef<Path>) -> Result<Self, RepoError> {
        let git_dir = git_dir.as_ref().to_path_buf();
        let kind = if git_dir
            .file_name()
            .is_some_and(|n| n == std::ffi::OsStr::new(".git"))
        {
            GitDirKind::DotGitDir
        } else if git_dir.join("commondir").is_file() {
            GitDirKind::DotGitFile
        } else {
            GitDirKind::Bare
        };
        let work_tree = infer_work_tree(&git_dir, kind)?;
        Self::open_git_dir(git_dir, kind, work_tree)
    }

    fn open_git_dir(
        git_dir: PathBuf,
        kind: GitDirKind,
        work_tree: Option<PathBuf>,
    ) -> Result<Self, RepoError> {
        if !git_dir.join("HEAD").is_file() {
            return Err(RepoError::NotFound(git_dir.display().to_string()));
        }
        let common_dir = resolve_common_dir(&git_dir)?;
        if !common_dir.join("objects").is_dir() {
            return Err(RepoError::NotFound(format!(
                "objects missing under common dir {}",
                common_dir.display()
            )));
        }
        let store = FilesystemGitStore::open(common_dir.join("objects"));
        let refs = if common_dir == git_dir {
            FilesystemRefStore::open(&git_dir)?
        } else {
            FilesystemRefStore::open_with_common(&git_dir, Some(&common_dir))?
        };
        Ok(Self {
            git_dir,
            common_dir,
            work_tree,
            kind,
            store,
            refs,
        })
    }

    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// `$GIT_COMMON_DIR` (equals `git_dir` for the main worktree).
    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    /// Working tree path when known (None for bare).
    pub fn work_tree(&self) -> Option<&Path> {
        self.work_tree.as_deref()
    }

    pub fn kind(&self) -> GitDirKind {
        self.kind
    }

    /// Path to `$GIT_DIR/index` (callers use `arks-index`; this crate does not decode).
    pub fn index_path(&self) -> PathBuf {
        self.git_dir.join("index")
    }

    pub fn store(&self) -> &FilesystemGitStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut FilesystemGitStore {
        &mut self.store
    }

    pub fn refs(&self) -> &FilesystemRefStore {
        &self.refs
    }

    pub fn refs_mut(&mut self) -> &mut FilesystemRefStore {
        &mut self.refs
    }

    /// Resolve `HEAD` to a direct OID (follows symrefs).
    pub fn head_oid(&self) -> Result<Oid, RepoError> {
        Ok(self.refs.resolve("HEAD")?)
    }

    /// One-hop read of `HEAD`.
    pub fn head_target(&self) -> Result<RefTarget, RepoError> {
        Ok(self.refs.read("HEAD")?)
    }

    /// Load an object through the store facade.
    pub fn get_object(&self, oid: &Oid) -> Result<RawObject, RepoError> {
        Ok(self.store.get(oid)?)
    }
}

fn infer_work_tree(git_dir: &Path, kind: GitDirKind) -> Result<Option<PathBuf>, RepoError> {
    match kind {
        GitDirKind::DotGitDir => Ok(git_dir.parent().map(|p| p.to_path_buf())),
        GitDirKind::Bare => Ok(None),
        GitDirKind::DotGitFile => {
            // Linked worktree: `$GIT_DIR/gitdir` points at the worktree `.git` file.
            let gitdir_file = git_dir.join("gitdir");
            if gitdir_file.is_file() {
                let text =
                    fs::read_to_string(&gitdir_file).map_err(|e| RepoError::Io(e.to_string()))?;
                let line = text
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .ok_or_else(|| RepoError::InvalidGitFile("empty gitdir pointer".into()))?;
                let path = PathBuf::from(line);
                let abs = if path.is_absolute() {
                    path
                } else {
                    git_dir.join(path)
                };
                let abs = fs::canonicalize(&abs).unwrap_or(abs);
                return Ok(abs.parent().map(|p| p.to_path_buf()));
            }
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arks_object::ObjectKind;
    use arks_refs::RefTarget;
    use arks_store::ObjectFormat;
    use tempfile::TempDir;

    fn oid(n: u8) -> Oid {
        Oid::from_hex(&format!("{n:040x}")).unwrap()
    }

    fn seed_worktree(root: &Path) -> PathBuf {
        let git_dir = root.join(".git");
        std::fs::create_dir_all(git_dir.join("objects")).unwrap();
        std::fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
        std::fs::write(git_dir.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        git_dir
    }

    #[test]
    fn compose_head_resolve_and_object() {
        let tmp = TempDir::new().unwrap();
        let git_dir = seed_worktree(tmp.path());
        let mut repo = Repository::discover(tmp.path()).unwrap();
        assert_eq!(repo.kind(), GitDirKind::DotGitDir);
        assert_eq!(repo.common_dir(), repo.git_dir());
        assert_eq!(
            fs::canonicalize(repo.work_tree().unwrap()).unwrap(),
            fs::canonicalize(tmp.path()).unwrap()
        );
        assert!(repo
            .index_path()
            .ends_with(std::path::Path::new(".git").join("index")));
        let _ = git_dir;

        let tip = oid(7);
        repo.refs_mut()
            .update("refs/heads/main", &tip, None)
            .unwrap();
        assert_eq!(
            repo.head_target().unwrap(),
            RefTarget::Symbolic("refs/heads/main".into())
        );
        assert_eq!(repo.head_oid().unwrap(), tip);

        let obj = RawObject::new(ObjectKind::Blob, b"repo-compose");
        let stored = repo
            .store_mut()
            .put(ObjectFormat::Sha1, &obj)
            .unwrap();
        let got = repo.get_object(&stored).unwrap();
        assert_eq!(got.payload, b"repo-compose");
    }

    #[test]
    fn open_known_git_dir() {
        let tmp = TempDir::new().unwrap();
        let git_dir = seed_worktree(tmp.path());
        let repo = Repository::open(&git_dir).unwrap();
        assert_eq!(repo.git_dir(), git_dir);
    }

    #[test]
    fn linked_worktree_uses_commondir_objects() {
        let tmp = TempDir::new().unwrap();
        let main = tmp.path().join("main");
        fs::create_dir_all(&main).unwrap();
        let common = seed_worktree(&main);
        let tip = oid(3);
        {
            let mut repo = Repository::open(&common).unwrap();
            repo.refs_mut()
                .update("refs/heads/main", &tip, None)
                .unwrap();
        }

        let linked = tmp.path().join("linked");
        fs::create_dir_all(&linked).unwrap();
        let admin = common.join("worktrees/linked");
        fs::create_dir_all(&admin).unwrap();
        fs::write(admin.join("commondir"), b"../..\n").unwrap();
        let gitfile = linked.join(".git");
        let admin_abs = fs::canonicalize(&admin).unwrap_or(admin.clone());
        fs::write(
            &gitfile,
            format!("gitdir: {}\n", admin_abs.display()),
        )
        .unwrap();
        fs::write(
            admin.join("gitdir"),
            format!("{}\n", fs::canonicalize(&gitfile).unwrap_or(gitfile).display()),
        )
        .unwrap();
        fs::write(admin.join("HEAD"), b"ref: refs/heads/feature\n").unwrap();
        {
            let mut main_repo = Repository::open(&common).unwrap();
            main_repo
                .refs_mut()
                .update("refs/heads/feature", &tip, None)
                .unwrap();
        }

        let repo = Repository::discover(&linked).unwrap();
        assert_eq!(repo.kind(), GitDirKind::DotGitFile);
        assert_eq!(
            fs::canonicalize(repo.common_dir()).unwrap(),
            fs::canonicalize(&common).unwrap()
        );
        assert_eq!(repo.head_oid().unwrap(), tip);
        assert_eq!(
            fs::canonicalize(repo.work_tree().unwrap()).unwrap(),
            fs::canonicalize(&linked).unwrap()
        );
    }
}
