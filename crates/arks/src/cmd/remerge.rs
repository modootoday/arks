//! `log --remerge-diff` / `--diff-merges=remerge`: re-merge a two-parent
//! merge's parents in memory (conflict markers labelled
//! `<abbrev> (<subject>)`) and diff that result against the recorded merge
//! tree, so the output shows how conflicts were resolved. Nothing is written
//! to the object store, the index or the work tree.
//! Primary: <https://git-scm.com/docs/git-log#Documentation/git-log.txt---remerge-diff>.
//!
//! Partial: octopus merges and merges without a merge base show no diff;
//! criss-cross merges use one merge base instead of a recursive one.

use crate::cmd::diffcore::options::DiffOptions;
use crate::cmd::diffcore::{self, Content, Pair, Side};
use crate::cmd::log_format::{read_commit, subject_line};
use crate::cmd::merge_engine::{MergeLabels, remerge_files};
use crate::error::CliResult;
use arks_object::{Commit, ObjectKind, RawObject};
use arks_repo::merge_base_oid;

/// Pairs (re-merge result → recorded merge) and the conflict messages.
pub fn remerge_pairs(
    content: &Content<'_>,
    o: &DiffOptions,
    c: &Commit,
    specs: &[String],
) -> CliResult<(Vec<Pair>, Vec<(String, String)>)> {
    let repo = content.repo;
    let [p1, p2] = c.parents.as_slice() else {
        return Ok((Vec::new(), Vec::new()));
    };
    let Some(base) = merge_base_oid(repo, p1, p2)? else {
        return Ok((Vec::new(), Vec::new()));
    };
    let label = |oid: &arks_oid::Oid| -> CliResult<String> {
        let pc = read_commit(repo, oid)?;
        Ok(format!(
            "{} ({})",
            crate::cmd::shared::abbrev_hex(repo, oid, 7),
            subject_line(&pc.message)
        ))
    };
    let labels = MergeLabels {
        ours: label(p1)?,
        theirs: label(p2)?,
        base: "merged common ancestors".into(),
        ..MergeLabels::default()
    };
    let (bt, ot, tt) = (
        read_commit(repo, &base)?.tree,
        read_commit(repo, p1)?.tree,
        read_commit(repo, p2)?.tree,
    );
    let (files, msgs) = remerge_files(repo, &bt, &ot, &tt, &labels)?;
    let actual = diffcore::flat_tree(repo, &c.tree)?;
    let mut paths: std::collections::BTreeSet<String> = files.keys().cloned().collect();
    paths.extend(actual.keys().cloned());
    let mut pairs = Vec::new();
    for path in paths {
        if !diffcore::spec_match(&path, specs) {
            continue;
        }
        let old = match files.get(&path) {
            Some(f) => {
                let oid = match (&f.oid, &f.bytes) {
                    (Some(o), _) => *o,
                    (None, Some(b)) => {
                        let oid = RawObject::new(ObjectKind::Blob, b.clone())
                            .oid(repo.object_format())?;
                        content.seed(oid, b.clone());
                        oid
                    }
                    (None, None) => continue,
                };
                Some(Side::blob(&path, f.mode, oid))
            }
            None => None,
        };
        let new = actual
            .get(&path)
            .map(|(mode, oid)| Side::blob(&path, *mode, *oid));
        if old.as_ref().map(|s| (s.mode, s.oid)) == new.as_ref().map(|s| (s.mode, s.oid)) {
            continue;
        }
        pairs.push(Pair::new(old, new));
    }
    diffcore::transform(content, o, &mut pairs)?;
    Ok((pairs, msgs))
}

/// Insert `remerge <message>` after the `diff --git` line of each path.
pub fn annotate(text: &str, msgs: &[(String, String)]) -> String {
    if msgs.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    for line in text.split_inclusive('\n') {
        out.push_str(line);
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let rest = rest.trim_end_matches('\n');
            for (path, m) in msgs {
                if rest.ends_with(&format!(" b/{path}")) {
                    out.push_str(&format!("remerge {m}\n"));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotates_headers() {
        let t = "diff --git a/x b/x\nindex 1..2\n";
        let m = vec![(
            "x".to_string(),
            "CONFLICT (content): Merge conflict in x".to_string(),
        )];
        assert_eq!(
            annotate(t, &m),
            "diff --git a/x b/x\nremerge CONFLICT (content): Merge conflict in x\nindex 1..2\n"
        );
    }
}
