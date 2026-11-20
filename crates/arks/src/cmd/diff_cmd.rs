//! `git diff` porcelain on the diff core: work tree vs index (default),
//! `--cached [<commit>]`, `<commit>` vs work tree, `<commit> <commit>`,
//! `A..B`, `A...B`, `--merge-base`, and `--no-index <path> <path>`.
//!
//! Porcelain defaults: rename detection on (`diff.renames`), the patch
//! format when none is chosen, `diff.context`, `diff.algorithm`,
//! `diff.noprefix` and `color.diff` / `color.ui`.
//!
//! Primary: <https://git-scm.com/docs/git-diff>, <https://git-scm.com/docs/diff-options>,
//! <https://git-scm.com/docs/git-config> (`diff.*`).

use crate::cmd::diffcore::emit::{EmitCtx, emit};
use crate::cmd::diffcore::options::{Detect, DiffOptions};
use crate::cmd::diffcore::sources::{index_entries, index_side, tree_sides, worktree_side};
use crate::cmd::diffcore::{self, Content, Pair, Side, flat_pairs};
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::Path;

/// Config-driven porcelain defaults (applied before explicit options win).
pub fn porcelain_defaults(repo: Option<&Repository>, o: &mut DiffOptions) {
    let cfg = match repo {
        Some(r) => arks_config::ConfigSet::load(Some(r.common_dir())),
        None => arks_config::ConfigSet::load(None),
    };
    match cfg.get("diff.renames").map(|v| v.to_ascii_lowercase()) {
        Some(v) if v == "copies" || v == "copy" => o.detect = Detect::Copies,
        Some(v) if matches!(v.as_str(), "false" | "no" | "off" | "0") => o.detect = Detect::Off,
        _ => o.detect = Detect::Renames,
    }
    if let Some(n) = cfg.get("diff.context").and_then(|v| v.parse().ok()) {
        o.context = Some(n);
    }
    if let Some(a) = cfg
        .get("diff.algorithm")
        .and_then(|v| crate::cmd::diffcore::xdiff::Algo::parse(&v))
    {
        o.line.algo = a;
    }
    if cfg.get_bool("diff.noprefix") == Some(true) {
        o.src_prefix.clear();
        o.dst_prefix.clear();
    }
    if let Some(n) = cfg.get("diff.renamelimit").and_then(|v| v.parse().ok()) {
        o.rename_limit = Some(n);
    }
    let color = cfg.get("color.diff").or_else(|| cfg.get("color.ui"));
    if let Some(c) = color {
        if matches!(c.to_ascii_lowercase().as_str(), "always") {
            o.color = Some(true);
        }
    }
}

/// Parse diff options on top of porcelain defaults: explicit options are
/// applied after the config so they win.
pub fn parse_porcelain(
    repo: Option<&Repository>,
    args: &[String],
) -> CliResult<(DiffOptions, Vec<String>)> {
    let mut o = DiffOptions::default();
    porcelain_defaults(repo, &mut o);
    let rest = o.parse_all(args)?;
    o.finish();
    Ok((o, rest))
}

/// Worktree-relative pathspec (cwd-relative input), `/`-separated.
fn pathspec(work: &Path, p: &str) -> String {
    let abs = std::env::current_dir()
        .map(|c| c.join(p))
        .unwrap_or_else(|_| std::path::PathBuf::from(p));
    let rel = abs
        .strip_prefix(work)
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| p.replace('\\', "/"));
    let rel = rel
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    if rel == "." { String::new() } else { rel }
}

/// Current directory relative to the work tree (`--relative` default).
pub fn cwd_prefix(work: &Path) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|c| {
            c.strip_prefix(work)
                .ok()
                .map(|r| r.to_string_lossy().replace('\\', "/"))
        })
        .unwrap_or_default()
}

fn keep_specs(map: BTreeMap<String, Side>, specs: &[String]) -> BTreeMap<String, Side> {
    if specs.is_empty() {
        return map;
    }
    map.into_iter()
        .filter(|(k, _)| diffcore::spec_match(k, specs))
        .collect()
}

