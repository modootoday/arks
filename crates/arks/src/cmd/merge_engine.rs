//! Three-way tree merge for M15 Fs-2 (diverged histories).
//!
//! Cite: <https://git-scm.com/docs/git-merge>, <https://git-scm.com/docs/git-merge-base>
//! Paths changed on both sides get a line-level three-way content merge;
//! conflicted paths land as index stages 1/2/3 + worktree conflict markers.

use crate::error::{CliError, CliResult};
use crate::util::write_index;
use crate::write_tree::write_tree_from_index;
use arks_index::{Index, IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject, TreeMode};
use arks_oid::Oid;
use arks_repo::{Repository, merge_base_oid};
use arks_store::ObjectStore;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Clone)]
struct Side {
    mode: u32,
    oid: Oid,
}

#[derive(Default)]
pub(crate) struct MergeResult {
    pub tree: Option<Oid>,
    pub conflicts: Vec<String>,
    /// Paths that went through a content merge (git prints `Auto-merging`).
    pub auto_merged: Vec<String>,
}

/// Three-way merge of commit tips into the index + worktree; returns the merge
/// tree when conflict-free. On conflicts, index/worktree are left conflicted
/// and `conflicts` is non-empty (caller MUST NOT create a merge commit).
/// With `ours_checked_out` the work tree already
/// holds `ours`, so paths whose result is the `ours` blob are not rewritten
/// (unrelated local edits survive, as with git-merge).
pub(crate) fn merge_three_way_opts(
    repo: &mut Repository,
    work: &Path,
    ours: &Oid,
    theirs: &Oid,
    ours_checked_out: bool,
) -> CliResult<MergeResult> {
    merge_three_way_labeled(
        repo,
        work,
        ours,
        theirs,
        ours_checked_out,
        &MergeLabels::default(),
    )
}

/// [`merge_three_way_opts`] with conflict marker labels.
pub(crate) fn merge_three_way_labeled(
    repo: &mut Repository,
    work: &Path,
    ours: &Oid,
    theirs: &Oid,
    ours_checked_out: bool,
    labels: &MergeLabels,
) -> CliResult<MergeResult> {
    let base = merge_base_oid(repo, ours, theirs)?
        .ok_or_else(|| CliError::Msg("merge: no common ancestor".into()))?;
    let ours_c = load_commit(repo, ours)?;
    let theirs_c = load_commit(repo, theirs)?;
    let base_c = load_commit(repo, &base)?;
    merge_trees_worktree(
        repo,
        work,
        &base_c.tree,
        &ours_c.tree,
        &theirs_c.tree,
        ours_checked_out,
        labels,
    )
}

/// Three-way merge of trees with an explicit base (cherry-pick / revert use
/// the picked commit's parent, or the commit itself) into the index and work
/// tree; the index is rebuilt from the merge result.
pub(crate) fn merge_trees_worktree(
    repo: &mut Repository,
    work: &Path,
    base_tree: &Oid,
    ours_tree: &Oid,
    theirs_tree: &Oid,
    ours_checked_out: bool,
    labels: &MergeLabels,
) -> CliResult<MergeResult> {
    let keep = ours_checked_out;
    let plan = plan_trees(repo, base_tree, ours_tree, theirs_tree, labels)?;
    let mut entries = plan.entries;
    let conflicts = plan.conflicts;
    let auto_merged = plan.auto_merged;
    for op in plan.ops {
        match op {
            WorkOp::Merged { path, bytes } => {
                let format = repo.object_format();
                repo.store_mut()
                    .put(format, &RawObject::new(ObjectKind::Blob, bytes.clone()))?;
                let abs = work.join(&path);
                if let Some(parent) = abs.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(abs, bytes)?;
            }
            WorkOp::ConflictText { path, bytes } => {
                let abs = work.join(&path);
                if let Some(parent) = abs.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(abs, bytes)?;
            }
            WorkOp::Write { path, side, ours } => {
                if !(keep && ours) {
                    write_blob(repo, work, &path, &side)?;
                }
            }
            WorkOp::Remove(path) => {
                let _ = fs::remove_file(work.join(&path));
            }
            WorkOp::Conflict { path, ours, theirs } => {
                write_conflict_file(repo, work, &path, ours.as_ref(), theirs.as_ref())?;
            }
        }
    }

    sort_entries(&mut entries);
    let index = Index {
        version: 2,
        entries,
        extensions: Vec::new(),
    };
    write_index(repo, &index)?;

    if !conflicts.is_empty() {
        return Ok(MergeResult {
            tree: None,
            conflicts,
            auto_merged,
        });
    }

    let tree = if index.entries.is_empty() {
        let obj = RawObject::new(ObjectKind::Tree, Vec::new());
        let format = repo.object_format();
        repo.store_mut().put(format, &obj)?
    } else {
        let format = repo.object_format();
        write_tree_from_index(repo.store_mut(), &index.entries, format)?
    };
    Ok(MergeResult {
        tree: Some(tree),
        conflicts,
        auto_merged,
    })
}

