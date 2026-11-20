//! `pack-objects` — write a pack (and its index) from an object list, from
//! revisions (`--revs`, `--all`, `--reflog`, `--indexed-objects`), from
//! other packs (`--stdin-packs`) or of unreachable objects (`--cruft`).
//!
//! Objects are deltified against up to `--window` similar objects of the
//! same type (chains at most `--depth` long, `OFS_DELTA` with
//! `--delta-base-offset`), compressed at `--compression`, split at
//! `--max-pack-size`, and indexed with `--index-version` 1 or 2.
//! Cite: <https://git-scm.com/docs/git-pack-objects>,
//! <https://git-scm.com/docs/gitformat-pack>.

use crate::cmd::opt_parse::{Mode, OptDef, Takes::*, o, parse};
use crate::cmd::revwalk::{self, RevOpts};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{ObjectKind, RawObject, Tree, TreeMode};
use arks_oid::Oid;
use arks_pack::{PackIndex, PackWriter};
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: git pack-objects --stdout [<options>] [< <ref-list> | < <object-list>]
   or: git pack-objects [<options>] <base-name> [< <ref-list> | < <object-list>]
";

const DEFS: &[OptDef] = &[
    o("quiet", 'q', No, true),
    o("progress", ' ', No, true),
    o("all-progress", ' ', No, true),
    o("all-progress-implied", ' ', No, true),
    o("index-version", ' ', Req, false),
    o("max-pack-size", ' ', Req, false),
    o("local", ' ', No, true),
    o("incremental", ' ', No, true),
    o("window", ' ', Req, true),
    o("window-memory", ' ', Req, false),
    o("depth", ' ', Req, true),
    o("reuse-delta", ' ', No, true),
    o("reuse-object", ' ', No, true),
    o("delta-base-offset", ' ', No, true),
    o("threads", ' ', Req, true),
    o("non-empty", ' ', No, true),
    o("revs", ' ', No, true),
    o("unpacked", ' ', No, false),
    o("all", ' ', No, false),
    o("reflog", ' ', No, false),
    o("indexed-objects", ' ', No, false),
    o("stdin-packs", ' ', No, true),
    o("stdout", ' ', No, true),
    o("include-tag", ' ', No, true),
    o("keep-unreachable", ' ', No, true),
    o("pack-loose-unreachable", ' ', No, true),
    o("unpack-unreachable", ' ', Opt, true),
    o("cruft", ' ', No, true),
    o("cruft-expiration", ' ', Opt, true),
    o("sparse", ' ', No, true),
    o("thin", ' ', No, true),
    o("shallow", ' ', No, true),
    o("honor-pack-keep", ' ', No, true),
    o("keep-pack", ' ', Req, true),
    o("compression", ' ', Req, true),
    o("keep-true-parents", ' ', No, true),
    o("use-bitmap-index", ' ', No, true),
    o("write-bitmap-index", ' ', No, true),
    o("filter", ' ', Req, true),
    o("missing", ' ', Req, false),
    o("exclude-promisor-objects", ' ', No, true),
    o("delta-islands", ' ', No, true),
    o("uri-protocol", ' ', Req, true),
];

fn usage() -> CliError {
    CliError::Exit(129, USAGE.to_string())
}

/// `--filter=<spec>` of the object walk.
#[derive(Clone, Copy)]
enum Filter {
    NoBlobs,
    BlobLimit(u64),
    TreeDepth(usize),
    Kind(ObjectKind),
}

fn size_value(v: &str) -> Option<u64> {
    let (num, mul) = match v.chars().last()? {
        'k' | 'K' => (&v[..v.len() - 1], 1024),
        'm' | 'M' => (&v[..v.len() - 1], 1024 * 1024),
        'g' | 'G' => (&v[..v.len() - 1], 1024 * 1024 * 1024),
        _ => (v, 1),
    };
    num.parse::<u64>().ok().map(|n| n * mul)
}

