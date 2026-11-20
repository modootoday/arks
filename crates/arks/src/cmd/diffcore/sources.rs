//! Diff sides read from the index and the work tree.
//!
//! As in git, plumbing (`diff-files`, `diff-index` without `--cached`)
//! does not refresh the index: a work-tree file whose stat data differs
//! from its index entry is reported with the all-zero object id. Porcelain
//! `diff` hashes such files and drops those whose content is unchanged.
//!
//! Primary: <https://git-scm.com/docs/git-diff-files>,
//! <https://git-scm.com/docs/git-diff-index>.

use super::{MODE_GITLINK, MODE_LINK, Side};
use crate::error::CliResult;
use crate::util::read_index;
use arks_index::IndexEntry;
use arks_object::{ObjectKind, RawObject};
use arks_repo::Repository;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Stage-0 index entries by path, plus the unmerged paths.
pub fn index_entries(
    repo: &Repository,
) -> CliResult<(BTreeMap<String, IndexEntry>, BTreeSet<String>)> {
    let mut map = BTreeMap::new();
    let mut unmerged = BTreeSet::new();
    for e in read_index(repo)?.entries {
        if e.stage() == 0 {
            map.insert(e.path.clone(), e);
        } else {
            unmerged.insert(e.path.clone());
        }
    }
    Ok((map, unmerged))
}

pub fn index_side(e: &IndexEntry) -> Side {
    Side::blob(&e.path, e.mode, e.oid)
}

/// git `ie_match_stat` (core.checkStat=default).
pub(crate) fn stat_clean(e: &IndexEntry, meta: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        e.size == meta.len().min(u32::MAX as u64) as u32
            && e.mtime_sec == meta.mtime() as u32
            && e.mtime_nsec == meta.mtime_nsec() as u32
            && e.ctime_sec == meta.ctime() as u32
            && e.ctime_nsec == meta.ctime_nsec() as u32
            && e.ino == meta.ino() as u32
            && e.uid == meta.uid()
            && e.gid == meta.gid()
    }
    #[cfg(not(unix))]
    {
        crate::cmd::shared::index_stat_matches(e, meta)
    }
}

/// Work-tree side of an index entry (`None`: file gone). With `refresh`,
/// stat-dirty files are hashed and keep the index id when unchanged.
pub fn worktree_side(
    repo: &Repository,
    work: &Path,
    e: &IndexEntry,
    refresh: bool,
) -> CliResult<Option<Side>> {
    let abs = work.join(&e.path);
    let Ok(meta) = fs::symlink_metadata(&abs) else {
        return Ok(None);
    };
    if e.mode == MODE_GITLINK {
        // Submodule content is not inspected; a checkout dir keeps the id.
        return Ok(Some(index_side(e)));
    }
    if meta.is_dir() {
        return Ok(None);
    }
    let mode = if meta.file_type().is_symlink() {
        MODE_LINK
    } else {
        crate::cmd::shared::file_mode(&meta)
    };
    if stat_clean(e, &meta) && mode == e.mode {
        return Ok(Some(index_side(e)));
    }
    let mut side = Side {
        path: e.path.clone(),
        mode,
        oid: None,
        work: Some(abs.clone()),
    };
    if refresh {
        let bytes = if mode == MODE_LINK {
            fs::read_link(&abs)?
                .to_string_lossy()
                .into_owned()
                .into_bytes()
        } else {
            fs::read(&abs)?
        };
        let oid = RawObject::new(ObjectKind::Blob, bytes).oid(repo.object_format())?;
        if oid == e.oid {
            side.oid = Some(oid);
            side.work = None;
        }
    }
    Ok(Some(side))
}

/// Flat `Side` map of a tree.
pub fn tree_sides(repo: &Repository, tree: &arks_oid::Oid) -> CliResult<BTreeMap<String, Side>> {
    Ok(super::flat_tree(repo, tree)?
        .into_iter()
        .map(|(p, (m, o))| {
            let s = Side::blob(&p, m, o);
            (p, s)
        })
        .collect())
}