/// Work-tree side effect of a planned three-way merge.
enum WorkOp {
    /// Write `side` at `path`; `ours` = the result equals our version.
    Write {
        path: String,
        side: Side,
        ours: bool,
    },
    Remove(String),
    Conflict {
        path: String,
        ours: Option<Side>,
        theirs: Option<Side>,
    },
    /// A clean content merge: store the blob and write it.
    Merged {
        path: String,
        bytes: Vec<u8>,
    },
    /// A conflicted content merge: write the file with markers.
    ConflictText {
        path: String,
        bytes: Vec<u8>,
    },
}

/// Pure three-way tree merge plan: result index entries (stage 0 or 1/2/3),
/// conflicted paths and the work-tree operations a checkout merge applies.
struct MergePlan {
    entries: Vec<IndexEntry>,
    conflicts: Vec<String>,
    ops: Vec<WorkOp>,
    auto_merged: Vec<String>,
}

fn plan_trees(
    repo: &Repository,
    base: &Oid,
    ours: &Oid,
    theirs: &Oid,
    labels: &MergeLabels,
) -> CliResult<MergePlan> {
    let base_side = blob_sides(repo, base)?;
    let ours_side = blob_sides(repo, ours)?;
    let theirs_side = blob_sides(repo, theirs)?;

    let mut paths = BTreeSet::new();
    paths.extend(base_side.keys().cloned());
    paths.extend(ours_side.keys().cloned());
    paths.extend(theirs_side.keys().cloned());

    let mut entries: Vec<IndexEntry> = Vec::new();
    let mut conflicts = Vec::new();
    let mut ops = Vec::new();
    let mut auto_merged = Vec::new();
    let take = |entries: &mut Vec<IndexEntry>, path: &str, s: &Side, ours: bool| {
        entries.push(stage0(path, s));
        WorkOp::Write {
            path: path.to_string(),
            side: s.clone(),
            ours,
        }
    };

    for path in paths {
        let b = base_side.get(&path);
        let o = ours_side.get(&path);
        let t = theirs_side.get(&path);
        match (b, o, t) {
            (None, None, None) => {}
            (Some(x), Some(y), Some(z)) if x.oid == y.oid && y.oid == z.oid => {
                ops.push(take(&mut entries, &path, y, true));
            }
            (_, Some(y), Some(z)) if y.oid == z.oid && y.mode == z.mode => {
                ops.push(take(&mut entries, &path, y, true));
            }
            (Some(x), Some(y), Some(z)) if x.oid == y.oid && x.mode == y.mode => {
                // unchanged in ours → take theirs
                ops.push(take(&mut entries, &path, z, false));
            }
            (Some(x), Some(y), Some(z)) if x.oid == z.oid && x.mode == z.mode => {
                // unchanged in theirs → take ours
                ops.push(take(&mut entries, &path, y, true));
            }
            (None, Some(y), None) => ops.push(take(&mut entries, &path, y, true)),
            (None, None, Some(z)) => ops.push(take(&mut entries, &path, z, false)),
            // deleted in both, or deleted on one side and unchanged on the other
            (Some(_), None, None) => ops.push(WorkOp::Remove(path.clone())),
            (Some(x), Some(y), None) if x.oid == y.oid => ops.push(WorkOp::Remove(path.clone())),
            (Some(x), None, Some(z)) if x.oid == z.oid => ops.push(WorkOp::Remove(path.clone())),
            (_, Some(y), Some(z)) if y.mode != 0o160000 && z.mode != 0o160000 => {
                // Both sides changed the content: line-level three-way merge.
                let blob = |s: Option<&Side>| -> CliResult<Vec<u8>> {
                    Ok(match s {
                        Some(s) => repo.get_object(&s.oid)?.payload,
                        None => Vec::new(),
                    })
                };
                let (bb, yb, zb) = (blob(b)?, blob(Some(y))?, blob(Some(z))?);
                let binary = [&bb, &yb, &zb]
                    .iter()
                    .any(|v| v[..v.len().min(8000)].contains(&0));
                auto_merged.push(path.clone());
                let (merged, conflicted) = if binary {
                    (yb.clone(), true)
                } else {
                    merge_text(&bb, &yb, &zb, labels)
                };
                // Mode: the side that changed it wins.
                let mode = match b {
                    Some(x) if x.mode == y.mode => z.mode,
                    _ => y.mode,
                };
                if !conflicted {
                    let oid = RawObject::new(ObjectKind::Blob, merged.clone())
                        .oid(repo.object_format())?;
                    entries.push(stage0(&path, &Side { mode, oid }));
                    ops.push(WorkOp::Merged {
                        path: path.clone(),
                        bytes: merged,
                    });
                    continue;
                }
                conflicts.push(path.clone());
                if let Some(x) = b {
                    entries.push(stage_n(&path, x, 1));
                }
                entries.push(stage_n(&path, y, 2));
                entries.push(stage_n(&path, z, 3));
                ops.push(WorkOp::ConflictText {
                    path: path.clone(),
                    bytes: merged,
                });
            }
            _ => {
                // Conflict: stages 1/2/3 + markers
                conflicts.push(path.clone());
                if let Some(x) = b {
                    entries.push(stage_n(&path, x, 1));
                }
                if let Some(y) = o {
                    entries.push(stage_n(&path, y, 2));
                }
                if let Some(z) = t {
                    entries.push(stage_n(&path, z, 3));
                }
                ops.push(WorkOp::Conflict {
                    path: path.clone(),
                    ours: o.cloned(),
                    theirs: t.cloned(),
                });
            }
        }
    }
    Ok(MergePlan {
        entries,
        conflicts,
        ops,
        auto_merged,
    })
}

