//! `fetch` / `push` against a local repository (plain path or same-host
//! `file://`): refspec matching, tag auto-following, pruning, fast-forward
//! checks, ref updates and the human / `--porcelain` reports, on top of the
//! object copy in `arks_remote::copy_objects`.
//!
//! Primary: <https://git-scm.com/docs/git-fetch>, <https://git-scm.com/docs/git-push>,
//! <https://git-scm.com/docs/git-config#Documentation/git-config.txt-receivedenyCurrentBranch>.
//! Output shapes (`From`, `To`, summary column, porcelain flags) follow the
//! OUTPUT sections of those pages and observed stock git output.

use crate::cmd::shared::{abbrev_hex, is_ancestor};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::fs;
use std::io::Write;

/// One `[+|^]<src>[:<dst>]` refspec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Refspec {
    pub force: bool,
    pub negative: bool,
    pub src: String,
    pub dst: Option<String>,
}

pub(crate) fn parse_refspec(s: &str) -> Refspec {
    let (negative, s) = match s.strip_prefix('^') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let (force, rest) = match s.strip_prefix('+') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let (src, dst) = match rest.split_once(':') {
        Some((a, b)) => (a.to_string(), Some(b.to_string())),
        None => (rest.to_string(), None),
    };
    Refspec {
        force,
        negative,
        src,
        dst,
    }
}

impl Refspec {
    pub fn is_glob(&self) -> bool {
        self.src.contains('*')
    }

    /// For a glob spec: the `*` part of `name` when it matches `src`.
    fn glob_mid<'a>(pat: &str, name: &'a str) -> Option<&'a str> {
        let (p, s) = pat.split_once('*')?;
        (name.len() >= p.len() + s.len() && name.starts_with(p) && name.ends_with(s))
            .then(|| &name[p.len()..name.len() - s.len()])
    }

    /// Map a source ref through this spec to its destination.
    pub fn map_src(&self, name: &str) -> Option<String> {
        let dst = self.dst.as_deref().filter(|d| !d.is_empty())?;
        if self.is_glob() {
            let mid = Self::glob_mid(&self.src, name)?;
            let (dp, ds) = dst.split_once('*')?;
            Some(format!("{dp}{mid}{ds}"))
        } else if self.src == name {
            Some(dst.to_string())
        } else {
            None
        }
    }

    /// Map a destination ref back to its source (prune).
    pub fn map_dst(&self, name: &str) -> Option<String> {
        let dst = self.dst.as_deref().filter(|d| !d.is_empty())?;
        if dst.contains('*') {
            let mid = Self::glob_mid(dst, name)?;
            let (sp, ss) = self.src.split_once('*')?;
            Some(format!("{sp}{mid}{ss}"))
        } else {
            None
        }
    }

    /// Negative-refspec exclusion (`^refs/heads/x` or a glob).
    pub fn excludes(&self, name: &str) -> bool {
        self.negative
            && if self.is_glob() {
                Self::glob_mid(&self.src, name).is_some()
            } else {
                self.src == name
            }
    }
}

/// DWIM a short source name against `refs` (exact, heads, tags, remotes).
pub(crate) fn dwim<'a>(name: &str, refs: &'a [(String, Oid)]) -> Option<&'a (String, Oid)> {
    let cands = [
        name.to_string(),
        format!("refs/{name}"),
        format!("refs/tags/{name}"),
        format!("refs/heads/{name}"),
        format!("refs/remotes/{name}"),
        format!("refs/remotes/{name}/HEAD"),
    ];
    cands.iter().find_map(|c| refs.iter().find(|(n, _)| n == c))
}

/// git's prettified ref name for reports.
pub(crate) fn short_ref(name: &str) -> &str {
    ["refs/heads/", "refs/tags/", "refs/remotes/"]
        .iter()
        .find_map(|p| name.strip_prefix(p))
        .unwrap_or(name)
}

/// URL as fetch reports and records it: trailing `/` and one `.git` removed.
pub(crate) fn display_url(url: &str) -> String {
    let t = url.trim_end_matches('/');
    let t = match t.strip_suffix(".git") {
        Some(s) if !s.is_empty() => s,
        _ => t,
    };
    if t.is_empty() { url } else { t }.to_string()
}

/// Destination ref for `<dst>`: full refs pass through, else the source's namespace.
pub(crate) fn qualify_dst(dst: &str, src_full: &str) -> String {
    if dst.starts_with("refs/") || dst == "HEAD" {
        dst.to_string()
    } else if src_full.starts_with("refs/tags/") {
        format!("refs/tags/{dst}")
    } else {
        format!("refs/heads/{dst}")
    }
}

