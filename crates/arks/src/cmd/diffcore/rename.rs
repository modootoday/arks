//! Rename / copy detection and rewrite breaking (gitdiffcore
//! "diffcore-rename" and "diffcore-break").
//!
//! Similarity follows the documented measure: the share of the larger
//! file that is common content, counted over chunks that end at a newline
//! or after 64 bytes (CR before LF ignored in text). Exact (same object)
//! renames are found first; remaining created files are matched to the
//! most similar source above the score threshold. A deleted source used
//! once is a rename; further uses (and modified or, with
//! `--find-copies-harder`, unmodified sources) are copies.
//!
//! Primary: <https://git-scm.com/docs/gitdiffcore>,
//! <https://git-scm.com/docs/diff-options> (`-M`, `-C`, `-B`, `-l`).

use super::options::{Detect, DiffOptions, MAX_SCORE};
use super::{Content, Pair, Side};
use crate::error::CliResult;
use std::collections::HashMap;

/// Chunk histogram of a blob: chunk bytes -> total bytes.
fn chunks(b: &[u8]) -> HashMap<&[u8], u64> {
    let text = !super::xdiff::is_binary(b);
    let mut m: HashMap<&[u8], u64> = HashMap::new();
    let mut start = 0;
    let mut i = 0;
    let mut len = 0u64;
    while i < b.len() {
        let c = b[i];
        i += 1;
        if text && c == b'\r' && b.get(i) == Some(&b'\n') {
            // Counted as part of the chunk key, not its size.
            continue;
        }
        len += 1;
        if c == b'\n' || i - start >= 64 {
            *m.entry(&b[start..i]).or_default() += len;
            start = i;
            len = 0;
        }
    }
    if start < b.len() {
        *m.entry(&b[start..]).or_default() += len;
    }
    m
}

/// `(copied bytes of src, bytes added in dst)`.
pub fn count_changes(src: &[u8], dst: &[u8]) -> (u64, u64) {
    let (a, b) = (chunks(src), chunks(dst));
    let mut copied = 0;
    let mut added = 0;
    for (k, &db) in &b {
        let sb = a.get(k).copied().unwrap_or(0);
        if sb < db {
            copied += sb;
            added += db - sb;
        } else {
            copied += db;
        }
    }
    (copied, added)
}

/// Similarity score of `dst` against `src` (0..=MAX_SCORE), or 0 when the
/// sizes alone rule out reaching `min`.
pub fn similarity(src: &[u8], dst: &[u8], min: u32) -> u32 {
    let (ss, ds) = (src.len() as u64, dst.len() as u64);
    let base = ss.max(ds);
    let delta = ss.abs_diff(ds);
    if base == 0 {
        return MAX_SCORE;
    }
    if base * ((MAX_SCORE - min) as u64) < delta * (MAX_SCORE as u64) {
        return 0;
    }
    let (copied, _) = count_changes(src, dst);
    (copied * MAX_SCORE as u64 / base) as u32
}