/// Merge three trees in memory (git-merge-tree): writes result tree objects
/// only — never the index or the work tree. Returns the result tree when
/// conflict-free, else the conflicted paths.
/// Cite: <https://git-scm.com/docs/git-merge-tree>
pub(crate) fn merge_trees_in_memory(
    repo: &mut Repository,
    base: &Oid,
    ours: &Oid,
    theirs: &Oid,
) -> CliResult<MergeResult> {
    let mut plan = plan_trees(repo, base, ours, theirs, &MergeLabels::default())?;
    if !plan.conflicts.is_empty() {
        return Ok(MergeResult {
            tree: None,
            conflicts: plan.conflicts,
            auto_merged: plan.auto_merged,
        });
    }
    sort_entries(&mut plan.entries);
    let format = repo.object_format();
    for op in &plan.ops {
        if let WorkOp::Merged { bytes, .. } = op {
            repo.store_mut()
                .put(format, &RawObject::new(ObjectKind::Blob, bytes.clone()))?;
        }
    }
    let tree = if plan.entries.is_empty() {
        let obj = RawObject::new(ObjectKind::Tree, Vec::new());
        repo.store_mut().put(format, &obj)?
    } else {
        write_tree_from_index(repo.store_mut(), &plan.entries, format)?
    };
    Ok(MergeResult {
        tree: Some(tree),
        conflicts: Vec::new(),
        auto_merged: plan.auto_merged,
    })
}

fn load_commit(repo: &Repository, oid: &Oid) -> CliResult<Commit> {
    let obj = repo.get_object(oid)?;
    if obj.kind != ObjectKind::Commit {
        return Err(CliError::Msg(format!("{} is not a commit", oid.to_hex())));
    }
    Ok(Commit::decode(&obj.payload)?)
}

fn blob_sides(repo: &Repository, tree: &Oid) -> CliResult<BTreeMap<String, Side>> {
    let mut out = BTreeMap::new();
    walk_tree(repo, tree, "", &mut out)?;
    Ok(out)
}