/// Write the queue (to `--output` when given) and compute the exit status.
pub fn finish_output(
    c: &Content<'_>,
    o: &DiffOptions,
    ctx: EmitCtx,
    pairs: &[Pair],
) -> CliResult<()> {
    if !o.quiet {
        if let Some(path) = &o.output {
            let mut f = io::BufWriter::new(std::fs::File::create(path).map_err(|e| {
                CliError::fatal(format!("could not open '{path}' for writing: {e}"))
            })?);
            emit(&mut f, c, o, ctx, pairs)?;
            f.flush()?;
        } else {
            let mut out = io::BufWriter::with_capacity(256 * 1024, io::stdout().lock());
            emit(&mut out, c, o, ctx, pairs)?;
            out.flush()?;
        }
    }
    if o.check && crate::cmd::diffcore::emit::check_failed() {
        return Err(CliError::silent(if o.exit_code && !pairs.is_empty() {
            3
        } else {
            2
        }));
    }
    if o.exit_code && !pairs.is_empty() {
        return Err(CliError::silent(1));
    }
    Ok(())
}

pub fn cmd_diff(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-diff
    let repo = open_repo().ok();
    let (mut o, rest) = parse_porcelain(repo.as_ref(), args)?;
    let mut cached = false;
    let mut no_index = false;
    let mut merge_base = false;
    let mut positional = Vec::new();
    let mut paths_raw: Vec<String> = Vec::new();
    let mut after_dd = false;
    for a in &rest {
        if after_dd {
            paths_raw.push(a.clone());
            continue;
        }
        match a.as_str() {
            "--" => after_dd = true,
            "--cached" | "--staged" => cached = true,
            "--no-index" => no_index = true,
            "--merge-base" => merge_base = true,
            "-1" | "--base" | "-2" | "--ours" | "-3" | "--theirs" | "-0" => {}
            x if x.starts_with('-') && x.len() > 1 => {
                return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
            }
            _ => positional.push(a.clone()),
        }
    }
    if !o.any_format() {
        o.patch = true;
    }
    if no_index || repo.is_none() {
        let mut files = positional.clone();
        files.extend(paths_raw.iter().cloned());
        return diff_no_index(&files, &o);
    }
    let repo = repo.unwrap_or_else(|| unreachable!());
    let work = work_tree(&repo)?;
    let mut revs: Vec<Oid> = Vec::new();
    let mut specs: Vec<String> = paths_raw.iter().map(|p| pathspec(&work, p)).collect();
    let mut symmetric = false;
    for p in &positional {
        if let Some((l, r)) = p.split_once("...") {
            let lo = resolve_rev(&repo, if l.is_empty() { "HEAD" } else { l })?;
            let ro = resolve_rev(&repo, if r.is_empty() { "HEAD" } else { r })?;
            let base = arks_repo::merge_base_oid(&repo, &lo, &ro)?
                .ok_or_else(|| CliError::Msg(format!("{p}: no merge base")))?;
            revs.push(base);
            revs.push(ro);
            symmetric = true;
        } else if let Some((l, r)) = p.split_once("..") {
            revs.push(resolve_rev(&repo, if l.is_empty() { "HEAD" } else { l })?);
            revs.push(resolve_rev(&repo, if r.is_empty() { "HEAD" } else { r })?);
        } else if let Ok(o) = resolve_rev(&repo, p) {
            revs.push(o);
        } else if work.join(p).exists() || Path::new(p).exists() {
            specs.push(pathspec(&work, p));
        } else {
            return Err(CliError::fatal(format!(
                "ambiguous argument '{p}': unknown revision or path not in the working tree.\n\
                 Use '--' to separate paths from revisions, like this:\n\
                 'git <command> [<revision>...] -- [<file>...]'"
            )));
        }
    }
    let _ = symmetric;
    if merge_base {
        match revs.len() {
            1 => {
                let head = resolve_rev(&repo, "HEAD")?;
                revs[0] = arks_repo::merge_base_oid(&repo, &revs[0], &head)?
                    .ok_or_else(|| CliError::fatal("no merge base found"))?;
            }
            2 => {
                revs[0] = arks_repo::merge_base_oid(&repo, &revs[0], &revs[1])?
                    .ok_or_else(|| CliError::fatal("no merge base found"))?;
            }
            _ => return Err(CliError::fatal("--merge-base requires a commit")),
        }
    }
    if let Some(r) = o.relative.as_mut() {
        if r.is_empty() {
            *r = cwd_prefix(&work);
        }
    }
    let tree_of = |oid: &Oid, name: &str| -> CliResult<Oid> {
        crate::cmd::shared::peel_to_kind(&repo, oid, crate::cmd::shared::ObjKind::Tree, name)
    };
    let (index, unmerged) = index_entries(&repo)?;
    let index_map = || -> BTreeMap<String, Side> {
        index
            .iter()
            .map(|(p, e)| (p.clone(), index_side(e)))
            .collect()
    };
    let worktree_map = || -> CliResult<BTreeMap<String, Side>> {
        let mut m = BTreeMap::new();
        for (p, e) in &index {
            if let Some(s) = worktree_side(&repo, &work, e, true)? {
                m.insert(p.clone(), s);
            }
        }
        Ok(m)
    };
    let empty_tree = || -> BTreeMap<String, Side> { BTreeMap::new() };
    let head_tree = || -> CliResult<BTreeMap<String, Side>> {
        match repo.head_oid() {
            Ok(h) => tree_sides(&repo, &tree_of(&h, "HEAD")?),
            Err(_) => Ok(empty_tree()),
        }
    };
    let (a, b) = match (revs.len(), cached) {
        (0, true) => (head_tree()?, index_map()),
        (0, false) => (index_map(), worktree_map()?),
        (1, true) => (tree_sides(&repo, &tree_of(&revs[0], "rev")?)?, index_map()),
        (1, false) => (
            tree_sides(&repo, &tree_of(&revs[0], "rev")?)?,
            worktree_map()?,
        ),
        (2, _) => (
            tree_sides(&repo, &tree_of(&revs[0], "rev")?)?,
            tree_sides(&repo, &tree_of(&revs[1], "rev")?)?,
        ),
        _ => {
            return Err(CliError::Usage(
                "diff [--cached] [<commit> [<commit>]] [-- <path>...]".into(),
            ));
        }
    };
    let unchanged: Vec<Side> = if o.find_copies_harder {
        a.iter()
            .filter(|(k, s)| b.get(*k) == Some(*s) && diffcore::spec_match(k, &specs))
            .map(|(_, s)| s.clone())
            .collect()
    } else {
        Vec::new()
    };
    let (a, b) = (keep_specs(a, &specs), keep_specs(b, &specs));
    let mut pairs = flat_pairs(&a, &b);
    if revs.is_empty() && !cached {
        for p in &unmerged {
            if diffcore::spec_match(p, &specs) {
                pairs.retain(|x| x.path() != p);
                let mut u = Pair::new(None, None);
                u.unmerged = true;
                u.new = Some(Side {
                    path: p.clone(),
                    mode: 0,
                    oid: None,
                    work: None,
                });
                pairs.push(u);
            }
        }
        pairs.sort_by(|x, y| x.path().cmp(y.path()));
    }
    let c = Content::new(&repo);
    diffcore::transform_with_sources(&c, &o, &mut pairs, &unchanged)?;
    finish_output(&c, &o, EmitCtx::default(), &pairs)
}

