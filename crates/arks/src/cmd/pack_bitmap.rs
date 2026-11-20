//! Reachability bitmap (`.bitmap`) writer for a pack.
//!
//! Version 1 with `BITMAP_OPT_FULL_DAG`: four EWAH type bitmaps (commits,
//! trees, blobs, tags) followed by one EWAH bitmap per selected commit (the
//! ref tips whose whole history is in the pack), each without XOR
//! compression. Bit `i` stands for the `i`-th object in pack (offset) order.
//! Cite: <https://git-scm.com/docs/gitformat-pack> and git's
//! `Documentation/technical/bitmap-format.txt` (format description only).

use crate::error::CliResult;
use arks_object::{ObjectKind, Tree, TreeMode};
use arks_oid::Oid;
use arks_pack::PackIndex;
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};

const BITMAP_OPT_FULL_DAG: u16 = 0x1;
const MAX_RUN: u64 = (1 << 32) - 1;
const MAX_LIT: u64 = (1 << 31) - 1;

/// EWAH-compress a plain bitmap given as set bit positions.
fn ewah(bits: &[usize]) -> Vec<u8> {
    let bit_size = bits.iter().max().map_or(0, |m| m + 1);
    let nwords = bit_size.div_ceil(64);
    let mut words = vec![0u64; nwords];
    for &b in bits {
        words[b / 64] |= 1u64 << (b % 64);
    }
    let mut buf: Vec<u64> = vec![0];
    let mut rlw = 0usize;
    let run_len = |w: u64| (w >> 1) & MAX_RUN;
    let lits = |w: u64| w >> 33;
    for w in words {
        if w == 0 || w == u64::MAX {
            let bit = u64::from(w != 0);
            let r = buf[rlw];
            if lits(r) == 0 && (run_len(r) == 0 || r & 1 == bit) && run_len(r) < MAX_RUN {
                buf[rlw] = (r & !1 & !(MAX_RUN << 1)) | bit | ((run_len(r) + 1) << 1);
            } else {
                buf.push(bit | (1 << 1));
                rlw = buf.len() - 1;
            }
        } else {
            let r = buf[rlw];
            if lits(r) < MAX_LIT {
                buf[rlw] = (r & ((1u64 << 33) - 1)) | ((lits(r) + 1) << 33);
            } else {
                buf.push(1u64 << 33);
                rlw = buf.len() - 1;
            }
            buf.push(w);
        }
    }
    let mut out = Vec::with_capacity(12 + buf.len() * 8);
    out.extend_from_slice(&(bit_size as u32).to_be_bytes());
    out.extend_from_slice(&(buf.len() as u32).to_be_bytes());
    for w in &buf {
        out.extend_from_slice(&w.to_be_bytes());
    }
    out.extend_from_slice(&(rlw as u32).to_be_bytes());
    out
}

/// Objects reachable from `commit` (commits, trees, blobs), or `None` when
/// one of them is not in the pack.
fn reachable(repo: &Repository, commit: &Oid, pos: &HashMap<Oid, usize>) -> Option<Vec<usize>> {
    let mut seen: HashSet<Oid> = HashSet::new();
    let mut bits = Vec::new();
    let mut stack = vec![*commit];
    while let Some(oid) = stack.pop() {
        if !seen.insert(oid) {
            continue;
        }
        bits.push(*pos.get(&oid)?);
        let obj = repo.get_object(&oid).ok()?;
        match obj.kind {
            ObjectKind::Commit => {
                let c = arks_object::Commit::decode(&obj.payload).ok()?;
                stack.push(c.tree);
                stack.extend(c.parents);
            }
            ObjectKind::Tree => {
                for e in Tree::decode(&obj.payload, repo.oid_kind()).ok()?.entries {
                    if e.mode != TreeMode::Gitlink {
                        stack.push(e.oid);
                    }
                }
            }
            ObjectKind::Tag => {
                stack.push(crate::cmd::shared::tag_target(&obj.payload)?);
            }
            ObjectKind::Blob => {}
        }
    }
    Some(bits)
}