fn walk_tree(
    repo: &Repository,
    tree_oid: &Oid,
    prefix: &str,
    out: &mut BTreeMap<String, Side>,
) -> CliResult<()> {
    let obj = repo.get_object(tree_oid)?;
    if obj.kind != ObjectKind::Tree {
        return Err(CliError::Msg(format!("not a tree {}", tree_oid.to_hex())));
    }
    let tree = arks_object::Tree::decode(&obj.payload, repo.oid_kind())?;
    for e in tree.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        match e.mode {
            TreeMode::Tree => walk_tree(repo, &e.oid, &path, out)?,
            TreeMode::Gitlink => {
                // Treat gitlink as opaque blob-like for merge (no recurse).
                out.insert(
                    path,
                    Side {
                        mode: 0o160000,
                        oid: e.oid,
                    },
                );
            }
            TreeMode::File | TreeMode::Executable | TreeMode::Symlink => {
                let mode = match e.mode {
                    TreeMode::Executable => 0o100755,
                    TreeMode::Symlink => 0o120000,
                    _ => 0o100644,
                };
                out.insert(path, Side { mode, oid: e.oid });
            }
        }
    }
    Ok(())
}

fn stage0(path: &str, s: &Side) -> IndexEntry {
    stage_n(path, s, 0)
}

fn stage_n(path: &str, s: &Side, stage: u16) -> IndexEntry {
    let flags = ((stage & 3) << 12) | ((path.len().min(0x0fff)) as u16);
    IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode: s.mode,
        uid: 0,
        gid: 0,
        size: 0,
        oid: s.oid,
        flags,
        flags_extended: None,
        path: path.to_owned(),
    }
}

fn write_blob(repo: &Repository, work: &Path, path: &str, s: &Side) -> CliResult<()> {
    if s.mode == 0o160000 {
        let abs = work.join(path);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&abs)?;
        return Ok(());
    }
    let obj = repo.get_object(&s.oid)?;
    let abs = work.join(path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&abs, &obj.payload)?;
    Ok(())
}

fn write_conflict_file(
    repo: &Repository,
    work: &Path,
    path: &str,
    ours: Option<&Side>,
    theirs: Option<&Side>,
) -> CliResult<()> {
    let mut body = String::new();
    body.push_str("<<<<<<< ours\n");
    if let Some(o) = ours {
        if o.mode != 0o160000 {
            let obj = repo.get_object(&o.oid)?;
            body.push_str(&String::from_utf8_lossy(&obj.payload));
            if !body.ends_with('\n') {
                body.push('\n');
            }
        }
    }
    body.push_str("=======\n");
    if let Some(t) = theirs {
        if t.mode != 0o160000 {
            let obj = repo.get_object(&t.oid)?;
            body.push_str(&String::from_utf8_lossy(&obj.payload));
            if !body.ends_with('\n') {
                body.push('\n');
            }
        }
    }
    body.push_str(">>>>>>> theirs\n");
    let abs = work.join(path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(abs, body)?;
    Ok(())
}

/// Conflict marker labels and style for content merges.
#[derive(Clone, Debug)]
pub(crate) struct MergeLabels {
    pub ours: String,
    pub base: String,
    pub theirs: String,
    /// `merge.conflictStyle=diff3` / `--conflict=diff3`: show the base too.
    pub diff3: bool,
    /// `-X ours` / `-X theirs` / `-X union`: resolve conflicting hunks.
    pub favor: Favor,
}

/// Side taken for conflicting hunks of a content merge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Favor {
    #[default]
    None,
    Ours,
    Theirs,
    Union,
}

impl Default for MergeLabels {
    fn default() -> Self {
        Self {
            ours: "ours".into(),
            base: "base".into(),
            theirs: "theirs".into(),
            diff3: false,
            favor: Favor::None,
        }
    }
}

fn split_keep(b: &[u8]) -> Vec<&[u8]> {
    if b.is_empty() {
        return Vec::new();
    }
    b.split_inclusive(|c| *c == b'\n').collect()
}

