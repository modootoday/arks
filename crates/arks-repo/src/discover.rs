//! Locate `$GIT_DIR` from a start path (claimed subset of gitrepository-layout).

use crate::RepoError;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// How a Git directory was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitDirKind {
    /// `.git/` directory under a work tree.
    DotGitDir,
    /// `.git` file containing `gitdir: <path>`.
    DotGitFile,
    /// Directory that itself looks like `$GIT_DIR` (HEAD + objects/).
    Bare,
}

/// Parse a `.git` file body (`gitdir: <path>`). Relative paths resolve against `file_parent`.
pub fn parse_gitdir_file(contents: &str, file_parent: &Path) -> Result<PathBuf, RepoError> {
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix("gitdir:") else {
            return Err(RepoError::InvalidGitFile(format!(
                "expected gitdir: line, got {line}"
            )));
        };
        let raw = rest.trim();
        if raw.is_empty() {
            return Err(RepoError::InvalidGitFile("empty gitdir path".into()));
        }
        let path = PathBuf::from(raw);
        let resolved = if path.is_absolute() {
            path
        } else {
            file_parent.join(path)
        };
        return Ok(fs::canonicalize(&resolved).unwrap_or(resolved));
    }
    Err(RepoError::InvalidGitFile("no gitdir: line".into()))
}

fn looks_like_git_dir(dir: &Path) -> bool {
    if !dir.join("HEAD").is_file() {
        return false;
    }
    // Main / bare: objects live here. Linked worktree private dir: objects via commondir.
    if dir.join("objects").is_dir() {
        return true;
    }
    resolve_common_dir(dir)
        .ok()
        .is_some_and(|cd| cd.join("objects").is_dir())
}

/// Parse `$GIT_DIR/commondir` (relative paths resolve against `$GIT_DIR`).
/// Cite: gitrepository-layout `commondir`.
pub fn parse_commondir(contents: &str, git_dir: &Path) -> Result<PathBuf, RepoError> {
    let line = contents
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .ok_or_else(|| RepoError::InvalidGitFile("empty commondir".into()))?;
    let path = PathBuf::from(line);
    let resolved = if path.is_absolute() {
        path
    } else {
        git_dir.join(path)
    };
    Ok(fs::canonicalize(&resolved).unwrap_or(resolved))
}

/// Resolve `$GIT_COMMON_DIR` for a private `$GIT_DIR`.
/// Order: env `$GIT_COMMON_DIR`, then `commondir` file, else `git_dir` itself.
pub fn resolve_common_dir(git_dir: &Path) -> Result<PathBuf, RepoError> {
    if let Ok(env_cd) = env::var("GIT_COMMON_DIR") {
        let p = PathBuf::from(env_cd);
        let resolved = if p.is_absolute() { p } else { git_dir.join(p) };
        return Ok(fs::canonicalize(&resolved).unwrap_or(resolved));
    }
    let file = git_dir.join("commondir");
    if file.is_file() {
        let text = fs::read_to_string(&file).map_err(|e| RepoError::Io(e.to_string()))?;
        return parse_commondir(&text, git_dir);
    }
    Ok(git_dir.to_path_buf())
}

/// Discovery result: `$GIT_DIR`, kind, and work-tree path when known from the
/// discovery walk (directory that held the `.git` file/dir).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    pub git_dir: PathBuf,
    pub kind: GitDirKind,
    /// Set when discovered via a work-tree `.git` directory or `.git` file
    /// (linked worktree **or** absorbed submodule checkout).
    pub work_tree: Option<PathBuf>,
}

/// Walk `start` and its parents for a Git directory.
///
/// Claimed: `.git/` directory, `.git` file with `gitdir:` (linked worktrees /
/// submodule checkouts), bare `$GIT_DIR`, and `commondir` / `$GIT_COMMON_DIR`.
pub fn discover_git_dir(start: impl AsRef<Path>) -> Result<(PathBuf, GitDirKind), RepoError> {
    let d = discover(start)?;
    Ok((d.git_dir, d.kind))
}