/// The `.bitmap` bytes for the pack described by `index` (`kinds` gives
/// each idx entry's object type); bits follow pack (offset) order.
pub(crate) fn encode(
    repo: &Repository,
    index: &PackIndex,
    kinds: &[ObjectKind],
) -> CliResult<Vec<u8>> {
    let mut by_offset: Vec<usize> = (0..index.entries.len()).collect();
    by_offset.sort_by_key(|i| index.entries[*i].offset);
    let order: Vec<(Oid, ObjectKind, usize)> = by_offset
        .iter()
        .map(|&i| (index.entries[i].oid, kinds[i], i))
        .collect();
    encode_ordered(repo, &order, &index.pack_checksum)
}

/// A bitmap over `order` (bit position = place in `order`); each element is
/// (object, type, index position written for a selected commit), and
/// `checksum` is the pack's (or multi-pack-index's) trailer.
pub(crate) fn encode_ordered(
    repo: &Repository,
    order: &[(Oid, ObjectKind, usize)],
    checksum: &[u8],
) -> CliResult<Vec<u8>> {
    let mut pos: HashMap<Oid, usize> = HashMap::new();
    let mut idx_pos: HashMap<Oid, (usize, ObjectKind)> = HashMap::new();
    let mut type_bits: [Vec<usize>; 4] = Default::default();
    for (p, (oid, kind, i)) in order.iter().enumerate() {
        pos.insert(*oid, p);
        idx_pos.insert(*oid, (*i, *kind));
        let t = match kind {
            ObjectKind::Commit => 0,
            ObjectKind::Tree => 1,
            ObjectKind::Blob => 2,
            ObjectKind::Tag => 3,
        };
        type_bits[t].push(p);
    }
    // Selected commits: ref tips (peeled) present in the pack, by name.
    let mut tips: Vec<(String, Oid)> = repo.refs().list_refs("refs/")?;
    tips.sort_by(|a, b| a.0.cmp(&b.0));
    if let Ok(h) = repo.head_oid() {
        tips.push(("HEAD".into(), h));
    }
    let mut selected: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut done: HashSet<Oid> = HashSet::new();
    for (_, oid) in tips {
        let Ok(c) = crate::cmd::shared::peel_tags(repo, &oid) else {
            continue;
        };
        let Some((i, kind)) = idx_pos.get(&c).copied() else {
            continue;
        };
        if kind != ObjectKind::Commit || !done.insert(c) {
            continue;
        }
        if let Some(mut bits) = reachable(repo, &c, &pos) {
            bits.sort_unstable();
            selected.push((i, bits));
        }
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"BITM");
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&BITMAP_OPT_FULL_DAG.to_be_bytes());
    out.extend_from_slice(&(selected.len() as u32).to_be_bytes());
    out.extend_from_slice(checksum);
    for t in &type_bits {
        out.extend_from_slice(&ewah(t));
    }
    for (i, bits) in &selected {
        out.extend_from_slice(&(*i as u32).to_be_bytes());
        out.push(0); // XOR offset
        out.push(0); // flags
        out.extend_from_slice(&ewah(bits));
    }
    arks_pack::trailer(&mut out, repo.object_format());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::ewah;

    #[test]
    fn ewah_runs_and_literals() {
        // Bits 0 and 1 set: one literal word.
        let e = ewah(&[0, 1]);
        assert_eq!(&e[..4], &2u32.to_be_bytes());
        assert_eq!(&e[4..8], &2u32.to_be_bytes());
        // 64 ones then a literal: a run word of ones plus one literal.
        let bits: Vec<usize> = (0..64).chain([65]).collect();
        let e = ewah(&bits);
        assert_eq!(&e[4..8], &2u32.to_be_bytes());
        let rlw = u64::from_be_bytes(e[8..16].try_into().unwrap());
        assert_eq!(rlw & 1, 1);
        assert_eq!((rlw >> 1) & ((1 << 32) - 1), 1);
        assert_eq!(rlw >> 33, 1);
    }
}