/// Branch `HEAD` of a non-bare repository points at, if any.
pub(crate) fn checked_out_branch(repo: &Repository) -> Option<String> {
    if repo.kind() == arks_repo::GitDirKind::Bare {
        return None;
    }
    match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => Some(s),
        _ => None,
    }
}

/// `HEAD` plus every ref under `refs/`, as a peer advertises them.
pub(crate) fn advertised_refs(src: &Repository) -> CliResult<Vec<(String, Oid)>> {
    let mut refs = src.refs().list_refs("refs/")?;
    if let Ok(h) = src.head_oid() {
        refs.insert(0, ("HEAD".to_string(), h));
    }
    Ok(refs)
}

/// Delete `name`, rewriting `packed-refs` when the ref is (also) packed.
/// Cite: <https://git-scm.com/docs/gitrepository-layout> (`packed-refs`).
pub(crate) fn delete_ref_any(repo: &mut Repository, name: &str) -> CliResult<()> {
    let loose = repo.common_dir().join(name);
    if loose.is_file() {
        repo.refs_mut().delete(name, None)?;
    }
    let packed = repo.common_dir().join("packed-refs");
    let Ok(text) = fs::read_to_string(&packed) else {
        return Ok(());
    };
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    let mut changed = false;
    for line in text.split_inclusive('\n') {
        if line.starts_with('^') {
            if !skipping {
                out.push_str(line);
            }
            continue;
        }
        skipping = line
            .trim_end()
            .split_once(' ')
            .is_some_and(|(_, n)| n == name);
        if skipping {
            changed = true;
        } else {
            out.push_str(line);
        }
    }
    if changed {
        let lock = repo.common_dir().join("packed-refs.lock");
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
            .map_err(|e| CliError::fatal(format!("Unable to create '{}': {e}", lock.display())))?;
        f.write_all(out.as_bytes())?;
        drop(f);
        fs::rename(&lock, &packed)?;
    }
    Ok(())
}

/// Annotated tag target (`object <hex>` header).
fn tag_target(payload: &[u8]) -> Option<Oid> {
    String::from_utf8_lossy(payload)
        .lines()
        .take_while(|l| !l.is_empty())
        .find_map(|l| l.strip_prefix("object "))
        .and_then(|h| Oid::from_hex(h.trim()).ok())
}

pub(crate) fn peel(repo: &Repository, oid: &Oid) -> Oid {
    let mut cur = *oid;
    while let Ok(obj) = repo.get_object(&cur) {
        match (obj.kind, tag_target(&obj.payload)) {
            (arks_object::ObjectKind::Tag, Some(next)) => cur = next,
            _ => break,
        }
    }
    cur
}

fn is_commit(repo: &Repository, oid: &Oid) -> bool {
    repo.get_object(oid)
        .is_ok_and(|o| o.kind == arks_object::ObjectKind::Commit)
}

/// The source repository of a fetch: a named remote or a bare URL.
pub(crate) struct FetchSource {
    pub name: Option<String>,
    pub url: String,
    pub config_specs: Vec<String>,
    /// `remote.<name>.tagOpt` (`--tags` / `--no-tags`).
    pub tag_opt: Option<String>,
}

#[derive(Default, Clone)]
pub(crate) struct LocalFetchOpts {
    pub verbose: bool,
    pub quiet: bool,
    pub porcelain: bool,
    pub dry_run: bool,
    pub append: bool,
    pub write_fetch_head: bool,
    pub force: bool,
    /// `Some(true)` = `--tags`, `Some(false)` = `--no-tags`.
    pub tags: Option<bool>,
    pub prune: bool,
    pub prune_tags: bool,
    pub update_head_ok: bool,
    pub atomic: bool,
    pub refmap: Option<Vec<String>>,
    pub prefetch: bool,
    pub set_upstream: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Spec,
    Tag,
    Opportunistic,
}

#[derive(Clone)]
struct Entry {
    src: String,
    oid: Oid,
    dst: Option<String>,
    force: bool,
    merge: bool,
    origin: Origin,
}

/// Fetch report line state.
enum Outcome {
    FetchHeadOnly,
    UpToDate,
    New,
    FastForward(Oid),
    Forced(Oid),
    TagUpdate(Oid),
    Rejected(Option<Oid>, &'static str),
}

/// git's ref-column width: at least 10, widened to the longest source name.
fn refcol(names: impl Iterator<Item = usize>) -> usize {
    names.fold(10, usize::max)
}

fn kind_of(name: &str) -> &'static str {
    if name.starts_with("refs/tags/") {
        "tag"
    } else if name.starts_with("refs/heads/") || name == "HEAD" {
        "branch"
    } else {
        "ref"
    }
}

