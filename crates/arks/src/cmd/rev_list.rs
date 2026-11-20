//! `git rev-list` on the shared revision walk: commit listing with
//! `--parents` / `--children` / `--timestamp` / `--left-right` /
//! `--boundary` / `--cherry-mark` marks, `--count`, `--header`,
//! `--pretty` / `--format` (`--[no-]commit-header`), abbreviations,
//! `--objects` (`--[no-]object-names`, `--objects-edge`), `--disk-usage`,
//! `--unpacked`, `--quiet` and the bisection helpers (`--bisect`,
//! `--bisect-vars`, `--bisect-all`).
//!
//! Primary: <https://git-scm.com/docs/git-rev-list>.

use crate::cmd::diffcore::options::Args;
use crate::cmd::log_format::{FmtCtx, Pretty, expand_ctx, read_commit, take_pretty};
use crate::cmd::revwalk::{self, RevOpts, Walked};
use crate::cmd::shared::abbrev_hex;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{ObjectKind, Tree};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};

#[derive(Default)]
struct Out {
    count: bool,
    parents: bool,
    children: bool,
    timestamp: bool,
    header: bool,
    commit_header: bool,
    objects: bool,
    objects_edge: bool,
    object_names: bool,
    disk_usage: Option<bool>,
    unpacked: bool,
    quiet: bool,
    abbrev_commit: bool,
    abbrev: usize,
    bisect: bool,
    bisect_vars: bool,
    bisect_all: bool,
    graph: bool,
}

