//! Repository discovery/layout; composes store + refs (M3 Phase R + residual worktree).
//!
//! # Claimed subset (honest)
//!
//! - Discover a Git directory from a start path by walking parents for a `.git`
//!   directory or `.git` file (`gitdir: …`), or treating a directory that already
//!   looks like `$GIT_DIR` (has `HEAD` + objects via common dir) as bare.
//! - Resolve `commondir` / `$GIT_COMMON_DIR` for linked worktrees.
//! - Compose [`FilesystemGitStore`] on `$GIT_COMMON_DIR/objects` and
//!   [`FilesystemRefStore`] with dual-root when linked.
//! - Resolve `HEAD` → OID via refs; optional [`Repository::get_object`] through
//!   the store facade; [`Repository::index_path`] / [`Repository::work_tree`].
//!
//! # Not claimed
//!
//! - CELIR (excluded), lease API, pack, CLI porcelain orchestration, remotes
//!
//! Primary: <https://git-scm.com/docs/gitrepository-layout>
//! Cite: `.spec/plan/20261108090648-m3-refs-index-repo.plan.md`,
//! `.spec/plan/20261109022956-worktree-submodule-residual.plan.md`,
//! `.spec/sot/crate-architecture.sot.md`
//! No `gix`.

#![forbid(unsafe_code)]

mod ancestry;
mod discover;
mod error;
mod repo;
mod tree;

pub use ancestry::{ancestor_set, first_parent_set, is_ancestor_first_parent, merge_base_oid};
pub use discover::{
    Discovery, GitDirKind, discover, discover_git_dir, parse_commondir, parse_gitdir_file,
    resolve_common_dir,
};
pub use error::RepoError;
pub use repo::Repository;
pub use tree::{commit_tree_map, flatten_tree, flatten_tree_entries, head_blob_map, tree_blob_map};