/// Line-level three-way merge (diff3 over two LCS alignments with the base).
/// Chunks changed on one side take that side; identical changes merge;
/// overlapping or touching changes conflict, trimmed to the lines that differ
/// (common leading / trailing lines of both sides stay outside the markers).
/// Returns the merged bytes and whether any conflict was written.
/// Cite: <https://git-scm.com/docs/git-merge-file> (behaviour, not code).
pub(crate) fn merge_text(
    base: &[u8],
    ours: &[u8],
    theirs: &[u8],
    labels: &MergeLabels,
) -> (Vec<u8>, bool) {
    use crate::cmd::unified_diff::matched_lines;
    let b = split_keep(base);
    let a = split_keep(ours);
    let c = split_keep(theirs);
    let ma = matched_lines(&a, &b);
    let mc = matched_lines(&c, &b);
    let (nb, na, nc) = (b.len(), a.len(), c.len());
    let mut out: Vec<u8> = Vec::new();
    let mut conflicted = false;
    let (mut i, mut j, mut k) = (0usize, 0usize, 0usize);
    let nl = |v: &mut Vec<u8>| {
        if !v.is_empty() && !v.ends_with(b"\n") {
            v.push(b'\n');
        }
    };
    loop {
        while i < nb && ma[i] == Some(j) && mc[i] == Some(k) {
            out.extend_from_slice(b[i]);
            i += 1;
            j += 1;
            k += 1;
        }
        if i >= nb && j >= na && k >= nc {
            break;
        }
        // Next base line kept by both sides.
        let mut ii = i;
        let (mut jj, mut kk) = (na, nc);
        while ii < nb {
            if let (Some(x), Some(y)) = (ma[ii], mc[ii]) {
                if x >= j && y >= k {
                    jj = x;
                    kk = y;
                    break;
                }
            }
            ii += 1;
        }
        if ii >= nb {
            ii = nb;
            jj = na;
            kk = nc;
        }
        let bs = &b[i..ii];
        let as_ = &a[j..jj];
        let cs = &c[k..kk];
        if as_ == bs {
            cs.iter().for_each(|l| out.extend_from_slice(l));
        } else if cs == bs || as_ == cs {
            as_.iter().for_each(|l| out.extend_from_slice(l));
        } else if labels.favor != Favor::None {
            if labels.favor != Favor::Theirs {
                as_.iter().for_each(|l| out.extend_from_slice(l));
            }
            if labels.favor != Favor::Ours {
                if labels.favor == Favor::Union {
                    nl(&mut out);
                }
                cs.iter().for_each(|l| out.extend_from_slice(l));
            }
        } else {
            // Zealous trimming: shared head and tail of both sides stay outside.
            let mut pre = 0;
            while pre < as_.len() && pre < cs.len() && as_[pre] == cs[pre] {
                pre += 1;
            }
            let mut suf = 0;
            while suf < as_.len() - pre
                && suf < cs.len() - pre
                && as_[as_.len() - 1 - suf] == cs[cs.len() - 1 - suf]
            {
                suf += 1;
            }
            if labels.diff3 {
                pre = 0;
                suf = 0;
            }
            as_[..pre].iter().for_each(|l| out.extend_from_slice(l));
            nl(&mut out);
            out.extend_from_slice(format!("<<<<<<< {}\n", labels.ours).as_bytes());
            as_[pre..as_.len() - suf]
                .iter()
                .for_each(|l| out.extend_from_slice(l));
            nl(&mut out);
            if labels.diff3 {
                out.extend_from_slice(format!("||||||| {}\n", labels.base).as_bytes());
                bs.iter().for_each(|l| out.extend_from_slice(l));
                nl(&mut out);
            }
            out.extend_from_slice(b"=======\n");
            cs[pre..cs.len() - suf]
                .iter()
                .for_each(|l| out.extend_from_slice(l));
            nl(&mut out);
            out.extend_from_slice(format!(">>>>>>> {}\n", labels.theirs).as_bytes());
            as_[as_.len() - suf..]
                .iter()
                .for_each(|l| out.extend_from_slice(l));
            conflicted = true;
        }
        i = ii;
        j = jj;
        k = kk;
    }
    (out, conflicted)
}

#[cfg(test)]
mod diff3_tests {
    use super::*;

    #[test]
    fn clean_and_conflicting_merges() {
        let l = MergeLabels::default();
        let (m, c) = merge_text(b"1\n2\n3\n", b"ONE\n2\n3\n", b"1\n2\n3\n4\n", &l);
        assert!(!c);
        assert_eq!(m, b"ONE\n2\n3\n4\n");
        let (m, c) = merge_text(b"1\n2\n3\n", b"1\n2\nX\n", b"1\n2\nY\n", &l);
        assert!(c);
        assert_eq!(
            m,
            b"1\n2\n<<<<<<< ours\nX\n=======\nY\n>>>>>>> theirs\n".to_vec()
        );
    }
}