fn take_out(o: &mut Out, a: &mut Args<'_>) -> CliResult<bool> {
    match a.cur() {
        "--count" => o.count = true,
        "--parents" => o.parents = true,
        "--children" => o.children = true,
        "--timestamp" => o.timestamp = true,
        "--header" => o.header = true,
        "--commit-header" => o.commit_header = true,
        "--no-commit-header" => o.commit_header = false,
        "--objects" | "--in-commit-order" => o.objects = true,
        "--objects-edge" | "--objects-edge-aggressive" => {
            o.objects = true;
            o.objects_edge = true;
        }
        "--object-names" => o.object_names = true,
        "--no-object-names" => o.object_names = false,
        "--disk-usage" => o.disk_usage = Some(false),
        "--disk-usage=human" => o.disk_usage = Some(true),
        "--unpacked" => o.unpacked = true,
        "--quiet" => o.quiet = true,
        "--abbrev-commit" => o.abbrev_commit = true,
        "--no-abbrev-commit" => o.abbrev_commit = false,
        "--no-abbrev" => o.abbrev = 40,
        "--bisect" => o.bisect = true,
        "--bisect-vars" => o.bisect_vars = true,
        "--bisect-all" => o.bisect_all = true,
        "--graph" => o.graph = true,
        "--progress"
        | "--no-progress"
        | "--indexed-objects"
        | "--exclude-promisor-objects"
        | "--filter-provided-objects"
        | "--filter-print-omitted"
        | "--no-filter" => {}
        x => {
            if let Some(v) = x.strip_prefix("--abbrev=") {
                let n: usize = v
                    .parse()
                    .map_err(|_| CliError::fatal("--abbrev expects a numerical value"))?;
                o.abbrev = if n == 0 { 40 } else { n.clamp(4, 40) };
            } else if x == "--abbrev" {
                o.abbrev = 7;
            } else if let Some(v) = x.strip_prefix("--disk-usage=") {
                return Err(CliError::fatal(format!(
                    "invalid value for '--disk-usage=<format>': '{v}', the only allowed format is 'human'"
                )));
            } else if let Some(v) = x.strip_prefix("--missing=") {
                if !matches!(v, "error" | "allow-any" | "allow-promisor" | "print") {
                    return Err(CliError::fatal(format!(
                        "invalid value for '--missing': '{v}'"
                    )));
                }
            } else if let Some(v) = x.strip_prefix("--filter=") {
                if v != "blob:none" && !v.starts_with("blob:limit=") && !v.starts_with("tree:") {
                    return Err(CliError::NotImplemented(format!(
                        "unsupported option: --filter={v}"
                    )));
                }
            } else {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn hex(repo: &Repository, o: &Out, oid: &Oid) -> String {
    if o.abbrev_commit && o.abbrev < 40 {
        abbrev_hex(repo, oid, o.abbrev)
    } else {
        oid.to_hex()
    }
}

/// Objects of the walked commits' trees not reachable from excluded commits.
fn list_objects(
    repo: &Repository,
    commits: &[Walked],
    excluded_trees: &[Oid],
) -> CliResult<Vec<(Oid, String)>> {
    let mut seen: HashSet<Oid> = HashSet::new();
    fn mark(repo: &Repository, t: &Oid, seen: &mut HashSet<Oid>) -> CliResult<()> {
        if !seen.insert(*t) {
            return Ok(());
        }
        let obj = repo.get_object(t)?;
        for e in Tree::decode(&obj.payload, repo.oid_kind())?.entries {
            if e.mode.is_tree() {
                mark(repo, &e.oid, seen)?;
            } else if !matches!(e.mode, arks_object::TreeMode::Gitlink) {
                seen.insert(e.oid);
            }
        }
        Ok(())
    }
    for t in excluded_trees {
        mark(repo, t, &mut seen)?;
    }
    let mut out = Vec::new();
    fn walk(
        repo: &Repository,
        t: &Oid,
        path: &str,
        seen: &mut HashSet<Oid>,
        out: &mut Vec<(Oid, String)>,
    ) -> CliResult<()> {
        if !seen.insert(*t) {
            return Ok(());
        }
        out.push((*t, path.to_string()));
        let obj = repo.get_object(t)?;
        for e in Tree::decode(&obj.payload, repo.oid_kind())?.entries {
            let p = if path.is_empty() {
                e.name.clone()
            } else {
                format!("{path}/{}", e.name)
            };
            if e.mode.is_tree() {
                walk(repo, &e.oid, &p, seen, out)?;
            } else if matches!(e.mode, arks_object::TreeMode::Gitlink) {
                continue;
            } else if seen.insert(e.oid) {
                out.push((e.oid, p));
            }
        }
        Ok(())
    }
    for w in commits {
        if w.boundary {
            continue;
        }
        walk(repo, &w.commit.tree, "", &mut seen, &mut out)?;
    }
    Ok(out)
}

/// On-disk size of an object: the loose file, or its packed entry.
fn disk_size(repo: &Repository, oid: &Oid, packs: &[(Vec<(Oid, u64)>, u64)]) -> u64 {
    let hex = oid.to_hex();
    let loose = repo
        .common_dir()
        .join("objects")
        .join(&hex[..2])
        .join(&hex[2..]);
    if let Ok(m) = std::fs::metadata(&loose) {
        return m.len();
    }
    for (entries, pack_len) in packs {
        if let Ok(i) = entries.binary_search_by(|(o, _)| o.as_bytes().cmp(oid.as_bytes())) {
            let off = entries[i].1;
            let mut next = pack_len.saturating_sub(20);
            for (_, o2) in entries {
                if *o2 > off && *o2 < next {
                    next = *o2;
                }
            }
            return next - off;
        }
    }
    0
}

fn load_packs(repo: &Repository) -> Vec<(Vec<(Oid, u64)>, u64)> {
    let mut v = Vec::new();
    let dir = repo.common_dir().join("objects").join("pack");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return v;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("idx") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&p) else {
            continue;
        };
        let Ok(idx) = arks_pack::PackIndex::decode(&bytes, repo.object_format()) else {
            continue;
        };
        let pack_len = std::fs::metadata(p.with_extension("pack"))
            .map(|m| m.len())
            .unwrap_or(0);
        let mut entries: Vec<(Oid, u64)> = idx.entries.iter().map(|x| (x.oid, x.offset)).collect();
        entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        v.push((entries, pack_len));
    }
    v
}

fn human(n: u64) -> String {
    const U: [&str; 5] = ["bytes", "KiB", "MiB", "GiB", "TiB"];
    if n < 1024 {
        return format!("{n} {}", if n == 1 { "byte" } else { "bytes" });
    }
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    format!("{v:.2} {}", U[i])
}

/// `floor(log2(n))` based estimate of the remaining bisection steps.
fn bisect_steps(all: usize) -> usize {
    if all < 3 {
        return 0;
    }
    let e = (usize::BITS - 1 - all.leading_zeros()) as usize;
    let x = all - (1 << e);
    if (1usize << e) < 3 * x { e } else { e - 1 }
}

pub fn cmd_rev_list(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-rev-list
    let (head, paths): (Vec<String>, Vec<String>) = match args.iter().position(|a| a == "--") {
        Some(i) => (args[..i].to_vec(), args[i + 1..].to_vec()),
        None => (args.to_vec(), Vec::new()),
    };
    let mut head = head;
    let pretty = if head
        .iter()
        .any(|a| a.starts_with("--pretty") || a.starts_with("--format") || a == "--oneline")
    {
        Some(take_pretty(&mut head)?)
    } else {
        None
    };
    let mut rev = RevOpts::default();
    let mut out = Out {
        abbrev: 7,
        commit_header: true,
        object_names: true,
        ..Default::default()
    };
    let mut a = Args::new(&head);
    while a.i < head.len() {
        let cur = a.cur().to_string();
        if take_out(&mut out, &mut a)? || rev.take(&mut a)? {
            a.i += 1;
            continue;
        }
        if cur.starts_with('-') && cur.len() > 1 {
            return Err(CliError::NotImplemented(format!(
                "unsupported option: {cur}"
            )));
        }
        // Pushed in order so `--not` negates only what follows it.
        rev.push_rev(&cur);
        a.i += 1;
    }
    let repo = open_repo()?;
    rev.specs = paths;
    if out.graph && rev.order.is_none() {
        rev.order = Some(revwalk::Order::Topo);
    }
    if rev.revs.is_empty() && !rev.stdin && !rev.walk_reflogs {
        return Err(CliError::Usage(
            "usage: git rev-list [<options>] <commit>... [--] [<path>...]".into(),
        ));
    }
    let tips = revwalk::resolve_tips(&repo, &rev, None)?;
    let (bisect, vars, all_b) = (out.bisect, out.bisect_vars, out.bisect_all);
    let mut list = if rev.walk_reflogs {
        revwalk::walk_reflogs(&repo, &rev)?
    } else {
        revwalk::walk(&repo, &rev, &tips)?
    };
    if out.unpacked {
        list.retain(|w| {
            let h = w.oid.to_hex();
            repo.common_dir()
                .join("objects")
                .join(&h[..2])
                .join(&h[2..])
                .exists()
        });
    }
    if out.quiet {
        return Ok(());
    }
    let mut w = io::BufWriter::new(io::stdout().lock());
    if bisect || vars || all_b {
        let set: HashSet<Oid> = list.iter().map(|x| x.oid).collect();
        let all = set.len();
        let mut dist: Vec<(Oid, usize, usize)> = Vec::new();
        for x in &list {
            let reach = arks_repo::ancestor_set(&repo, x.oid)?
                .into_iter()
                .filter(|o| set.contains(o))
                .count();
            dist.push((x.oid, reach, reach.min(all - reach)));
        }
        let best = dist.iter().max_by_key(|d| d.2).copied();
        if all_b {
            let decos = crate::cmd::log_cmd::decoration_labels(&repo)?;
            let mut sorted = dist.clone();
            sorted.sort_by_key(|d| std::cmp::Reverse(d.2));
            for (oid, _, d) in sorted {
                match decos.get(&oid) {
                    Some(l) => writeln!(w, "{} ({}, dist={d})", oid.to_hex(), l.join(", "))?,
                    None => writeln!(w, "{} (dist={d})", oid.to_hex())?,
                }
            }
            if !vars {
                return Ok(w.flush()?);
            }
            writeln!(w)?;
        }
        let Some((boid, reach, _)) = best else {
            return Ok(w.flush()?);
        };
        if vars {
            let good = all - reach;
            writeln!(w, "bisect_rev='{}'", boid.to_hex())?;
            writeln!(w, "bisect_nr={}", good.saturating_sub(1))?;
            writeln!(w, "bisect_good={}", good.saturating_sub(1))?;
            writeln!(w, "bisect_bad={}", reach.saturating_sub(1))?;
            writeln!(w, "bisect_all={all}")?;
            writeln!(w, "bisect_steps={}", bisect_steps(all))?;
        } else if bisect && !all_b {
            writeln!(w, "{}", boid.to_hex())?;
        }
        return Ok(w.flush()?);
    }
    if let Some(human_fmt) = out.disk_usage {
        let packs = load_packs(&repo);
        let mut total = 0u64;
        for x in &list {
            total += disk_size(&repo, &x.oid, &packs);
        }
        if out.objects {
            let ex: Vec<Oid> = tips
                .exclude
                .iter()
                .filter_map(|e| read_commit(&repo, e).ok().map(|c| c.tree))
                .collect();
            for (o, _) in list_objects(&repo, &list, &ex)? {
                total += disk_size(&repo, &o, &packs);
            }
        }
        if human_fmt {
            writeln!(w, "{}", human(total))?;
        } else {
            writeln!(w, "{total}")?;
        }
        return Ok(w.flush()?);
    }
    if out.count {
        if rev.left_right {
            let l = list.iter().filter(|x| x.side == Some('<')).count();
            let r = list.iter().filter(|x| x.side == Some('>')).count();
            if rev.cherry_mark {
                let e = list.iter().filter(|x| x.cherry == Some('=')).count();
                let l = list
                    .iter()
                    .filter(|x| x.side == Some('<') && x.cherry != Some('='))
                    .count();
                let r = list
                    .iter()
                    .filter(|x| x.side == Some('>') && x.cherry != Some('='))
                    .count();
                writeln!(w, "{l}\t{r}\t{e}")?;
            } else {
                writeln!(w, "{l}\t{r}")?;
            }
        } else if rev.cherry_mark {
            let e = list.iter().filter(|x| x.cherry == Some('=')).count();
            writeln!(w, "{}\t{e}", list.len() - e)?;
        } else {
            writeln!(w, "{}", list.len())?;
        }
        return Ok(w.flush()?);
    }
    let children: HashMap<Oid, Vec<Oid>> = if out.children {
        let mut m: HashMap<Oid, Vec<Oid>> = HashMap::new();
        for x in &list {
            for p in &x.parents {
                m.entry(*p).or_default().push(x.oid);
            }
        }
        m
    } else {
        HashMap::new()
    };
    let decos = match &pretty {
        Some(Pretty::Format(f)) | Some(Pretty::TFormat(f))
            if f.contains("%d") || f.contains("%D") =>
        {
            Some(crate::cmd::log_cmd::decoration_labels(&repo)?)
        }
        _ => None,
    };
    let ctx = FmtCtx {
        date: Default::default(),
        decorations: decos.as_ref(),
    };
    let one_line = matches!(pretty, Some(Pretty::Oneline));
    for x in &list {
        let mut line = String::new();
        if out.timestamp {
            line.push_str(&format!("{} ", x.commit.committer.timestamp));
        }
        if x.boundary {
            line.push('-');
        } else if let Some(c) = x.cherry {
            line.push(c);
        } else if rev.left_right {
            if let Some(s) = x.side {
                line.push(s);
            }
        }
        let use_abbrev = out.abbrev_commit || one_line;
        let h = if use_abbrev && out.abbrev < 40 {
            abbrev_hex(&repo, &x.oid, out.abbrev)
        } else {
            x.oid.to_hex()
        };
        line.push_str(&h);
        if out.parents {
            for p in &x.parents {
                line.push(' ');
                line.push_str(&hex(&repo, &out, p));
            }
        }
        if out.children {
            for c in children.get(&x.oid).into_iter().flatten() {
                line.push(' ');
                line.push_str(&hex(&repo, &out, c));
            }
        }
        if out.header {
            let raw = repo.get_object(&x.oid)?.payload;
            writeln!(w, "{line}")?;
            let text = String::from_utf8_lossy(&raw);
            let (hd, body) = text.split_once("\n\n").unwrap_or((&text, ""));
            writeln!(w, "{hd}")?;
            writeln!(w)?;
            for l in body.trim_end_matches('\n').lines() {
                writeln!(w, "    {l}")?;
            }
            w.write_all(b"\0")?;
            continue;
        }
        match &pretty {
            None => writeln!(w, "{line}")?,
            Some(Pretty::Oneline) => writeln!(
                w,
                "{line} {}",
                crate::cmd::log_format::subject_line(&x.commit.message)
            )?,
            Some(Pretty::TFormat(f)) | Some(Pretty::Format(f)) => {
                if out.commit_header {
                    writeln!(w, "commit {line}")?;
                }
                let text = expand_ctx(f, &x.oid, &x.commit, &ctx);
                writeln!(w, "{text}")?;
            }
            Some(p) => {
                let spec = match p {
                    Pretty::Short => "short",
                    Pretty::Full => "full",
                    Pretty::Fuller => "fuller",
                    Pretty::Raw => "raw",
                    _ => "medium",
                };
                let text =
                    crate::cmd::log_cmd::format_commit_header(&repo, &x.oid, &x.commit, spec)?;
                // rev-list headers end without the trailing record newline.
                w.write_all(text.as_bytes())?;
            }
        }
    }
    if out.objects {
        let mut ex: Vec<Oid> = Vec::new();
        for e in &tips.exclude {
            if let Ok(c) = read_commit(&repo, e) {
                ex.push(c.tree);
                if out.objects_edge {
                    writeln!(w, "-{}", e.to_hex())?;
                }
            }
        }
        for (o, p) in list_objects(&repo, &list, &ex)? {
            let kind = repo.get_object(&o).map(|x| x.kind).ok();
            if out.object_names && kind != Some(ObjectKind::Commit) {
                writeln!(w, "{} {p}", o.to_hex())?;
            } else {
                writeln!(w, "{}", o.to_hex())?;
            }
        }
    }
    w.flush()?;
    Ok(())
}