fn parse_filter(v: &str) -> CliResult<Filter> {
    let bad = || CliError::fatal(format!("invalid filter-spec '{v}'"));
    if v == "blob:none" {
        return Ok(Filter::NoBlobs);
    }
    if let Some(n) = v.strip_prefix("blob:limit=") {
        return Ok(Filter::BlobLimit(size_value(n).ok_or_else(bad)?));
    }
    if let Some(n) = v.strip_prefix("tree:") {
        return Ok(Filter::TreeDepth(n.parse().map_err(|_| bad())?));
    }
    if let Some(k) = v.strip_prefix("object:type=") {
        return Ok(Filter::Kind(match k {
            "blob" => ObjectKind::Blob,
            "tree" => ObjectKind::Tree,
            "commit" => ObjectKind::Commit,
            "tag" => ObjectKind::Tag,
            _ => return Err(bad()),
        }));
    }
    Err(bad())
}

/// One object chosen for the pack, with the path it was reached by.
struct Want {
    oid: Oid,
    name: String,
}

/// Packs on disk: base path, kept?, promisor?, object names.
struct PackInfo {
    base: PathBuf,
    name: String,
    keep: bool,
    promisor: bool,
    objects: Vec<Oid>,
}

fn load_packs(repo: &Repository) -> Vec<PackInfo> {
    let dir = repo.common_dir().join("objects/pack");
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(&dir) else {
        return out;
    };
    let mut idx: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "idx"))
        .collect();
    idx.sort();
    for p in idx {
        let Ok(bytes) = fs::read(&p) else { continue };
        let Ok(index) = PackIndex::decode(&bytes, repo.object_format()) else {
            continue;
        };
        let base = p.with_extension("");
        let name = base
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(PackInfo {
            keep: base.with_extension("keep").is_file(),
            promisor: base.with_extension("promisor").is_file(),
            base,
            name,
            objects: index.entries.iter().map(|e| e.oid).collect(),
        });
    }
    out
}

fn pack_matches(p: &PackInfo, arg: &str) -> bool {
    let a = arg.rsplit('/').next().unwrap_or(arg);
    a == format!("{}.pack", p.name) || a == p.name || a == format!("{}.idx", p.name)
}

struct Opts {
    stdout: bool,
    quiet: bool,
    progress: bool,
    window: usize,
    depth: usize,
    ofs_delta: bool,
    level: Option<u32>,
    max_pack_size: u64,
    index_version: u32,
    non_empty: bool,
    write_bitmap: bool,
    include_tag: bool,
    filter: Option<Filter>,
    missing: String,
}

fn int_opt(p: &crate::cmd::opt_parse::Parsed, key: &str) -> CliResult<Option<i64>> {
    match p.last(key) {
        None | Some(None) => Ok(None),
        Some(Some(v)) => v.parse::<i64>().map(Some).map_err(|_| {
            CliError::Exit(
                129,
                format!("error: option `{key}' expects a numerical value"),
            )
        }),
    }
}