/// `diff --no-index <path> <path>`: compare two files or directories on
/// disk; exits 1 when they differ.
fn diff_no_index(files: &[String], o: &DiffOptions) -> CliResult<()> {
    let [a, b] = files else {
        return Err(CliError::Usage(
            "git diff --no-index [<options>] <path> <path>".into(),
        ));
    };
    let fmt = arks_store::ObjectFormat::Sha1;
    let side = |p: &str| -> CliResult<BTreeMap<String, Side>> {
        let mut m = BTreeMap::new();
        let path = Path::new(p);
        let mut stack = vec![(path.to_path_buf(), String::new())];
        let is_dir = path.is_dir();
        while let Some((abs, rel)) = stack.pop() {
            let meta = std::fs::symlink_metadata(&abs).map_err(|e| {
                CliError::fatal(format!("Could not access '{}': {e}", abs.display()))
            })?;
            if meta.is_dir() {
                for ent in std::fs::read_dir(&abs)? {
                    let ent = ent?;
                    let name = ent.file_name().to_string_lossy().into_owned();
                    let r = if rel.is_empty() {
                        name
                    } else {
                        format!("{rel}/{name}")
                    };
                    stack.push((ent.path(), r));
                }
                continue;
            }
            let mode = if meta.file_type().is_symlink() {
                diffcore::MODE_LINK
            } else {
                crate::cmd::shared::file_mode(&meta)
            };
            let bytes = if mode == diffcore::MODE_LINK {
                std::fs::read_link(&abs)?
                    .to_string_lossy()
                    .into_owned()
                    .into_bytes()
            } else {
                std::fs::read(&abs)?
            };
            let oid = arks_object::RawObject::new(arks_object::ObjectKind::Blob, bytes).oid(fmt)?;
            let key = if is_dir { rel.clone() } else { String::new() };
            m.insert(
                key,
                Side {
                    path: abs.to_string_lossy().trim_start_matches("./").to_string(),
                    mode,
                    oid: Some(oid),
                    work: Some(abs.clone()),
                },
            );
        }
        Ok(m)
    };
    let (ma, mb) = (side(a)?, side(b)?);
    let mut pairs = Vec::new();
    let mut keys: Vec<&String> = ma.keys().chain(mb.keys()).collect();
    keys.sort();
    keys.dedup();
    for k in keys {
        let (x, y) = (ma.get(k), mb.get(k));
        if x.is_some() && y.is_some() && x.map(|s| (s.mode, s.oid)) == y.map(|s| (s.mode, s.oid)) {
            continue;
        }
        let p = Pair::new(x.cloned(), y.cloned());
        pairs.push(p);
    }
    // Without a repository the content cache needs one only for
    // abbreviations; compute them from content.
    let mut out = io::BufWriter::new(io::stdout().lock());
    if !o.quiet {
        no_index_emit(&mut out, o, &pairs)?;
    }
    out.flush()?;
    if pairs.is_empty() {
        return Ok(());
    }
    Err(CliError::silent(1))
}

