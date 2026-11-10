//! Product-private sidecar layout for CELIR indexes.
//!
//! Normative path (**beside** `.git` for non-bare):
//! `<worktree>/.arks/celir-index/`
//! Bare: `<bare-root>/.arks/celir-index/`
//! ([[sot:root/celir-index-dual-path]], M24). Outside Git object/ref/pack meaning.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Leaf directory name for the product-private CELIR index root.
pub const SIDECAR_LEAF: &str = "celir-index";

/// Product-private container directory name (sibling of `.git` when non-bare).
pub const SIDECAR_CONTAINER: &str = ".arks";

/// Resolve the product-private sidecar index root for a Git directory.
///
/// - Non-bare (`…/.git`): `<parent>/.arks/celir-index` (beside `.git`)
/// - Bare: `<git-dir>/.arks/celir-index`
///
/// Does not create directories.
pub fn sidecar_root(git_dir: impl AsRef<Path>) -> PathBuf {
    let git_dir = git_dir.as_ref();
    let base = if git_dir
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case(".git"))
    {
        git_dir.parent().unwrap_or(git_dir)
    } else {
        git_dir
    };
    base.join(SIDECAR_CONTAINER).join(SIDECAR_LEAF)
}

/// Ensure the sidecar root exists; return the path.
pub fn ensure_sidecar_root(git_dir: impl AsRef<Path>) -> io::Result<PathBuf> {
    let root = sidecar_root(git_dir);
    fs::create_dir_all(&root)?;
    Ok(root)
}

/// True when `path` looks like a product-private arks CELIR sidecar (best-effort).
pub fn is_sidecar_path(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    let mut comps = path.components().rev();
    let Some(std::path::Component::Normal(a)) = comps.next() else {
        return false;
    };
    let Some(std::path::Component::Normal(b)) = comps.next() else {
        return false;
    };
    a == SIDECAR_LEAF && b == SIDECAR_CONTAINER
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn sidecar_beside_dot_git_not_objects() {
        let dir = TempDir::new().unwrap();
        let git = dir.path().join(".git");
        fs::create_dir_all(git.join("objects")).unwrap();
        let root = ensure_sidecar_root(&git).unwrap();
        assert_eq!(root, dir.path().join(".arks").join("celir-index"));
        assert!(root.is_dir());
        assert!(!root.starts_with(git.join("objects")));
        assert!(is_sidecar_path(&root));
        // Beside `.git`, not under it.
        assert!(!root.starts_with(&git));
    }

    #[test]
    fn bare_repo_sidecar_under_bare_root() {
        let dir = TempDir::new().unwrap();
        let bare = dir.path().join("repo.git");
        fs::create_dir_all(bare.join("objects")).unwrap();
        let root = sidecar_root(&bare);
        assert_eq!(root, bare.join(".arks").join("celir-index"));
        assert!(is_sidecar_path(&root));
    }
}