/// `fetch` from a local repository. Returns `Err(Exit(1))` when a ref update
/// was rejected (after reporting), as git does.
pub(crate) fn fetch_local(
    source: &FetchSource,
    cmdline: &[String],
    o: &LocalFetchOpts,
) -> CliResult<()> {
    let url = source.url.as_str();
    let src = arks_remote::open_local_remote(url).map_err(|e| CliError::fatal(e.to_string()))?;
    let mut repo = open_repo()?;
    let remote_refs = advertised_refs(&src)?;
    let explicit = !cmdline.is_empty();
    let mut specs: Vec<Refspec> = if explicit {
        cmdline.iter().map(|s| parse_refspec(s)).collect()
    } else if source.name.is_some() && !source.config_specs.is_empty() {
        source
            .config_specs
            .iter()
            .map(|s| parse_refspec(s))
            .collect()
    } else {
        vec![parse_refspec("HEAD")]
    };
    let tag_opt = source.tag_opt.as_deref();
    let tags_mode = o.tags.or(match tag_opt {
        Some("--tags") => Some(true),
        Some("--no-tags") => Some(false),
        _ => None,
    });
    if o.prefetch {
        // git-fetch --prefetch: store under refs/prefetch/, never into tags.
        specs.retain(|s| s.negative || s.dst.as_deref().is_some_and(|d| !d.is_empty()));
        specs.retain(|s| s.negative || !s.dst.as_deref().unwrap_or("").starts_with("refs/tags/"));
        for s in specs.iter_mut().filter(|s| !s.negative) {
            let d = s.dst.take().unwrap_or_default();
            s.dst = Some(format!(
                "refs/prefetch/{}",
                d.strip_prefix("refs/").unwrap_or(&d)
            ));
            s.force = true;
        }
    }
    if tags_mode == Some(true) && !o.prefetch {
        specs.push(parse_refspec("+refs/tags/*:refs/tags/*"));
    } else if o.prune_tags && !o.prefetch {
        // git-fetch --prune-tags: shorthand for a `refs/tags/*:refs/tags/*` refspec.
        specs.push(parse_refspec("refs/tags/*:refs/tags/*"));
    }
    let negatives: Vec<Refspec> = specs.iter().filter(|s| s.negative).cloned().collect();
    let excluded = |n: &str| negatives.iter().any(|s| s.excludes(n));

    // For-merge: explicit refs, else the current branch's configured upstream.
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let current = checked_out_branch(&repo);
    let upstream_merge: Option<String> = current.as_deref().and_then(|b| {
        let b = b.strip_prefix("refs/heads/")?;
        let r = cfg.get(&format!("branch.{b}.remote"))?;
        (Some(r.as_str()) == source.name.as_deref() || r == url)
            .then(|| cfg.get(&format!("branch.{b}.merge")))
            .flatten()
    });

    let mut plan: Vec<Entry> = Vec::new();
    for (si, spec) in specs.iter().enumerate().filter(|(_, s)| !s.negative) {
        let tags_spec =
            (tags_mode == Some(true) || o.prune_tags) && si + 1 == specs.len() && !o.prefetch;
        if spec.is_glob() {
            let mut hits: Vec<&(String, Oid)> = remote_refs
                .iter()
                .filter(|(n, _)| n != "HEAD" && spec.map_src(n).is_some() && !excluded(n))
                .collect();
            hits.sort_by(|a, b| a.0.cmp(&b.0));
            for (n, oid) in hits {
                // Peeled `^{}` entries are not refs.
                if n.ends_with("^{}") {
                    continue;
                }
                let merge = if explicit && !tags_spec {
                    true
                } else if tags_spec {
                    false
                } else {
                    upstream_merge.as_deref() == Some(n.as_str())
                };
                plan.push(Entry {
                    src: n.clone(),
                    oid: *oid,
                    dst: spec.map_src(n),
                    force: spec.force || o.force,
                    merge,
                    origin: Origin::Spec,
                });
            }
            continue;
        }
        let (full, oid) = dwim(&spec.src, &remote_refs)
            .cloned()
            .ok_or_else(|| CliError::fatal(format!("couldn't find remote ref {}", spec.src)))?;
        let dst = spec
            .dst
            .as_deref()
            .filter(|d| !d.is_empty())
            .map(|d| qualify_dst(d, &full));
        let merge = explicit || source.name.is_none() || upstream_merge.as_deref() == Some(&full);
        plan.push(Entry {
            src: full,
            oid,
            dst,
            force: spec.force || o.force,
            merge,
            origin: Origin::Spec,
        });
    }
    let stores_refs = plan.iter().any(|e| e.dst.is_some());

    // git truncates FETCH_HEAD before it updates any ref.
    if o.write_fetch_head && !o.dry_run && !o.append {
        fs::write(repo.git_dir().join("FETCH_HEAD"), "")?;
    }
    // Refuse to update the checked-out branch (git-fetch --update-head-ok).
    if let Some(cur) = &current {
        if !o.update_head_ok && plan.iter().any(|p| p.dst.as_deref() == Some(cur.as_str())) {
            let work = crate::util::work_tree(&repo)?;
            return Err(CliError::fatal(format!(
                "refusing to fetch into branch '{cur}' checked out at '{}'",
                work.display()
            )));
        }
    }

    let tips: Vec<Oid> = plan.iter().map(|p| p.oid).collect();
    arks_remote::copy_objects(&src, &mut repo, &tips).map_err(|e| CliError::Msg(e.to_string()))?;

    // Tag auto-following: remote tags we lack whose target we now have.
    if tags_mode.is_none() && stores_refs {
        let mut tag_entries = Vec::new();
        for (n, oid) in remote_refs
            .iter()
            .filter(|(n, _)| n.starts_with("refs/tags/"))
        {
            if repo.refs().resolve(n).is_ok() || plan.iter().any(|p| p.dst.as_ref() == Some(n)) {
                continue;
            }
            let target = peel(&src, oid);
            if repo.get_object(&target).is_ok() {
                tag_entries.push(Entry {
                    src: n.clone(),
                    oid: *oid,
                    dst: Some(n.clone()),
                    force: false,
                    merge: false,
                    origin: Origin::Tag,
                });
            }
        }
        let tag_tips: Vec<Oid> = tag_entries.iter().map(|e| e.oid).collect();
        arks_remote::copy_objects(&src, &mut repo, &tag_tips)
            .map_err(|e| CliError::Msg(e.to_string()))?;
        plan.extend(tag_entries);
    }

    // Opportunistic remote-tracking updates for command-line refs.
    if explicit {
        if let Some(name) = &source.name {
            let _ = name;
            let refmap: Vec<Refspec> = match &o.refmap {
                Some(m) => m
                    .iter()
                    .filter(|s| !s.is_empty())
                    .map(|s| parse_refspec(s))
                    .collect(),
                None => source
                    .config_specs
                    .iter()
                    .map(|s| parse_refspec(s))
                    .collect(),
            };
            let mut extra = Vec::new();
            for e in plan.iter().filter(|e| e.origin == Origin::Spec) {
                for m in refmap.iter().filter(|m| !m.negative) {
                    let Some(d) = m.map_src(&e.src) else {
                        continue;
                    };
                    let taken = plan
                        .iter()
                        .chain(extra.iter())
                        .any(|p: &Entry| p.dst.as_deref() == Some(d.as_str()));
                    if !taken {
                        extra.push(Entry {
                            src: e.src.clone(),
                            oid: e.oid,
                            dst: Some(d),
                            force: m.force || o.force,
                            merge: false,
                            origin: Origin::Opportunistic,
                        });
                    }
                }
            }
            plan.extend(extra);
        }
    }
    if o.dry_run && tags_mode.is_none() && stores_refs {
        // Stock git reports auto-followed tags a second time in a dry run
        // (the tag backfill pass sees them still missing).
        let again: Vec<Entry> = plan
            .iter()
            .filter(|e| e.origin == Origin::Tag)
            .cloned()
            .collect();
        plan.extend(again);
    }

    // Prune: local refs under a destination pattern without a remote source.
    let mut pruned: Vec<(String, Oid)> = Vec::new();
    // `--prune-tags` alone only adds the tag refspec; deleting needs --prune.
    if o.prune {
        let mut pspecs: Vec<Refspec> = specs.clone();
        if o.prune_tags
            && !specs
                .iter()
                .any(|s| s.dst.as_deref() == Some("refs/tags/*"))
        {
            pspecs.push(parse_refspec("refs/tags/*:refs/tags/*"));
        }
        let local = repo.refs().list_refs("refs/")?;
        for s in pspecs.iter().filter(|s| !s.negative && s.is_glob()) {
            if !o.prune && s.dst.as_deref() != Some("refs/tags/*") {
                continue;
            }
            for (n, oid) in &local {
                let Some(srcname) = s.map_dst(n) else {
                    continue;
                };
                let exists = remote_refs.iter().any(|(r, _)| *r == srcname);
                if !exists && !excluded(&srcname) && !pruned.iter().any(|(p, _)| p == n) {
                    pruned.push((n.clone(), *oid));
                }
            }
        }
    }

    // Decide every update.
    let mut outcomes: Vec<Outcome> = Vec::new();
    for e in &plan {
        let Some(dst) = &e.dst else {
            outcomes.push(Outcome::FetchHeadOnly);
            continue;
        };
        let old = repo.refs().resolve(dst).ok();
        let out = match old {
            None => Outcome::New,
            Some(old) if old == e.oid => Outcome::UpToDate,
            Some(old) if dst.starts_with("refs/tags/") => {
                if e.force {
                    Outcome::TagUpdate(old)
                } else {
                    Outcome::Rejected(Some(old), "would clobber existing tag")
                }
            }
            Some(old) => {
                let ff = is_commit(&repo, &old)
                    && is_commit(&repo, &e.oid)
                    && is_ancestor(&repo, &old, &e.oid)?;
                if ff {
                    Outcome::FastForward(old)
                } else if e.force {
                    Outcome::Forced(old)
                } else {
                    Outcome::Rejected(Some(old), "non-fast-forward")
                }
            }
        };
        outcomes.push(out);
    }
    let rejected = outcomes.iter().any(|x| matches!(x, Outcome::Rejected(..)));
    let apply = !o.dry_run && !(o.atomic && rejected);

    if apply {
        for (n, _) in &pruned {
            delete_ref_any(&mut repo, n)?;
        }
        for (e, out) in plan.iter().zip(&outcomes) {
            let Some(dst) = &e.dst else { continue };
            let (old, msg) = match out {
                Outcome::New => (None, "storing head"),
                Outcome::FastForward(old) => (Some(*old), "fast-forward"),
                Outcome::Forced(old) | Outcome::TagUpdate(old) => (Some(*old), "forced-update"),
                _ => continue,
            };
            if repo.refs().resolve(dst).ok() == Some(e.oid) {
                continue;
            }
            repo.refs_mut().update(dst, &e.oid, None)?;
            if dst.starts_with("refs/heads/") || dst.starts_with("refs/remotes/") {
                let name = source.name.as_deref().unwrap_or(url);
                let _ = crate::reflog::append_reflog(
                    &repo,
                    dst,
                    old.as_ref(),
                    &e.oid,
                    &format!("fetch {name}: {msg}"),
                );
            }
        }
    }

    // Reports.
    let width = refcol(
        plan.iter()
            .map(|e| short_ref(&e.src).len())
            .chain(std::iter::once(6).filter(|_| !pruned.is_empty())),
    );
    let mut lines: Vec<String> = Vec::new();
    let mut porcelain: Vec<String> = Vec::new();
    let zero = "0".repeat(plan.first().map_or(40, |e| e.oid.to_hex().len()));
    for (n, oid) in &pruned {
        lines.push(format!(
            " - {:<17} {:<width$} -> {}",
            "[deleted]",
            "(none)",
            short_ref(n)
        ));
        porcelain.push(format!("- {} {zero} {n}", oid.to_hex()));
    }
    for (e, out) in plan.iter().zip(&outcomes) {
        let s = short_ref(&e.src);
        let d = e.dst.as_deref().map(short_ref).unwrap_or("FETCH_HEAD");
        let hex = |x: &Oid| abbrev_hex(&repo, x, 7);
        let (flag, summary, suffix, pflag, old): (char, String, &str, char, Option<Oid>) = match out
        {
            Outcome::FetchHeadOnly => {
                let k = match kind_of(&e.src) {
                    "ref" => "",
                    k => k,
                };
                lines.push(format!(
                    " * {k:<17} {:<width$} -> FETCH_HEAD",
                    if e.src == "HEAD" { "HEAD" } else { s }
                ));
                continue;
            }
            Outcome::UpToDate => {
                if o.verbose {
                    lines.push(format!(" = {:<17} {s:<width$} -> {d}", "[up to date]"));
                }
                continue;
            }
            Outcome::New => {
                let what = match e.dst.as_deref() {
                    Some(x) if x.starts_with("refs/tags/") => "[new tag]",
                    Some(x) if x.starts_with("refs/heads/") && e.src.starts_with("refs/heads/") => {
                        "[new branch]"
                    }
                    Some(x)
                        if x.starts_with("refs/remotes/") || x.starts_with("refs/prefetch/") =>
                    {
                        if e.src.starts_with("refs/heads/") {
                            "[new branch]"
                        } else {
                            "[new ref]"
                        }
                    }
                    Some(_) if e.src.starts_with("refs/heads/") => "[new branch]",
                    _ => "[new ref]",
                };
                ('*', what.to_string(), "", '*', None)
            }
            Outcome::FastForward(old) => (
                ' ',
                format!("{}..{}", hex(old), hex(&e.oid)),
                "",
                ' ',
                Some(*old),
            ),
            Outcome::Forced(old) => (
                '+',
                format!("{}...{}", hex(old), hex(&e.oid)),
                "  (forced update)",
                '+',
                Some(*old),
            ),
            Outcome::TagUpdate(old) => ('t', "[tag update]".into(), "", 't', Some(*old)),
            Outcome::Rejected(old, why) => {
                lines.push(format!(
                    " ! {:<17} {s:<width$} -> {d}  ({why})",
                    "[rejected]"
                ));
                porcelain.push(format!(
                    "! {} {} {}",
                    old.map(|x| x.to_hex()).unwrap_or_else(|| zero.clone()),
                    e.oid.to_hex(),
                    e.dst.as_deref().unwrap_or("")
                ));
                continue;
            }
        };
        lines.push(format!(" {flag} {summary:<17} {s:<width$} -> {d}{suffix}"));
        porcelain.push(format!(
            "{pflag} {} {} {}",
            old.map(|x| x.to_hex()).unwrap_or_else(|| zero.clone()),
            e.oid.to_hex(),
            e.dst.as_deref().unwrap_or("")
        ));
    }
    if o.porcelain {
        let mut out = std::io::stdout().lock();
        for l in &porcelain {
            writeln!(out, "{l}")?;
        }
    } else if !o.quiet && !lines.is_empty() {
        eprintln!("From {}", display_url(url));
        for l in &lines {
            eprintln!("{l}");
        }
    }

    // FETCH_HEAD: for-merge entries first, then the rest (git-fetch OUTPUT).
    if o.write_fetch_head && !o.dry_run && apply {
        let shown = display_url(url);
        let mut body = String::new();
        for pass in [true, false] {
            for e in plan
                .iter()
                .filter(|e| e.origin != Origin::Opportunistic && e.merge == pass)
            {
                let label = match kind_of(&e.src) {
                    _ if e.src == "HEAD" => String::new(),
                    "ref" => format!("'{}' of ", e.src),
                    k => format!("{k} '{}' of ", short_ref(&e.src)),
                };
                let tag = if e.merge { "" } else { "not-for-merge" };
                body.push_str(&format!("{}\t{tag}\t{label}{shown}\n", e.oid.to_hex()));
            }
        }
        let path = repo.git_dir().join("FETCH_HEAD");
        if o.append {
            let mut f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            f.write_all(body.as_bytes())?;
        } else {
            fs::write(&path, body)?;
        }
    }

    // --set-upstream: the single fetched branch becomes the current branch's upstream.
    if o.set_upstream && !o.dry_run {
        let branches: Vec<&Entry> = plan
            .iter()
            .filter(|e| e.origin == Origin::Spec && e.src.starts_with("refs/heads/"))
            .collect();
        match (
            current
                .as_deref()
                .and_then(|c| c.strip_prefix("refs/heads/")),
            branches.as_slice(),
        ) {
            (Some(b), [one]) if explicit => {
                let remote = source.name.clone().unwrap_or_else(|| url.to_string());
                crate::cmd::config::cmd_config(&[format!("branch.{b}.remote"), remote])?;
                crate::cmd::config::cmd_config(&[format!("branch.{b}.merge"), one.src.clone()])?;
            }
            (None, _) => eprintln!(
                "warning: could not set upstream of HEAD to '{}' from '{}' when it does not point to any branch.",
                branches.first().map(|e| short_ref(&e.src)).unwrap_or(""),
                url
            ),
            _ => eprintln!("warning: multiple branches detected, incompatible with --set-upstream"),
        }
    }

    if rejected {
        if o.atomic {
            eprintln!("error: ");
        } else if !o.porcelain {
            eprintln!("error: some local refs could not be updated");
        }
        return Err(CliError::silent(1));
    }
    Ok(())
}