/// Full discovery including work-tree path for `.git` file checkouts (submodules).
pub fn discover(start: impl AsRef<Path>) -> Result<Discovery, RepoError> {
    let start = start.as_ref();
    let mut cur = if start.is_file() {
        start
            .parent()
            .ok_or_else(|| RepoError::NotFound(start.display().to_string()))?
            .to_path_buf()
    } else {
        start.to_path_buf()
    };
    cur = fs::canonicalize(&cur).unwrap_or(cur);

    loop {
        if looks_like_git_dir(&cur) {
            return Ok(Discovery {
                git_dir: cur,
                kind: GitDirKind::Bare,
                work_tree: None,
            });
        }
        let dot_git = cur.join(".git");
        if dot_git.is_dir() {
            if looks_like_git_dir(&dot_git) {
                return Ok(Discovery {
                    git_dir: dot_git,
                    kind: GitDirKind::DotGitDir,
                    work_tree: Some(cur),
                });
            }
            return Err(RepoError::NotFound(format!(
                ".git dir incomplete at {}",
                dot_git.display()
            )));
        }
        if dot_git.is_file() {
            let text = fs::read_to_string(&dot_git).map_err(|e| RepoError::Io(e.to_string()))?;
            let git_dir = parse_gitdir_file(&text, &cur)?;
            if looks_like_git_dir(&git_dir) {
                // `cur` is the checkout that held the `.git` file (worktree or submodule).
                return Ok(Discovery {
                    git_dir,
                    kind: GitDirKind::DotGitFile,
                    work_tree: Some(cur),
                });
            }
            return Err(RepoError::NotFound(format!(
                "gitdir from .git file incomplete: {}",
                git_dir.display()
            )));
        }
        match cur.parent() {
            Some(parent) if parent != cur => cur = parent.to_path_buf(),
            _ => {
                return Err(RepoError::NotFound(start.display().to_string()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn seed_git_dir(git_dir: &Path) {
        fs::create_dir_all(git_dir.join("objects")).unwrap();
        fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
        fs::write(git_dir.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
    }

    #[test]
    fn discovers_dot_git_dir() {
        let tmp = TempDir::new().unwrap();
        let wt = tmp.path().join("wt");
        fs::create_dir_all(&wt).unwrap();
        seed_git_dir(&wt.join(".git"));
        let (gd, kind) = discover_git_dir(&wt).unwrap();
        assert_eq!(kind, GitDirKind::DotGitDir);
        assert!(gd.ends_with(".git"));
    }

    #[test]
    fn discovers_bare() {
        let tmp = TempDir::new().unwrap();
        let bare = tmp.path().join("bare.git");
        seed_git_dir(&bare);
        let (gd, kind) = discover_git_dir(&bare).unwrap();
        assert_eq!(kind, GitDirKind::Bare);
        assert_eq!(gd, fs::canonicalize(&bare).unwrap_or(bare));
    }

    #[test]
    fn discovers_gitdir_file() {
        let tmp = TempDir::new().unwrap();
        let actual = tmp.path().join("actual.git");
        seed_git_dir(&actual);
        let wt = tmp.path().join("linked");
        fs::create_dir_all(&wt).unwrap();
        let mut f = fs::File::create(wt.join(".git")).unwrap();
        writeln!(f, "gitdir: ../actual.git").unwrap();
        let d = discover(&wt).unwrap();
        assert_eq!(d.kind, GitDirKind::DotGitFile);
        assert!(looks_like_git_dir(&d.git_dir));
        assert_eq!(
            fs::canonicalize(d.work_tree.as_ref().unwrap()).unwrap(),
            fs::canonicalize(&wt).unwrap()
        );
        let (gd, kind) = discover_git_dir(&wt).unwrap();
        assert_eq!(kind, GitDirKind::DotGitFile);
        assert!(looks_like_git_dir(&gd));
    }

    #[test]
    fn parse_gitdir_absolute() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("x.git");
        seed_git_dir(&target);
        let abs = fs::canonicalize(&target).unwrap();
        let parsed =
            parse_gitdir_file(&format!("gitdir: {}\n", abs.display()), tmp.path()).unwrap();
        assert_eq!(parsed, abs);
    }
}