/// Detect renames / copies in place (and `-B` rewrites).
pub fn detect(
    c: &Content<'_>,
    o: &DiffOptions,
    pairs: &mut Vec<Pair>,
    unchanged: &[Side],
) -> CliResult<()> {
    if let Some((brk, _merge)) = o.break_rewrites {
        mark_rewrites(c, pairs, brk)?;
    }
    if o.detect == Detect::Off {
        return Ok(());
    }
    let copies = o.detect == Detect::Copies;
    // Destinations: created files.
    let dsts: Vec<usize> = (0..pairs.len())
        .filter(|&i| pairs[i].old.is_none() && pairs[i].new.as_ref().is_some_and(|s| !s.is_tree()))
        .collect();
    if dsts.is_empty() {
        return Ok(());
    }
    // Sources: deleted files; with -C also modified ones (and, harder,
    // unmodified ones).
    struct Src {
        side: Side,
        deleted: Option<usize>,
    }
    let mut srcs: Vec<Src> = Vec::new();
    for (i, p) in pairs.iter().enumerate() {
        if let Some(s) = &p.old {
            if s.is_tree() {
                continue;
            }
            if p.new.is_none() {
                srcs.push(Src {
                    side: s.clone(),
                    deleted: Some(i),
                });
            } else if copies {
                srcs.push(Src {
                    side: s.clone(),
                    deleted: None,
                });
            }
        }
    }
    if copies && o.find_copies_harder {
        for s in unchanged {
            srcs.push(Src {
                side: s.clone(),
                deleted: None,
            });
        }
    }
    if srcs.is_empty() {
        return Ok(());
    }
    // (dst index, src index, score)
    let mut matched: HashMap<usize, (usize, u32)> = HashMap::new();
    // Exact matches first (same object id and file type).
    for &d in &dsts {
        let ds = pairs[d].new.clone().unwrap_or_else(|| unreachable!());
        let doid = c.oid(&ds)?;
        if !o.rename_empty && c.bytes(&ds)?.is_empty() {
            continue;
        }
        let mut best: Option<usize> = None;
        for (si, s) in srcs.iter().enumerate() {
            if s.side.mode & 0o170000 != ds.mode & 0o170000 {
                continue;
            }
            if c.oid(&s.side)? == doid {
                // Prefer a source with the same basename, then a deleted one.
                let base = |p: &str| p.rsplit('/').next().unwrap_or(p).to_string();
                let better = match best {
                    None => true,
                    Some(b) => {
                        let bb = base(&srcs[b].side.path) == base(&ds.path);
                        let sb = base(&s.side.path) == base(&ds.path);
                        (sb && !bb)
                            || (sb == bb && s.deleted.is_some() && srcs[b].deleted.is_none())
                    }
                };
                if better {
                    best = Some(si);
                }
            }
        }
        if let Some(si) = best {
            matched.insert(d, (si, MAX_SCORE));
        }
    }
    // Inexact matches.
    let limit = o.rename_limit.unwrap_or(1000);
    let rem: Vec<usize> = dsts
        .iter()
        .copied()
        .filter(|d| !matched.contains_key(d))
        .collect();
    let min_score = if copies {
        o.copy_score.min(o.rename_score)
    } else {
        o.rename_score
    };
    if !rem.is_empty() && (limit == 0 || rem.len() * srcs.len() <= limit * limit) {
        let mut cands: Vec<(u32, usize, usize)> = Vec::new();
        for &d in &rem {
            let ds = pairs[d].new.clone().unwrap_or_else(|| unreachable!());
            let db = c.bytes(&ds)?;
            if !o.rename_empty && db.is_empty() {
                continue;
            }
            for (si, s) in srcs.iter().enumerate() {
                if s.side.mode & 0o170000 != ds.mode & 0o170000 {
                    continue;
                }
                let sb = c.bytes(&s.side)?;
                if sb.is_empty() && !o.rename_empty {
                    continue;
                }
                let need = if s.deleted.is_some() {
                    o.rename_score
                } else {
                    o.copy_score
                };
                let score = similarity(&sb, &db, need);
                if score >= need && score > 0 {
                    cands.push((score, d, si));
                }
            }
        }
        let _ = min_score;
        // Best score first; ties keep the earlier destination / source.
        cands.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        for (score, d, si) in cands {
            if matched.contains_key(&d) {
                continue;
            }
            matched.insert(d, (si, score));
        }
    }
    if matched.is_empty() {
        return Ok(());
    }
    // Rename for the first use (in destination order) of a deleted
    // source, copies for every other use.
    let mut used: HashMap<usize, bool> = HashMap::new();
    let mut drop: Vec<usize> = Vec::new();
    let mut order: Vec<usize> = matched.keys().copied().collect();
    order.sort_by(|a, b| pairs[*a].path().cmp(pairs[*b].path()));
    for d in order {
        let (si, score) = matched[&d];
        let src = &srcs[si];
        let rename = src.deleted.is_some() && !used.contains_key(&si);
        if rename {
            used.insert(si, true);
            if let Some(di) = src.deleted {
                drop.push(di);
            }
        }
        if !rename && !copies {
            continue;
        }
        let p = &mut pairs[d];
        p.old = Some(src.side.clone());
        p.status = Some(if rename { 'R' } else { 'C' });
        p.score = score;
    }
    drop.sort_unstable();
    drop.dedup();
    for i in drop.into_iter().rev() {
        pairs.remove(i);
    }
    // Queue order: by the destination path (renames sort with creates).
    pairs.sort_by(|a, b| a.path().cmp(b.path()));
    Ok(())
}

/// `-B`: mark modified pairs whose content was mostly rewritten.
fn mark_rewrites(c: &Content<'_>, pairs: &mut [Pair], brk: u32) -> CliResult<()> {
    for p in pairs.iter_mut() {
        let (Some(a), Some(b)) = (&p.old, &p.new) else {
            continue;
        };
        if a.is_tree() || b.is_tree() || a.mode & 0o170000 != b.mode & 0o170000 {
            continue;
        }
        let (ab, bb) = (c.bytes(a)?, c.bytes(b)?);
        // Small files are never broken (git's minimum break size).
        if ab.is_empty() || ab.len().max(bb.len()) < 400 {
            continue;
        }
        let (copied, added) = count_changes(&ab, &bb);
        let src_removed = ab.len() as u64 - copied.min(ab.len() as u64);
        let delta = src_removed + added;
        let base = (ab.len() as u64).max(bb.len() as u64).max(1);
        let dissim = (delta.min(base) * MAX_SCORE as u64 / base) as u32;
        if dissim >= brk {
            p.score = (src_removed * MAX_SCORE as u64 / ab.len() as u64) as u32;
            p.status = Some('M');
            p.unmerged = false;
            // Rewrites keep status M; the score shows as dissimilarity.
            p.score = p.score.max(1);
        }
    }
    Ok(())
}