pub fn cmd_pack_objects(args: &[String]) -> CliResult<()> {
    let p = parse("pack-objects", args, DEFS, Mode::default())?;
    let stdout_pack = p.on("stdout");
    let argv = p.args();
    if (stdout_pack && !argv.is_empty()) || (!stdout_pack && argv.len() != 1) {
        return Err(usage());
    }
    let window = int_opt(&p, "window")?.unwrap_or(10).max(0) as usize;
    let depth = int_opt(&p, "depth")?.unwrap_or(50).clamp(0, 4095) as usize;
    let _threads = int_opt(&p, "threads")?;
    let level = match int_opt(&p, "compression")? {
        None | Some(-1) => None,
        Some(n) if (0..=9).contains(&n) => Some(n as u32),
        Some(n) => return Err(CliError::fatal(format!("bad pack compression level {n}"))),
    };
    let mut max_pack_size = match p.value("max-pack-size") {
        None => 0,
        Some(v) => size_value(&v).ok_or_else(|| {
            CliError::Exit(
                129,
                "error: option `max-pack-size' expects a non-negative integer value with an optional k/m/g suffix".into(),
            )
        })?,
    };
    if max_pack_size > 0 && max_pack_size < 1024 * 1024 {
        eprintln!("warning: minimum pack size limit is 1 MiB");
        max_pack_size = 1024 * 1024;
    }
    if let Some(v) = p.value("window-memory") {
        size_value(&v).ok_or_else(|| {
            CliError::Exit(
                129,
                "error: option `window-memory' expects a non-negative integer value with an optional k/m/g suffix".into(),
            )
        })?;
    }
    let index_version = match p.value("index-version") {
        None => 2,
        Some(v) => {
            let (ver, _off) = v.split_once(',').unwrap_or((v.as_str(), ""));
            match ver.parse::<u32>() {
                Ok(n @ (1 | 2)) => n,
                Ok(n) => return Err(CliError::fatal(format!("unsupported index version {n}"))),
                Err(_) => return Err(CliError::fatal(format!("bad index version '{v}'"))),
            }
        }
    };
    let filter = match p.last("filter").and_then(|v| v) {
        Some(v) => Some(parse_filter(&v)?),
        None => None,
    };
    let missing = p.value("missing").unwrap_or_else(|| "error".into());
    if !matches!(missing.as_str(), "error" | "allow-any" | "allow-promisor") {
        return Err(CliError::fatal(format!(
            "invalid value for '--missing': '{missing}'"
        )));
    }
    let cruft = p.on("cruft") || p.flag("cruft-expiration") == Some(true);
    let stdin_packs = p.on("stdin-packs");
    let internal_revs = p.on("revs")
        || p.on("all")
        || p.on("reflog")
        || p.on("indexed-objects")
        || p.on("unpacked");
    if stdin_packs && internal_revs {
        return Err(CliError::fatal(
            "cannot use internal rev list with --stdin-packs",
        ));
    }
    if cruft && internal_revs {
        return Err(CliError::fatal("cannot use internal rev list with --cruft"));
    }
    if cruft && stdin_packs {
        return Err(CliError::fatal("cannot use --stdin-packs with --cruft"));
    }
    let o = Opts {
        stdout: stdout_pack,
        quiet: p.on("quiet"),
        progress: p.on("progress") || p.on("all-progress"),
        window,
        depth,
        ofs_delta: p.on("delta-base-offset"),
        level,
        max_pack_size,
        index_version,
        non_empty: p.on("non-empty"),
        write_bitmap: p.on("write-bitmap-index"),
        include_tag: p.on("include-tag"),
        filter,
        missing,
    };
    let mut repo = open_repo()?;
    let packs = load_packs(&repo);
    // Objects excluded because they already live in a pack we must not repack.
    let mut skip: HashSet<Oid> = HashSet::new();
    let keep_names = p.values("keep-pack");
    for pk in &packs {
        let excluded = (p.on("honor-pack-keep") && pk.keep)
            || keep_names.iter().any(|k| pack_matches(pk, k))
            || (p.on("exclude-promisor-objects") && pk.promisor)
            || p.on("unpacked")
            || p.on("incremental");
        if excluded {
            skip.extend(pk.objects.iter().copied());
        }
    }
    let mut mtimes: Option<HashMap<Oid, u32>> = None;
    let wants: Vec<Want> = if stdin_packs {
        stdin_pack_objects(&packs)?
    } else if cruft {
        let expiration = p.last("cruft-expiration").flatten();
        let (w, m) = cruft_objects(&repo, &packs, expiration.as_deref())?;
        mtimes = Some(m);
        w
    } else if internal_revs {
        rev_objects(&repo, &p, &o, &skip)?
    } else {
        object_list(&repo, &skip)?
    };
    if internal_revs {
        unreachable_extras(&mut repo, &p, &packs, &wants)?;
    }
    let mut wants = wants;
    // "Unreachable" here means not reached by this walk.
    if internal_revs && (p.on("keep-unreachable") || p.on("pack-loose-unreachable")) {
        let have: HashSet<Oid> = wants.iter().map(|w| w.oid).collect();
        let mut add: Vec<Oid> = Vec::new();
        if p.on("keep-unreachable") {
            for pk in packs.iter().filter(|pk| !pk.keep) {
                add.extend(pk.objects.iter().filter(|x| !have.contains(x)));
            }
        }
        if p.on("pack-loose-unreachable") {
            add.extend(
                crate::cmd::plumbing_ext::loose_objects(&repo)
                    .into_iter()
                    .map(|(x, _)| x)
                    .filter(|x| !have.contains(x)),
            );
        }
        let mut seen = have;
        for x in add {
            if !skip.contains(&x) && seen.insert(x) {
                wants.push(Want {
                    oid: x,
                    name: String::new(),
                });
            }
        }
    }
    write_packs(&repo, &o, &wants, &argv, mtimes.as_ref())
}