/// Patch output for `--no-index` (no repository: plain abbreviations).
fn no_index_emit(out: &mut impl Write, o: &DiffOptions, pairs: &[Pair]) -> CliResult<()> {
    for p in pairs {
        let read = |s: &Option<Side>| -> Vec<u8> {
            s.as_ref()
                .and_then(|s| s.work.as_ref())
                .and_then(|w| std::fs::read(w).ok())
                .unwrap_or_default()
        };
        let (x, y) = (read(&p.old), read(&p.new));
        let name = |s: &Option<Side>| s.as_ref().map(|s| s.path.clone()).unwrap_or_default();
        let (na, nb) = (name(&p.old), name(&p.new));
        let na = if na.is_empty() { nb.clone() } else { na };
        let nb = if nb.is_empty() { na.clone() } else { nb };
        if o.name_only {
            writeln!(out, "{nb}")?;
            continue;
        }
        if o.name_status {
            writeln!(out, "{}\t{nb}", p.status())?;
            continue;
        }
        crate::cmd::unified_diff::write_file_patch_paths(
            out,
            arks_store::ObjectFormat::Sha1,
            (&na, &nb),
            p.old.as_ref().map(|_| x.as_slice()),
            p.new.as_ref().map(|_| y.as_slice()),
            o.context(),
        )?;
    }
    Ok(())
}