/// One resolved push update (`dst` on the remote; `new` None = delete).
pub(crate) struct LocalPush {
    pub src_label: String,
    pub dst: String,
    pub new: Option<Oid>,
    pub force: bool,
    /// `--force-with-lease` expectation: `Some(None)` = must not exist.
    pub expect: Option<Option<Oid>>,
}

#[derive(Default, Clone)]
pub(crate) struct LocalPushOpts {
    pub quiet: bool,
    pub verbose: bool,
    pub porcelain: bool,
    pub dry_run: bool,
    pub atomic: bool,
}

/// Status of one pushed ref, as `push --porcelain` reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PushStatus {
    UpToDate,
    New,
    FastForward(Oid),
    Forced(Oid),
    Deleted,
    Rejected(&'static str),
    /// The receiving end refused the ref; the reason is the peer's text.
    RemoteRejected(String),
}

/// Push updates into a local repository: copy objects, refuse non-fast-forward
/// (unless forced), honor leases and the remote's checked-out branch, then
/// update refs. Returns per-update statuses (in input order).
pub(crate) fn push_local(
    url: &str,
    updates: &[LocalPush],
    remote_refs: &[(String, Oid)],
    o: &LocalPushOpts,
) -> CliResult<Vec<PushStatus>> {
    let repo = open_repo()?;
    let mut dst_repo =
        arks_remote::open_local_remote(url).map_err(|e| CliError::fatal(e.to_string()))?;
    let current = checked_out_branch(&dst_repo);
    let mut statuses = Vec::new();
    for u in updates {
        let old = dst_repo.refs().resolve(&u.dst).ok();
        let st = if let Some(expect) = &u.expect {
            if old != *expect {
                PushStatus::Rejected("stale info")
            } else {
                classify_push(&repo, u, old)?
            }
        } else {
            classify_push(&repo, u, old)?
        };
        let st = match st {
            PushStatus::New | PushStatus::FastForward(_) | PushStatus::Forced(_)
                if current.as_deref() == Some(u.dst.as_str()) =>
            {
                PushStatus::RemoteRejected("branch is currently checked out".into())
            }
            PushStatus::Deleted if current.as_deref() == Some(u.dst.as_str()) => {
                PushStatus::RemoteRejected("deletion of the current branch prohibited".into())
            }
            s => s,
        };
        statuses.push(st);
    }
    let failed = statuses
        .iter()
        .any(|s| matches!(s, PushStatus::Rejected(_) | PushStatus::RemoteRejected(_)));
    if o.atomic && failed {
        for s in statuses.iter_mut() {
            if matches!(
                s,
                PushStatus::New
                    | PushStatus::FastForward(_)
                    | PushStatus::Forced(_)
                    | PushStatus::Deleted
            ) {
                *s = PushStatus::Rejected("atomic push failed");
            }
        }
    }
    if !o.dry_run {
        let tips: Vec<Oid> = updates
            .iter()
            .zip(&statuses)
            .filter(|(_, s)| {
                matches!(
                    s,
                    PushStatus::New | PushStatus::FastForward(_) | PushStatus::Forced(_)
                )
            })
            .filter_map(|(u, _)| u.new)
            .collect();
        arks_remote::copy_objects(&repo, &mut dst_repo, &tips)
            .map_err(|e| CliError::Msg(e.to_string()))?;
        for (u, s) in updates.iter().zip(&statuses) {
            match (s, u.new) {
                (PushStatus::New | PushStatus::FastForward(_) | PushStatus::Forced(_), Some(n)) => {
                    dst_repo.refs_mut().update(&u.dst, &n, None)?;
                }
                (PushStatus::Deleted, _) => delete_ref_any(&mut dst_repo, &u.dst)?,
                _ => {}
            }
        }
    }
    report_push(&repo, url, updates, &statuses, remote_refs, o)?;
    Ok(statuses)
}