/// The default input: `<object-id>[ <name>]` per line.
fn object_list(repo: &Repository, skip: &HashSet<Oid>) -> CliResult<Vec<Want>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let hex_len = repo.object_format().digest_len() * 2;
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let (hex, name) = line.split_at(line.len().min(hex_len));
        let oid = Oid::from_hex(hex)
            .ok()
            .filter(|_| name.is_empty() || name.starts_with(' '))
            .ok_or_else(|| CliError::fatal(format!("expected object ID, got garbage:\n {line}")))?;
        if skip.contains(&oid) || !seen.insert(oid) {
            continue;
        }
        out.push(Want {
            oid,
            name: name.trim_start().to_string(),
        });
    }
    Ok(out)
}

/// `--stdin-packs`: objects of the listed packs, minus `^`-excluded packs.
fn stdin_pack_objects(packs: &[PackInfo]) -> CliResult<Vec<Want>> {
    let mut include: Vec<&PackInfo> = Vec::new();
    let mut exclude: HashSet<Oid> = HashSet::new();
    for line in io::stdin().lock().lines() {
        let line = line?;
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let (neg, name) = match l.strip_prefix('^') {
            Some(n) => (true, n),
            None => (false, l),
        };
        let pk = packs
            .iter()
            .find(|p| pack_matches(p, name))
            .ok_or_else(|| CliError::fatal(format!("could not find pack '{name}'")))?;
        if neg {
            exclude.extend(pk.objects.iter().copied());
        } else {
            include.push(pk);
        }
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for pk in include {
        for x in &pk.objects {
            if !exclude.contains(x) && seen.insert(*x) {
                out.push(Want {
                    oid: *x,
                    name: String::new(),
                });
            }
        }
    }
    Ok(out)
}

fn mtime_secs(p: &Path) -> u32 {
    fs::metadata(p)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

fn expiry(v: &str) -> Option<i64> {
    match v {
        "now" => Some(i64::MAX),
        "never" => None,
        _ => crate::cmd::config::expiry_date(v),
    }
}

/// `--cruft`: objects of the packs listed with a leading `-` and loose
/// objects, minus objects of the other (retained) listed packs.
fn cruft_objects(
    repo: &Repository,
    packs: &[PackInfo],
    expiration: Option<&str>,
) -> CliResult<(Vec<Want>, HashMap<Oid, u32>)> {
    let mut retained: HashSet<Oid> = HashSet::new();
    let mut discard: Vec<&PackInfo> = Vec::new();
    for line in io::stdin().lock().lines() {
        let line = line?;
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let (gone, name) = match l.strip_prefix('-') {
            Some(n) => (true, n),
            None => (false, l),
        };
        let pk = packs
            .iter()
            .find(|p| pack_matches(p, name))
            .ok_or_else(|| CliError::fatal(format!("could not find pack '{name}'")))?;
        if gone {
            discard.push(pk);
        } else {
            retained.extend(pk.objects.iter().copied());
        }
    }
    let limit = expiration.and_then(expiry);
    let mut out = Vec::new();
    let mut mtimes = HashMap::new();
    let mut seen = HashSet::new();
    let mut consider = |oid: Oid, mtime: u32, out: &mut Vec<Want>| {
        if retained.contains(&oid) || !seen.insert(oid) {
            return;
        }
        if let Some(l) = limit {
            if l == i64::MAX || i64::from(mtime) <= l {
                return;
            }
        }
        mtimes.insert(oid, mtime);
        out.push(Want {
            oid,
            name: String::new(),
        });
    };
    for pk in discard {
        let m = mtime_secs(&pk.base.with_extension("pack"));
        for x in &pk.objects {
            consider(*x, m, &mut out);
        }
    }
    for (x, path) in crate::cmd::plumbing_ext::loose_objects(repo) {
        consider(x, mtime_secs(&path), &mut out);
    }
    Ok((out, mtimes))
}

/// `--unpack-unreachable[=<time>]`: unreachable packed objects become loose.
fn unreachable_extras(
    repo: &mut Repository,
    p: &crate::cmd::opt_parse::Parsed,
    packs: &[PackInfo],
    wants: &[Want],
) -> CliResult<()> {
    let Some(when) = p.last("unpack-unreachable") else {
        return Ok(());
    };
    let limit = when.as_deref().and_then(expiry);
    let packed: HashSet<Oid> = wants.iter().map(|w| w.oid).collect();
    let objects_dir = repo.store().objects_dir().to_path_buf();
    let format = repo.object_format();
    for pk in packs.iter().filter(|pk| !pk.keep) {
        let m = i64::from(mtime_secs(&pk.base.with_extension("pack")));
        if limit.is_some_and(|l| l == i64::MAX || m <= l) {
            continue;
        }
        for x in &pk.objects {
            if packed.contains(x) {
                continue;
            }
            if let Ok(obj) = repo.get_object(x) {
                arks_object::write_loose(&objects_dir, format, &obj)?;
            }
        }
    }
    Ok(())
}

/// Revision input (`--revs`, `--all`, ...): the objects `rev-list --objects`
/// would list, plus annotated tags named as tips.
fn rev_objects(
    repo: &Repository,
    p: &crate::cmd::opt_parse::Parsed,
    o: &Opts,
    skip: &HashSet<Oid>,
) -> CliResult<Vec<Want>> {
    let mut rev = RevOpts::default();
    let mut revs: Vec<String> = Vec::new();
    if p.on("revs") {
        let mut not = false;
        for line in io::stdin().lock().lines() {
            let line = line?;
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l == "--not" {
                not = !not;
                continue;
            }
            if l.starts_with("--shallow ") || l == "--shallow" {
                continue;
            }
            if l.starts_with('-') {
                return Err(CliError::fatal(format!("not a rev '{l}'")));
            }
            let neg = l.starts_with('^') != not;
            let body = l.trim_start_matches('^');
            crate::cmd::shared::resolve_rev(repo, body.split("..").last().unwrap_or(body))
                .map_err(|_| CliError::fatal(format!("bad revision '{l}'")))?;
            revs.push(if neg {
                format!("^{body}")
            } else {
                body.to_string()
            });
        }
    }
    for r in revs {
        rev.push_rev(&r);
    }
    if p.on("all") {
        let args = vec!["--all".to_string()];
        let mut a = crate::cmd::diffcore::options::Args::new(&args);
        rev.take(&mut a)?;
    }
    let mut extra_tips: Vec<Oid> = Vec::new();
    if p.on("reflog") {
        for name in crate::cmd::inspect::all_reflog_names(repo) {
            for (old, new, _) in crate::reflog::read_entries(repo, &name) {
                extra_tips.push(new);
                extra_tips.extend(old);
            }
        }
    }
    let mut index_objects: Vec<Want> = Vec::new();
    if p.on("indexed-objects") {
        if let Ok(index) = crate::util::read_index(repo) {
            for e in &index.entries {
                if e.mode != 0o160000 {
                    index_objects.push(Want {
                        oid: e.oid,
                        name: e.path.clone(),
                    });
                }
            }
        }
    }
    let mut out: Vec<Want> = Vec::new();
    let mut seen: HashSet<Oid> = skip.clone();
    let mut tags: Vec<Oid> = Vec::new();
    let mut commits: Vec<Oid> = Vec::new();
    let mut hidden_trees: Vec<Oid> = Vec::new();
    if !rev.revs.is_empty() || !extra_tips.is_empty() {
        let mut tips = revwalk::resolve_tips(repo, &rev, None)?;
        for t in &extra_tips {
            if let Ok(obj) = repo.get_object(t) {
                if obj.kind == ObjectKind::Commit {
                    tips.include.push(*t);
                }
            }
        }
        // Annotated tags named as tips are packed too.
        for r in &rev.revs {
            if r.starts_with('^') {
                continue;
            }
            let names: Vec<String> = match r.strip_prefix('\u{0}') {
                Some(code) => revwalk::select_refs(repo, code)?,
                None => vec![r.clone()],
            };
            for n in names {
                let oid = repo
                    .refs()
                    .resolve(&n)
                    .ok()
                    .or_else(|| crate::cmd::shared::resolve_rev(repo, &n).ok());
                let Some(mut cur) = oid else { continue };
                while let Ok(obj) = repo.get_object(&cur) {
                    if obj.kind != ObjectKind::Tag {
                        break;
                    }
                    tags.push(cur);
                    match crate::cmd::shared::tag_target(&obj.payload) {
                        Some(t) => cur = t,
                        None => break,
                    }
                }
            }
        }
        if !tips.include.is_empty() {
            for w in revwalk::walk(repo, &rev, &tips)? {
                commits.push(w.oid);
            }
        }
        for x in &tips.exclude {
            if let Ok(c) = crate::cmd::log_format::read_commit(repo, x) {
                hidden_trees.push(c.tree);
            }
        }
    }
    // Objects reachable from excluded commits stay out (they are "edges").
    let mut hidden: HashSet<Oid> = HashSet::new();
    for t in &hidden_trees {
        mark_tree(repo, t, &mut hidden)?;
    }
    let keep = |kind: ObjectKind, oid: &Oid, depth: usize| -> bool {
        match o.filter {
            None => true,
            Some(Filter::NoBlobs) => kind != ObjectKind::Blob,
            Some(Filter::BlobLimit(n)) => {
                kind != ObjectKind::Blob
                    || repo
                        .get_object(oid)
                        .is_ok_and(|x| (x.payload.len() as u64) < n)
            }
            Some(Filter::TreeDepth(n)) => {
                !matches!(kind, ObjectKind::Tree | ObjectKind::Blob) || depth < n
            }
            Some(Filter::Kind(k)) => kind == k,
        }
    };
    for c in &commits {
        if seen.insert(*c) && keep(ObjectKind::Commit, c, 0) {
            out.push(Want {
                oid: *c,
                name: String::new(),
            });
        }
    }
    for t in &tags {
        if seen.insert(*t) && keep(ObjectKind::Tag, t, 0) {
            out.push(Want {
                oid: *t,
                name: String::new(),
            });
        }
    }
    let mut stack_trees: Vec<(Oid, String, usize)> = Vec::new();
    for c in &commits {
        let commit = crate::cmd::log_format::read_commit(repo, c)?;
        stack_trees.push((commit.tree, String::new(), 0));
        walk_tree_objects(
            repo,
            &mut stack_trees,
            &hidden,
            &mut seen,
            &keep,
            &mut out,
            &o.missing,
        )?;
    }
    for w in index_objects {
        if !hidden.contains(&w.oid) && seen.insert(w.oid) && keep(ObjectKind::Blob, &w.oid, 1) {
            out.push(w);
        }
    }
    if o.include_tag {
        let have: HashSet<Oid> = out.iter().map(|w| w.oid).collect();
        for (name, oid) in repo.refs().list_refs("refs/tags/")? {
            let mut cur = oid;
            let mut chain = Vec::new();
            while let Ok(obj) = repo.get_object(&cur) {
                if obj.kind != ObjectKind::Tag {
                    break;
                }
                chain.push(cur);
                match crate::cmd::shared::tag_target(&obj.payload) {
                    Some(t) => cur = t,
                    None => break,
                }
            }
            if !chain.is_empty() && have.contains(&cur) {
                for t in chain {
                    if seen.insert(t) {
                        out.push(Want {
                            oid: t,
                            name: name.clone(),
                        });
                    }
                }
            }
        }
    }
    Ok(out)
}

fn mark_tree(repo: &Repository, t: &Oid, seen: &mut HashSet<Oid>) -> CliResult<()> {
    if !seen.insert(*t) {
        return Ok(());
    }
    let Ok(obj) = repo.get_object(t) else {
        return Ok(());
    };
    for e in Tree::decode(&obj.payload, repo.oid_kind())?.entries {
        if e.mode.is_tree() {
            mark_tree(repo, &e.oid, seen)?;
        } else if e.mode != TreeMode::Gitlink {
            seen.insert(e.oid);
        }
    }
    Ok(())
}

fn walk_tree_objects(
    repo: &Repository,
    stack: &mut Vec<(Oid, String, usize)>,
    hidden: &HashSet<Oid>,
    seen: &mut HashSet<Oid>,
    keep: &dyn Fn(ObjectKind, &Oid, usize) -> bool,
    out: &mut Vec<Want>,
    missing: &str,
) -> CliResult<()> {
    while let Some((t, path, depth)) = stack.pop() {
        if hidden.contains(&t) || !seen.insert(t) {
            continue;
        }
        let obj = match repo.get_object(&t) {
            Ok(o) => o,
            Err(_) if missing != "error" => continue,
            Err(_) => {
                return Err(CliError::fatal(format!(
                    "unable to read tree {}",
                    t.to_hex()
                )));
            }
        };
        if keep(ObjectKind::Tree, &t, depth) {
            out.push(Want {
                oid: t,
                name: path.clone(),
            });
        }
        let mut subtrees = Vec::new();
        for e in Tree::decode(&obj.payload, repo.oid_kind())?.entries {
            let p = if path.is_empty() {
                e.name.clone()
            } else {
                format!("{path}/{}", e.name)
            };
            if e.mode.is_tree() {
                subtrees.push((e.oid, p, depth + 1));
            } else if e.mode != TreeMode::Gitlink {
                if hidden.contains(&e.oid) || seen.contains(&e.oid) {
                    continue;
                }
                if missing == "error" && repo.get_object(&e.oid).is_err() {
                    return Err(CliError::fatal(format!(
                        "unable to read {}",
                        e.oid.to_hex()
                    )));
                }
                if repo.get_object(&e.oid).is_err() {
                    continue;
                }
                seen.insert(e.oid);
                if keep(ObjectKind::Blob, &e.oid, depth + 1) {
                    out.push(Want {
                        oid: e.oid,
                        name: p,
                    });
                }
            }
        }
        // Depth-first in tree order.
        subtrees.reverse();
        stack.extend(subtrees);
    }
    Ok(())
}

/// Delta grouping key: objects with the same trailing path sort together.
fn name_hash(name: &str) -> u32 {
    let mut h: u32 = 0;
    for c in name.bytes().filter(|c| !c.is_ascii_whitespace()) {
        h = (h >> 2).wrapping_add(u32::from(c) << 24);
    }
    h
}

fn kind_rank(k: ObjectKind) -> u8 {
    match k {
        ObjectKind::Commit => 0,
        ObjectKind::Tag => 1,
        ObjectKind::Tree => 2,
        ObjectKind::Blob => 3,
    }
}

struct Entry {
    oid: Oid,
    obj: RawObject,
    /// Delta base and instruction stream, when deltified.
    delta: Option<(usize, Vec<u8>)>,
    depth: usize,
}

/// Choose delta bases: a sliding window over objects sorted by type, name
/// hash and size (largest first), keeping the smallest delta found.
fn find_deltas(entries: &mut [Entry], names: &[String], window: usize, max_depth: usize) {
    if window == 0 || max_depth == 0 {
        return;
    }
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by(|a, b| {
        let (ea, eb) = (&entries[*a], &entries[*b]);
        kind_rank(ea.obj.kind)
            .cmp(&kind_rank(eb.obj.kind))
            .then(name_hash(&names[*a]).cmp(&name_hash(&names[*b])))
            .then(eb.obj.payload.len().cmp(&ea.obj.payload.len()))
    });
    for (pos, &i) in order.iter().enumerate() {
        let size = entries[i].obj.payload.len();
        if size < 64 {
            continue;
        }
        let max = size / 2 - 20;
        let mut best: Option<(usize, Vec<u8>)> = None;
        for &j in order[pos.saturating_sub(window)..pos].iter().rev() {
            if entries[j].obj.kind != entries[i].obj.kind || entries[j].depth >= max_depth {
                continue;
            }
            // A base may not depend on the target (no cycles).
            let mut k = j;
            let mut cyclic = false;
            while let Some((b, _)) = &entries[k].delta {
                if *b == i {
                    cyclic = true;
                    break;
                }
                k = *b;
            }
            if cyclic {
                continue;
            }
            let limit = best.as_ref().map_or(max, |(_, d)| d.len());
            if let Some(d) =
                arks_pack::create_delta(&entries[j].obj.payload, &entries[i].obj.payload, limit)
            {
                best = Some((j, d));
            }
        }
        if let Some((j, d)) = best {
            entries[i].depth = entries[j].depth + 1;
            entries[i].delta = Some((j, d));
        }
    }
}

fn write_packs(
    repo: &Repository,
    o: &Opts,
    wants: &[Want],
    argv: &[String],
    mtimes: Option<&HashMap<Oid, u32>>,
) -> CliResult<()> {
    let format = repo.object_format();
    let mut entries: Vec<Entry> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for w in wants {
        match repo.get_object(&w.oid) {
            Ok(obj) => {
                entries.push(Entry {
                    oid: w.oid,
                    obj,
                    delta: None,
                    depth: 0,
                });
                names.push(w.name.clone());
            }
            Err(_) if o.missing != "error" => {}
            Err(_) => {
                return Err(CliError::fatal(format!(
                    "unable to read {}",
                    w.oid.to_hex()
                )));
            }
        }
    }
    if entries.is_empty() && o.non_empty {
        return Ok(());
    }
    find_deltas(&mut entries, &names, o.window, o.depth);
    // Write order: commits, tags, trees, blobs (each in walk order), with
    // every delta base before the objects depending on it.
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|i| kind_rank(entries[*i].obj.kind));
    let mut packs: Vec<arks_pack::WrittenPack> = Vec::new();
    let mut writer = PackWriter::new(format, 0, o.level);
    let mut placed: Vec<Option<usize>> = vec![None; entries.len()];
    let mut stack: Vec<usize> = Vec::new();
    let (mut deltas, mut total) = (0usize, 0usize);
    for &start in &order {
        stack.push(start);
        while let Some(&i) = stack.last() {
            if placed[i].is_some() {
                stack.pop();
                continue;
            }
            if let Some((b, _)) = &entries[i].delta {
                if placed[*b].is_none() {
                    stack.push(*b);
                    continue;
                }
            }
            stack.pop();
            if o.max_pack_size > 0 && !writer.is_empty() && writer.len() >= o.max_pack_size {
                let full = std::mem::replace(&mut writer, PackWriter::new(format, 0, o.level));
                packs.push(full.finish());
            }
            let e = &entries[i];
            let here = packs.len();
            match &e.delta {
                Some((b, d)) if placed[*b] == Some(here) => {
                    if o.ofs_delta {
                        writer.add_ofs_delta(e.oid, &entries[*b].oid, d)?;
                    } else {
                        writer.add_ref_delta(e.oid, &entries[*b].oid, d)?;
                    }
                    deltas += 1;
                }
                _ => writer.add_full(e.oid, e.obj.kind, &e.obj.payload)?,
            }
            total += 1;
            placed[i] = Some(here);
        }
    }
    packs.push(writer.finish());
    if o.progress && !o.quiet {
        eprintln!("Total {total} (delta {deltas}), reused 0 (delta 0), pack-reused 0");
    }
    if o.stdout {
        let mut out = io::stdout().lock();
        for pk in &packs {
            out.write_all(&pk.pack)?;
        }
        return Ok(out.flush()?);
    }
    let base = &argv[0];
    let write_rev = arks_config::ConfigSet::load(Some(repo.common_dir()))
        .get("pack.writeReverseIndex")
        .is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1"));
    for pk in packs {
        let hex: String = pk.checksum.iter().map(|b| format!("{b:02x}")).collect();
        let stem = format!("{base}-{hex}");
        if let Some(parent) = Path::new(&stem).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let index = PackIndex::from_entries(format, pk.entries.clone(), pk.checksum.clone())?;
        fs::write(format!("{stem}.pack"), &pk.pack)?;
        let idx = if o.index_version == 1 {
            arks_pack::encode_idx_v1(&index)?
        } else {
            index.encode()?
        };
        if let Some(m) = mtimes {
            fs::write(
                format!("{stem}.mtimes"),
                arks_pack::encode_mtimes(&index, m),
            )?;
        }
        if write_rev || mtimes.is_some() || o.write_bitmap {
            fs::write(format!("{stem}.rev"), arks_pack::encode_rev(&index))?;
        }
        if o.write_bitmap {
            let bitmap =
                crate::cmd::pack_bitmap::encode(repo, &index, &entries_kinds(repo, &index))?;
            fs::write(format!("{stem}.bitmap"), bitmap)?;
        }
        fs::write(format!("{stem}.idx"), idx)?;
        println!("{hex}");
    }
    Ok(())
}

fn entries_kinds(repo: &Repository, index: &PackIndex) -> Vec<ObjectKind> {
    index
        .entries
        .iter()
        .map(|e| {
            repo.get_object(&e.oid)
                .map(|o| o.kind)
                .unwrap_or(ObjectKind::Blob)
        })
        .collect()
}
