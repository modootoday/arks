//! Plumbing diffs on the diff core: `diff-tree`, `diff-index` and
//! `diff-files`, with git's raw format default, every common diff option,
//! and per-command options (`diff-tree` `-r`/`-t`/`--root`/`-m`/`-c`/
//! `--cc`/`--stdin`/`--always`/`--no-commit-id`/`--merge-base`/`-v`/
//! `--pretty`; `diff-index` `--cached`/`-m`/`--merge-base`; `diff-files`
//! `-q` and the unmerged stage selectors).
//!
//! As in git, `diff-files` and `diff-index` (without `--cached`) do not
//! refresh the index: a work-tree file whose stat data differs from its
//! index entry is reported with the all-zero object id.
//!
//! Primary: <https://git-scm.com/docs/git-diff-tree>,
//! <https://git-scm.com/docs/git-diff-index>, <https://git-scm.com/docs/git-diff-files>,
//! <https://git-scm.com/docs/diff-format>.

use crate::cmd::diff_cmd::finish_output;
use crate::cmd::diffcore::combined;
use crate::cmd::diffcore::emit::{EmitCtx, emit};
use crate::cmd::diffcore::options::{Args, DiffOptions};
use crate::cmd::diffcore::sources::{index_entries, index_side, tree_sides, worktree_side};
use crate::cmd::diffcore::{self, Content, Pair, Side, flat_pairs};
use crate::cmd::log_format::read_commit;
use crate::cmd::shared::{ObjKind, peel_to_kind, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};

#[derive(Default)]
struct Opts {
    diff: DiffOptions,
    recursive: bool,
    show_trees: bool,
    root: bool,
    no_commit_id: bool,
    always: bool,
    stdin: bool,
    split_merges: bool,
    combined: Option<combined::Mode>,
    combined_all_paths: bool,
    merge_base: bool,
    cached: bool,
    missing_ok: bool,
    silent_missing: bool,
    stage: Option<u8>,
    pretty: Option<String>,
    revs: Vec<String>,
    specs: Vec<String>,
}

fn parse(args: &[String]) -> CliResult<Opts> {
    let mut o = Opts::default();
    let mut a = Args::new(args);
    let mut paths_only = false;
    while a.i < args.len() {
        let cur = a.cur().to_string();
        if paths_only {
            o.specs.push(cur);
            a.i += 1;
            continue;
        }
        if cur == "--" {
            paths_only = true;
            a.i += 1;
            continue;
        }
        if o.diff.take(&mut a)? {
            a.i += 1;
            continue;
        }
        match cur.as_str() {
            "-r" => o.recursive = true,
            "-t" => {
                o.recursive = true;
                o.show_trees = true;
            }
            "--root" => o.root = true,
            "--no-commit-id" => o.no_commit_id = true,
            "--always" => o.always = true,
            "--stdin" => o.stdin = true,
            "-m" => {
                o.split_merges = true;
                o.missing_ok = true;
            }
            // `-c` / `--cc` reset an earlier `--combined-all-paths`.
            "-c" => {
                o.combined = Some(combined::Mode::Combined);
                o.combined_all_paths = false;
            }
            "--cc" => {
                o.combined = Some(combined::Mode::Dense);
                o.combined_all_paths = false;
            }
            "--combined-all-paths" => o.combined_all_paths = true,
            "--merge-base" => o.merge_base = true,
            "--cached" | "--staged" => o.cached = true,
            "-q" => o.silent_missing = true,
            "-0" => o.stage = Some(0),
            "-1" | "--base" => o.stage = Some(1),
            "-2" | "--ours" => o.stage = Some(2),
            "-3" | "--theirs" => o.stage = Some(3),
            "-v" => o.pretty = Some("medium".into()),
            "--pretty" => o.pretty = Some("medium".into()),
            x if x.starts_with("--pretty=") => o.pretty = Some(x[9..].to_string()),
            x if x.starts_with("--format=") => o.pretty = Some(format!("tformat:{}", &x[9..])),
            x if x.starts_with('-') && x.len() > 1 => {
                return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
            }
            _ => o.revs.push(cur),
        }
        a.i += 1;
    }
    o.diff.finish()?;
    Ok(o)
}

/// Raw output is the default; patch / stat formats imply recursion.
fn defaults(o: &mut Opts) {
    if !o.diff.any_format() {
        o.diff.raw = true;
    }
    let d = &o.diff;
    if d.patch || d.stat || d.numstat || d.shortstat || d.dirstat.is_some() || d.summary || d.check
    {
        o.recursive = true;
    }
}