pub(crate) fn classify_push(
    repo: &Repository,
    u: &LocalPush,
    old: Option<Oid>,
) -> CliResult<PushStatus> {
    Ok(match (old, u.new) {
        (None, None) => PushStatus::Rejected("remote ref does not exist"),
        (Some(_), None) => PushStatus::Deleted,
        (None, Some(_)) => PushStatus::New,
        (Some(o), Some(n)) if o == n => PushStatus::UpToDate,
        (Some(o), Some(n)) => {
            let have = repo.get_object(&o).is_ok();
            let ff = have
                && !u.dst.starts_with("refs/tags/")
                && is_commit(repo, &o)
                && is_commit(repo, &n)
                && is_ancestor(repo, &o, &n)?;
            if ff {
                PushStatus::FastForward(o)
            } else if u.force || u.expect.is_some() {
                PushStatus::Forced(o)
            } else if u.dst.starts_with("refs/tags/") {
                PushStatus::Rejected("already exists")
            } else if !have {
                PushStatus::Rejected("fetch first")
            } else {
                PushStatus::Rejected("non-fast-forward")
            }
        }
    })
}

/// Human (`To <url>` on stderr) or `--porcelain` (stdout) push report.
pub(crate) fn report_push(
    repo: &Repository,
    url: &str,
    updates: &[LocalPush],
    statuses: &[PushStatus],
    remote_refs: &[(String, Oid)],
    o: &LocalPushOpts,
) -> CliResult<()> {
    let hex = |x: &Oid| abbrev_hex(repo, x, 7);
    // The peer's existing refs (advertised order) come before new ones, in
    // three passes: up to date (verbose / porcelain), done, then failures.
    let mut order: Vec<usize> = (0..updates.len()).collect();
    order.sort_by_key(|&i| {
        remote_refs
            .iter()
            .position(|(n, _)| *n == updates[i].dst)
            .unwrap_or(remote_refs.len() + i)
    });
    let show_uptodate = o.verbose || o.porcelain;
    let mut seq: Vec<usize> = Vec::new();
    for pass in 0..3 {
        for &i in &order {
            let p = match statuses[i] {
                PushStatus::UpToDate => 0,
                PushStatus::Rejected(_) | PushStatus::RemoteRejected(_) => 2,
                _ => 1,
            };
            if p == pass && (p != 0 || show_uptodate) {
                seq.push(i);
            }
        }
    }
    let failed_any = statuses
        .iter()
        .any(|s| matches!(s, PushStatus::Rejected(_) | PushStatus::RemoteRejected(_)));
    let updates: Vec<&LocalPush> = seq.iter().map(|&i| &updates[i]).collect();
    let statuses: Vec<&PushStatus> = seq.iter().map(|&i| &statuses[i]).collect();
    let failed = failed_any;
    if o.porcelain {
        let mut out = std::io::stdout().lock();
        writeln!(out, "To {url}")?;
        for (u, s) in updates.iter().zip(statuses.iter()) {
            let from = if u.new.is_some() {
                full_src(&u.src_label, &u.dst)
            } else {
                String::new()
            };
            let (flag, summary) = match s {
                PushStatus::UpToDate => ('=', "[up to date]".to_string()),
                PushStatus::New => ('*', new_label(&u.dst, &from).to_string()),
                PushStatus::FastForward(old) => (
                    ' ',
                    format!("{}..{}", hex(old), hex(&u.new.unwrap_or(*old))),
                ),
                PushStatus::Forced(old) => (
                    '+',
                    format!(
                        "{}...{} (forced update)",
                        hex(old),
                        hex(&u.new.unwrap_or(*old))
                    ),
                ),
                PushStatus::Deleted => ('-', "[deleted]".to_string()),
                PushStatus::Rejected(why) => ('!', format!("[rejected] ({why})")),
                PushStatus::RemoteRejected(why) => ('!', format!("[remote rejected] ({why})")),
            };
            writeln!(out, "{flag}\t{from}:{}\t{summary}", u.dst)?;
        }
    } else if !o.quiet || failed {
        let shown: Vec<(&&LocalPush, &&PushStatus)> = updates.iter().zip(statuses.iter()).collect();
        if !shown.is_empty() {
            eprintln!("To {url}");
        }
        for (u, s) in shown {
            let d = short_ref(&u.dst);
            let line = match s {
                PushStatus::UpToDate => format!(" = {:<17} {} -> {d}", "[up to date]", u.src_label),
                PushStatus::New => format!(
                    " * {:<17} {} -> {d}",
                    new_label(&u.dst, &full_src(&u.src_label, &u.dst)),
                    u.src_label
                ),
                PushStatus::FastForward(old) => format!(
                    "   {:<17} {} -> {d}",
                    format!("{}..{}", hex(old), hex(&u.new.unwrap_or(*old))),
                    u.src_label
                ),
                PushStatus::Forced(old) => format!(
                    " + {:<17} {} -> {d} (forced update)",
                    format!("{}...{}", hex(old), hex(&u.new.unwrap_or(*old))),
                    u.src_label
                ),
                PushStatus::Deleted => format!(" - {:<17} {d}", "[deleted]"),
                PushStatus::Rejected(why) => {
                    format!(" ! {:<17} {} -> {d} ({why})", "[rejected]", u.src_label)
                }
                // A refused deletion names only the remote ref.
                PushStatus::RemoteRejected(why) if u.new.is_none() => {
                    format!(" ! [remote rejected] {d} ({why})")
                }
                PushStatus::RemoteRejected(why) => {
                    format!(" ! [remote rejected] {} -> {d} ({why})", u.src_label)
                }
            };
            eprintln!("{line}");
        }
    }
    if failed {
        eprintln!("error: failed to push some refs to '{url}'");
    }
    Ok(())
}

/// The full source ref name a push report shows (`refs/heads/main`).
fn full_src(label: &str, dst: &str) -> String {
    if label.starts_with("refs/")
        || label.is_empty()
        || (label.len() >= 7 && label.chars().all(|c| c.is_ascii_hexdigit()))
    {
        label.to_string()
    } else if dst.starts_with("refs/tags/") {
        format!("refs/tags/{label}")
    } else {
        format!("refs/heads/{label}")
    }
}

fn new_label(dst: &str, from: &str) -> &'static str {
    if dst.starts_with("refs/tags/") {
        "[new tag]"
    } else if dst.starts_with("refs/heads/") && (from.starts_with("refs/heads/") || from.is_empty())
    {
        "[new branch]"
    } else {
        "[new reference]"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refspec_mapping() {
        let s = parse_refspec("+refs/heads/*:refs/remotes/origin/*");
        assert!(s.force && s.is_glob());
        assert_eq!(
            s.map_src("refs/heads/a/b").as_deref(),
            Some("refs/remotes/origin/a/b")
        );
        assert_eq!(
            s.map_dst("refs/remotes/origin/x").as_deref(),
            Some("refs/heads/x")
        );
        assert!(parse_refspec("^refs/heads/x").excludes("refs/heads/x"));
        assert_eq!(display_url(".git/upstream.git"), ".git/upstream");
        assert_eq!(display_url("."), ".");
        assert_eq!(display_url("/srv/r.git/"), "/srv/r");
    }
}