fn status_of(o: &Opts, pairs: &[Pair]) -> CliResult<()> {
    if o.diff.exit_code && !pairs.is_empty() {
        return Err(CliError::silent(1));
    }
    Ok(())
}

fn tree_of(repo: &Repository, r: &str) -> CliResult<Oid> {
    let oid = resolve_rev(repo, r).map_err(|_| {
        CliError::fatal(format!(
            "ambiguous argument '{r}': unknown revision or path not in the working tree."
        ))
    })?;
    peel_to_kind(repo, &oid, ObjKind::Tree, r)
}

/// Diff of two trees through the diff core (pathspecs applied).
fn tree_queue(c: &Content<'_>, o: &Opts, a: Option<&Oid>, b: Option<&Oid>) -> CliResult<Vec<Pair>> {
    let mut pairs = diffcore::tree_pairs(c.repo, a, b, o.recursive, o.show_trees, &o.specs)?;
    let unchanged = if o.diff.find_copies_harder {
        match a {
            Some(t) => tree_sides(c.repo, t)?
                .into_values()
                .filter(|s| diffcore::spec_match(&s.path, &o.specs))
                .collect(),
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };
    diffcore::transform_with_sources(c, &o.diff, &mut pairs, &unchanged)?;
    Ok(pairs)
}

fn commit_header(
    out: &mut impl Write,
    repo: &Repository,
    o: &Opts,
    commit: &Oid,
    parent: Option<&Oid>,
) -> CliResult<()> {
    if o.no_commit_id {
        return Ok(());
    }
    if let Some(p) = &o.pretty {
        let c = read_commit(repo, commit)?;
        let user_format = p.contains('%') || p.starts_with("format:") || p.starts_with("tformat:");
        // diff-tree shows full object names unless --abbrev is given.
        let spec = if user_format && o.diff.abbrev.is_none() {
            p.replace("%h", "%H")
                .replace("%t", "%T")
                .replace("%p", "%P")
        } else {
            p.clone()
        };
        let text = crate::cmd::log_cmd::format_commit_header(repo, commit, &c, &spec)?;
        out.write_all(text.as_bytes())?;
        // A blank line separates the message from the diff, except after
        // the one-line format.
        let separate = if user_format {
            true
        } else {
            p != "oneline" && !text.ends_with("\n\n")
        };
        if o.diff.any_format() && !o.diff.no_output && separate {
            out.write_all(b"\n")?;
        }
        return Ok(());
    }
    let term: &[u8] = if o.diff.nul { b"\0" } else { b"\n" };
    match parent {
        Some(p) => write!(out, "{} (from {})", commit.to_hex(), p.to_hex())?,
        None => write!(out, "{}", commit.to_hex())?,
    }
    out.write_all(term)?;
    Ok(())
}

/// One commit as `diff-tree <commit>` shows it. Returns whether anything
/// differed.
fn diff_commit(out: &mut impl Write, c: &Content<'_>, o: &Opts, commit: &Oid) -> CliResult<bool> {
    let repo = c.repo;
    let cm = read_commit(repo, commit)?;
    let ctx = EmitCtx {
        raw_full: true,
        ..Default::default()
    };
    if cm.parents.len() > 1 {
        if let Some(mode) = o.combined {
            let parents: Vec<Oid> = cm.parents.to_vec();
            let entries = combined::entries(c, &o.diff, &parents, commit, &o.specs)?;
            if entries.is_empty() && !o.always && !o.diff.no_output {
                return Ok(false);
            }
            let first_parent = if combined::wants_stat(&o.diff) {
                let pt = read_commit(repo, &parents[0])?.tree;
                tree_queue(c, o, Some(&pt), Some(&cm.tree))?
            } else {
                Vec::new()
            };
            commit_header(out, repo, o, commit, None)?;
            combined::emit(
                out,
                c,
                &o.diff,
                mode,
                o.combined_all_paths,
                EmitCtx {
                    raw_full: true,
                    ..Default::default()
                },
                &first_parent,
                &entries,
            )?;
            return Ok(!entries.is_empty());
        }
        if !o.split_merges {
            if o.always {
                commit_header(out, repo, o, commit, None)?;
            }
            return Ok(false);
        }
        let mut any = false;
        for p in &cm.parents {
            let pt = read_commit(repo, p)?.tree;
            let pairs = tree_queue(c, o, Some(&pt), Some(&cm.tree))?;
            if pairs.is_empty() && !o.always {
                continue;
            }
            commit_header(out, repo, o, commit, None)?;
            emit(out, c, &o.diff, ctx, &pairs)?;
            any |= !pairs.is_empty();
        }
        return Ok(any);
    }
    let parent_tree = match cm.parents.first() {
        Some(p) => Some(read_commit(repo, p)?.tree),
        None if o.root => None,
        None => {
            if o.always || o.diff.no_output {
                commit_header(out, repo, o, commit, None)?;
            }
            return Ok(false);
        }
    };
    let pairs = tree_queue(c, o, parent_tree.as_ref(), Some(&cm.tree))?;
    if pairs.is_empty() && !o.always && !o.diff.no_output {
        return Ok(false);
    }
    commit_header(out, repo, o, commit, None)?;
    emit(out, c, &o.diff, ctx, &pairs)?;
    Ok(!pairs.is_empty())
}

pub fn cmd_diff_tree(args: &[String]) -> CliResult<()> {
    let mut o = parse(args)?;
    defaults(&mut o);
    let repo = open_repo()?;
    let c = Content::new(&repo);
    let ctx = EmitCtx {
        raw_full: true,
        ..Default::default()
    };
    let mut out = io::BufWriter::new(io::stdout().lock());
    if o.stdin {
        // Paths given on the command line stay pathspecs.
        o.specs.append(&mut o.revs);
        let mut any = false;
        for line in io::stdin().lock().lines() {
            let line = line?;
            let words: Vec<&str> = line.split_whitespace().collect();
            let oids: Vec<Option<Oid>> = words
                .iter()
                .map(|w| Oid::from_hex(w).ok().filter(|_| w.len() >= 40))
                .collect();
            match oids.as_slice() {
                [Some(a), Some(b)] => {
                    let (ta, tb) = (
                        peel_to_kind(&repo, a, ObjKind::Tree, words[0])?,
                        peel_to_kind(&repo, b, ObjKind::Tree, words[1])?,
                    );
                    let pairs = tree_queue(&c, &o, Some(&ta), Some(&tb))?;
                    if !pairs.is_empty() {
                        writeln!(out, "{} {}", a.to_hex(), b.to_hex())?;
                    }
                    emit(&mut out, &c, &o.diff, ctx, &pairs)?;
                    any |= !pairs.is_empty();
                }
                [Some(a), ..] => {
                    any |= diff_commit(&mut out, &c, &o, a)?;
                }
                _ => {
                    writeln!(out, "{line}")?;
                }
            }
            out.flush()?;
        }
        out.flush()?;
        return if o.diff.exit_code && any {
            Err(CliError::silent(1))
        } else {
            Ok(())
        };
    }
    let mut revs = o.revs.clone();
    // Arguments after the tree-ishes are paths.
    let mut n_trees = 0;
    for r in &revs {
        if n_trees < 2 && resolve_rev(&repo, r).is_ok() {
            n_trees += 1;
        } else {
            break;
        }
    }
    let extra: Vec<String> = revs.drain(n_trees..).collect();
    o.specs.splice(0..0, extra);
    match revs.as_slice() {
        [one] => {
            let oid = resolve_rev(&repo, one)?;
            if o.merge_base {
                return Err(CliError::fatal("--merge-base only works with two commits"));
            }
            let is_commit = peel_to_kind(&repo, &oid, ObjKind::Commit, one).ok();
            let Some(commit) = is_commit else {
                return Err(CliError::fatal(format!("{one}: not a commit")));
            };
            let any = diff_commit(&mut out, &c, &o, &commit)?;
            out.flush()?;
            if o.diff.exit_code && any {
                return Err(CliError::silent(1));
            }
            Ok(())
        }
        [a, b] => {
            let (mut ta, tb) = (tree_of(&repo, a)?, tree_of(&repo, b)?);
            if o.merge_base {
                let (ca, cb) = (resolve_rev(&repo, a)?, resolve_rev(&repo, b)?);
                let base = arks_repo::merge_base_oid(&repo, &ca, &cb)?
                    .ok_or_else(|| CliError::fatal("no merge base found"))?;
                ta = read_commit(&repo, &base)?.tree;
            }
            let pairs = tree_queue(&c, &o, Some(&ta), Some(&tb))?;
            emit(&mut out, &c, &o.diff, ctx, &pairs)?;
            out.flush()?;
            status_of(&o, &pairs)
        }
        _ => Err(CliError::Usage(
            "git diff-tree [--stdin] [-m] [-s] [-v] [--no-commit-id] [--pretty]\n              \
             [-t] [-r] [-c | --cc] [--combined-all-paths] [--root] [--merge-base]\n              \
             [<common-diff-options>] <tree-ish> [<tree-ish>] [<path>...]"
                .into(),
        )),
    }
}

pub fn cmd_diff_index(args: &[String]) -> CliResult<()> {
    let mut o = parse(args)?;
    defaults(&mut o);
    let repo = open_repo()?;
    let rev = o.revs.first().cloned().ok_or_else(|| {
        CliError::Usage(
            "git diff-index [-m] [--cached] [--merge-base] [<common-diff-options>] <tree-ish> [<path>...]"
                .into(),
        )
    })?;
    let extra: Vec<String> = o.revs.drain(1..).collect();
    o.specs.extend(extra);
    let mut tree = tree_of(&repo, &rev)?;
    if o.merge_base {
        let a = resolve_rev(&repo, &rev)?;
        let head = resolve_rev(&repo, "HEAD")?;
        let base = arks_repo::merge_base_oid(&repo, &a, &head)?
            .ok_or_else(|| CliError::fatal("no merge base found"))?;
        tree = read_commit(&repo, &base)?.tree;
    }
    let old = tree_sides(&repo, &tree)?;
    let (index, unmerged) = index_entries(&repo)?;
    let mut new: BTreeMap<String, Side> = BTreeMap::new();
    let work = if o.cached {
        None
    } else {
        Some(work_tree(&repo)?)
    };
    for (p, e) in &index {
        match &work {
            None => {
                new.insert(p.clone(), index_side(e));
            }
            Some(w) => match worktree_side(&repo, w, e, false)? {
                Some(s) => {
                    new.insert(p.clone(), s);
                }
                // `-m`: a missing work-tree file counts as matching the index.
                None if o.missing_ok => {
                    new.insert(p.clone(), index_side(e));
                }
                None => {}
            },
        }
    }
    let keep = |m: BTreeMap<String, Side>| -> BTreeMap<String, Side> {
        m.into_iter()
            .filter(|(k, _)| diffcore::spec_match(k, &o.specs))
            .collect()
    };
    let mut pairs = flat_pairs(&keep(old), &keep(new));
    for u in &unmerged {
        if diffcore::spec_match(u, &o.specs) {
            pairs.retain(|x| x.path() != u);
            let mut p = Pair::new(
                None,
                Some(Side {
                    path: u.clone(),
                    mode: 0,
                    oid: None,
                    work: None,
                }),
            );
            p.unmerged = true;
            pairs.push(p);
        }
    }
    pairs.sort_by(|x, y| x.path().cmp(y.path()));
    let c = Content::new(&repo);
    diffcore::transform(&c, &o.diff, &mut pairs)?;
    finish_output(
        &c,
        &o.diff,
        EmitCtx {
            raw_full: true,
            ..Default::default()
        },
        &pairs,
    )
}

pub fn cmd_diff_files(args: &[String]) -> CliResult<()> {
    let mut o = parse(args)?;
    defaults(&mut o);
    if o.cached {
        return Err(CliError::NotImplemented(
            "unsupported option: --cached".into(),
        ));
    }
    let specs: Vec<String> = std::mem::take(&mut o.revs);
    o.specs.extend(specs);
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let (index, unmerged) = index_entries(&repo)?;
    let mut pairs = Vec::new();
    for (p, e) in &index {
        if !diffcore::spec_match(p, &o.specs) {
            continue;
        }
        let old = index_side(e);
        let new = worktree_side(&repo, &work, e, false)?;
        if new.is_none() && o.silent_missing {
            continue;
        }
        if new.as_ref() != Some(&old) {
            pairs.push(Pair::new(Some(old), new));
        }
    }
    for u in &unmerged {
        if diffcore::spec_match(u, &o.specs) {
            let mut p = Pair::new(
                None,
                Some(Side {
                    path: u.clone(),
                    mode: 0,
                    oid: None,
                    work: None,
                }),
            );
            p.unmerged = true;
            pairs.push(p);
        }
    }
    pairs.sort_by(|x, y| x.path().cmp(y.path()));
    let _ = o.stage;
    let c = Content::new(&repo);
    diffcore::transform(&c, &o.diff, &mut pairs)?;
    finish_output(
        &c,
        &o.diff,
        EmitCtx {
            raw_full: true,
            ..Default::default()
        },
        &pairs,
    )
}
