//! `fast-export` — dump history as a `git fast-import` stream.
//!
//! Commits come in reverse topological order, each labelled with the ref it
//! was reached from; blobs precede the commit that first uses them; annotated
//! tags and refs whose tip carries another label follow the commits.
//! Options: `--progress`, `--signed-tags`, `--tag-of-filtered-object`,
//! `--reencode`, `--export-marks`, `--import-marks[-if-exists]`,
//! `--fake-missing-tagger`, `--full-tree`, `--use-done-feature`,
//! `--[no-]data`, `--refspec`, `--anonymize[-map]`,
//! `--reference-excluded-parents`, `--show-original-ids`, `--mark-tags`,
//! plus the rev-list options and pathspecs of the walk.
//! Cite: <https://git-scm.com/docs/git-fast-export>,
//! <https://git-scm.com/docs/git-fast-import> (stream format).

use crate::cmd::diffcore::options::Args;
use crate::cmd::opt_parse::{Item, Mode, OptDef, Takes::*, o, parse};
use crate::cmd::revwalk::{self, Order, RevOpts};
use crate::cmd::shared::{peel_tags, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{ObjectKind, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};
use std::fs;
use std::io::{self, Write};

const USAGE: &str = "usage: git fast-export [<rev-list-opts>]

    --progress <n>        show progress after <n> objects
    --signed-tags <mode>  select handling of signed tags
    --tag-of-filtered-object <mode>
                          select handling of tags that tag filtered objects
    --reencode <mode>     select handling of commit messages in an alternate encoding
    --export-marks <file>
                          dump marks to this file
    --import-marks <file>
                          import marks from this file
    --import-marks-if-exists <file>
                          import marks from this file if it exists
    --fake-missing-tagger
                          fake a tagger when tags lack one
    --full-tree           output full tree for each commit
    --use-done-feature    use the done feature to terminate the stream
    --no-data             skip output of blob data
    --data                opposite of --no-data
    --refspec <refspec>   apply refspec to exported refs
    --anonymize           anonymize output
    --anonymize-map <from:to>
                          convert <from> to <to> in anonymized output
    --reference-excluded-parents
                          reference parents which are not in fast-export stream by object id
    --show-original-ids   show original object ids of blobs/commits
    --mark-tags           label tags with mark ids
";

const DEFS: &[OptDef] = &[
    o("progress", ' ', Req, true),
    o("signed-tags", ' ', Req, true),
    o("tag-of-filtered-object", ' ', Req, true),
    o("reencode", ' ', Req, true),
    o("export-marks", ' ', Req, true),
    o("import-marks", ' ', Req, true),
    o("import-marks-if-exists", ' ', Req, true),
    o("fake-missing-tagger", ' ', No, true),
    o("full-tree", ' ', No, true),
    o("use-done-feature", ' ', No, true),
    o("no-data", ' ', No, true),
    o("refspec", ' ', Req, true),
    o("anonymize", ' ', No, true),
    o("anonymize-map", ' ', Req, false),
    o("reference-excluded-parents", ' ', No, true),
    o("show-original-ids", ' ', No, true),
    o("mark-tags", ' ', No, true),
];

fn usage() -> CliError {
    CliError::Exit(129, USAGE.to_string())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Signed {
    Abort,
    Verbatim,
    Warn,
    WarnStrip,
    Strip,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filtered {
    Abort,
    Drop,
    Rewrite,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reencode {
    Abort,
    Yes,
    No,
}

/// Anonymization maps (token → replacement), one per kind of token.
#[derive(Default)]
struct Anon {
    seeds: HashMap<String, String>,
    refs: HashMap<String, String>,
    paths: HashMap<String, String>,
    idents: HashMap<String, String>,
    subjects: HashMap<Vec<u8>, String>,
    tag_msgs: HashMap<Vec<u8>, String>,
    blobs: HashMap<Oid, String>,
    oids: HashMap<Oid, String>,
}

impl Anon {
    fn token(
        map: &mut HashMap<String, String>,
        seeds: &HashMap<String, String>,
        tok: &str,
        prefix: &str,
    ) -> String {
        if let Some(s) = seeds.get(tok) {
            return s.clone();
        }
        let n = map.len();
        map.entry(tok.to_string())
            .or_insert_with(|| format!("{prefix}{n}"))
            .clone()
    }

    fn path(&mut self, p: &str) -> String {
        p.split('/')
            .map(|c| Self::token(&mut self.paths, &self.seeds, c, "path"))
            .collect::<Vec<_>>()
            .join("/")
    }

    fn refname(&mut self, r: &str) -> String {
        for pfx in ["refs/heads/", "refs/tags/", "refs/remotes/", "refs/"] {
            if let Some(rest) = r.strip_prefix(pfx) {
                let body = rest
                    .split('/')
                    .map(|c| Self::token(&mut self.refs, &self.seeds, c, "ref"))
                    .collect::<Vec<_>>()
                    .join("/");
                return format!("{pfx}{body}");
            }
        }
        if r == "HEAD" {
            return r.to_string();
        }
        Self::token(&mut self.refs, &self.seeds, r, "ref")
    }

    /// `Name <email>` of an ident line (the date part is kept).
    fn ident(&mut self, line: &str) -> String {
        let (who, when) = match line.rfind('>') {
            Some(i) => (&line[..=i], &line[i + 1..]),
            None => (line, ""),
        };
        let n = self.idents.len();
        let anon = self
            .idents
            .entry(who.to_string())
            .or_insert_with(|| format!("User {n} <user{n}@example.com>"))
            .clone();
        format!("{anon}{when}")
    }

    fn oid(&mut self, oid: &Oid) -> String {
        let n = self.oids.len() + 1;
        self.oids
            .entry(*oid)
            .or_insert_with(|| format!("{n:040x}"))
            .clone()
    }
}

struct Opts {
    progress: Option<u64>,
    signed: Signed,
    filtered: Filtered,
    reencode: Reencode,
    fake_tagger: bool,
    full_tree: bool,
    done: bool,
    no_data: bool,
    refspecs: Vec<(String, String)>,
    anonymize: bool,
    ref_excluded: bool,
    original_ids: bool,
    mark_tags: bool,
}

struct Exporter<'r> {
    repo: &'r Repository,
    o: Opts,
    out: Vec<u8>,
    marks: HashMap<Oid, u32>,
    last_mark: u32,
    /// Commits marked by `--import-marks` (already exported).
    shown: HashSet<Oid>,
    objects: u64,
    anon: Anon,
    specs: Vec<String>,
}

/// The raw pieces of a commit object.
struct RawCommit {
    tree: Oid,
    author: Option<String>,
    committer: Option<String>,
    encoding: Option<String>,
    message: Vec<u8>,
}

fn parse_raw_commit(payload: &[u8]) -> CliResult<RawCommit> {
    let end = payload
        .windows(2)
        .position(|w| w == b"\n\n")
        .map(|p| p + 1)
        .unwrap_or(payload.len());
    let header = String::from_utf8_lossy(&payload[..end]).into_owned();
    let message = payload.get(end + 1..).unwrap_or(&[]).to_vec();
    let mut c = RawCommit {
        tree: Oid::from_hex(&"0".repeat(40)).map_err(|e| CliError::Msg(e.to_string()))?,
        author: None,
        committer: None,
        encoding: None,
        message,
    };
    for line in header.lines() {
        if let Some(v) = line.strip_prefix("tree ") {
            c.tree = Oid::from_hex(v.trim()).map_err(|e| CliError::Msg(e.to_string()))?;
        } else if let Some(v) = line.strip_prefix("author ") {
            c.author = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("committer ") {
            c.committer = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("encoding ") {
            c.encoding = Some(v.to_string());
        }
    }
    Ok(c)
}

/// fast-import path quoting: C-style quoting, also for paths with a space.
fn quote(p: &str) -> String {
    let q = crate::cmd::diffcore::emit::quote_path(p);
    if q == p && p.contains(' ') {
        format!("\"{p}\"")
    } else {
        q
    }
}

/// Depth-first change order: byte order on the common length, then the
/// longer (deeper) path first, so `d/e` precedes `d`.
fn depth_first(a: &str, b: &str) -> std::cmp::Ordering {
    let n = a.len().min(b.len());
    a.as_bytes()[..n]
        .cmp(&b.as_bytes()[..n])
        .then_with(|| b.len().cmp(&a.len()))
}

fn mode_bits(m: TreeMode) -> u32 {
    match m {
        TreeMode::Executable => 0o100755,
        TreeMode::Symlink => 0o120000,
        TreeMode::Gitlink => 0o160000,
        TreeMode::Tree => 0o040000,
        TreeMode::File => 0o100644,
    }
}

fn flatten(
    repo: &Repository,
    tree: &Oid,
    prefix: &str,
    out: &mut BTreeMap<String, (u32, Oid)>,
) -> CliResult<()> {
    let obj = repo.get_object(tree)?;
    let t = arks_object::Tree::decode(&obj.payload, repo.oid_kind())?;
    for e in t.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        if e.mode == TreeMode::Tree {
            flatten(repo, &e.oid, &path, out)?;
        } else {
            out.insert(path, (mode_bits(e.mode), e.oid));
        }
    }
    Ok(())
}

fn in_specs(specs: &[String], path: &str) -> bool {
    specs.is_empty()
        || specs.iter().any(|s| {
            let s = s.trim_end_matches('/');
            s.is_empty() || s == "." || path == s || path.starts_with(&format!("{s}/"))
        })
}

fn has_signature(msg: &[u8]) -> Option<usize> {
    for marker in [
        "-----BEGIN PGP SIGNATURE-----",
        "-----BEGIN PGP MESSAGE-----",
        "-----BEGIN SSH SIGNATURE-----",
        "-----BEGIN SIGNED MESSAGE-----",
    ] {
        let m = marker.as_bytes();
        if let Some(p) = msg.windows(m.len()).position(|w| w == m) {
            if p == 0 || msg[p - 1] == b'\n' {
                return Some(p);
            }
        }
    }
    None
}

pub fn cmd_fast_export(args: &[String]) -> CliResult<()> {
    if args.is_empty() {
        return Err(usage());
    }
    let p = parse(
        "fast-export",
        args,
        DEFS,
        Mode {
            keep_unknown: true,
            ..Mode::default()
        },
    )?;
    let mode_of = |key: &str| p.last(key).and_then(|v| v);
    let signed = match mode_of("signed-tags").as_deref() {
        None | Some("abort") => Signed::Abort,
        Some("verbatim") | Some("ignore") => Signed::Verbatim,
        Some("warn") => Signed::Warn,
        Some("warn-strip") => Signed::WarnStrip,
        Some("strip") => Signed::Strip,
        Some(m) => {
            return Err(CliError::Exit(
                129,
                format!("error: Unknown signed-tags mode: {m}"),
            ));
        }
    };
    let filtered = match mode_of("tag-of-filtered-object").as_deref() {
        None | Some("abort") => Filtered::Abort,
        Some("drop") => Filtered::Drop,
        Some("rewrite") => Filtered::Rewrite,
        Some(m) => {
            return Err(CliError::Exit(
                129,
                format!("error: Unknown tag-of-filtered mode: {m}"),
            ));
        }
    };
    let reencode = match mode_of("reencode").as_deref() {
        None | Some("abort") => Reencode::Abort,
        Some("yes") | Some("true") | Some("on") => Reencode::Yes,
        Some("no") | Some("false") | Some("off") => Reencode::No,
        Some(m) => {
            return Err(CliError::Exit(
                129,
                format!("error: Unknown reencoding mode: {m}"),
            ));
        }
    };
    let progress = match mode_of("progress") {
        None => None,
        Some(v) => Some(v.parse::<u64>().map_err(|_| {
            CliError::Exit(
                129,
                "error: option `progress' expects a numerical value".into(),
            )
        })?),
    };
    let anonymize = p.on("anonymize");
    let maps = p.values("anonymize-map");
    if !maps.is_empty() && !anonymize {
        return Err(CliError::fatal(
            "the option '--anonymize-map' requires '--anonymize'",
        ));
    }
    let mut refspecs = Vec::new();
    for r in p.values("refspec") {
        // A refspec without a destination renames nothing.
        if let Some((src, dst)) = r.split_once(':') {
            refspecs.push((src.trim_start_matches('+').to_string(), dst.to_string()));
        }
    }
    let o = Opts {
        progress: progress.filter(|n| *n > 0),
        signed,
        filtered,
        reencode,
        fake_tagger: p.on("fake-missing-tagger"),
        full_tree: p.on("full-tree"),
        done: p.on("use-done-feature"),
        no_data: p.on("no-data"),
        refspecs,
        anonymize,
        ref_excluded: p.on("reference-excluded-parents"),
        original_ids: p.on("show-original-ids"),
        mark_tags: p.on("mark-tags"),
    };
    // Revision arguments in order (unknown options are rev-list options).
    let mut rev_args: Vec<String> = Vec::new();
    for it in &p.items {
        match it {
            Item::Arg(a) | Item::Unknown(a) | Item::AfterDashDash(a) => rev_args.push(a.clone()),
            Item::Number(n) => rev_args.push(format!("-{n}")),
            Item::Opt { .. } => {}
        }
    }
    let repo = open_repo()?;
    let mut rev = RevOpts::default();
    let mut specs: Vec<String> = Vec::new();
    let mut a = Args::new(&rev_args);
    while a.i < rev_args.len() {
        let cur = a.cur().to_string();
        if !specs.is_empty() {
            specs.push(cur);
            a.i += 1;
            continue;
        }
        if rev.take(&mut a)? {
            a.i += 1;
            continue;
        }
        if cur.starts_with('-') && cur.len() > 1 {
            return Err(usage());
        }
        let body = cur.trim_start_matches('^');
        let sides: Vec<&str> = match body.split_once("...") {
            Some((l, r)) => vec![l, r],
            None => body.split("..").collect(),
        };
        let is_rev = sides.iter().filter(|s| !s.is_empty()).all(|s| {
            let s = s
                .strip_suffix("^@")
                .or_else(|| s.strip_suffix("^!"))
                .unwrap_or(s);
            resolve_rev(&repo, s).is_ok()
        });
        if is_rev && !body.is_empty() {
            rev.push_rev(&cur);
        } else {
            let wt = crate::util::work_tree(&repo).ok();
            let exists = wt.as_ref().is_some_and(|w| w.join(&cur).exists());
            if !exists {
                return Err(CliError::fatal(format!(
                    "ambiguous argument '{cur}': unknown revision or path not in the working tree.\nUse '--' to separate paths from revisions, like this:\n'git <command> [<revision>...] -- [<file>...]'"
                )));
            }
            specs.push(cur);
        }
        a.i += 1;
    }
    if rev.order.is_none() {
        rev.order = Some(Order::Topo);
    }
    rev.rewrite = !specs.is_empty();
    rev.specs = specs.clone();
    let mut ex = Exporter {
        repo: &repo,
        o,
        out: Vec::new(),
        marks: HashMap::new(),
        last_mark: 0,
        shown: HashSet::new(),
        objects: 0,
        anon: Anon::default(),
        specs,
    };
    for m in &maps {
        let (from, to) = m.split_once(':').unwrap_or((m.as_str(), m.as_str()));
        ex.anon.seeds.insert(from.to_string(), to.to_string());
    }
    if let Some(f) = mode_of("import-marks") {
        ex.import_marks(&f, false)?;
    }
    if let Some(f) = mode_of("import-marks-if-exists") {
        ex.import_marks(&f, true)?;
    }
    let res = ex.run(&rev);
    let mut stdout = io::stdout().lock();
    stdout.write_all(&ex.out)?;
    stdout.flush()?;
    res?;
    if let Some(f) = mode_of("export-marks") {
        ex.export_marks(&f)?;
    }
    Ok(())
}

impl Exporter<'_> {
    fn import_marks(&mut self, file: &str, if_exists: bool) -> CliResult<()> {
        let text = match fs::read_to_string(file) {
            Ok(t) => t,
            Err(e) if if_exists && e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(_) => {
                return Err(CliError::fatal(format!(
                    "could not open '{file}' for reading: No such file or directory"
                )));
            }
        };
        for line in text.lines() {
            let bad = || CliError::fatal(format!("corrupt mark line: {line}"));
            let (m, hex) = line
                .strip_prefix(':')
                .and_then(|r| r.split_once(' '))
                .ok_or_else(bad)?;
            let mark: u32 = m.parse().map_err(|_| bad())?;
            let oid = Oid::from_hex(hex.trim()).map_err(|_| bad())?;
            let obj = self
                .repo
                .get_object(&oid)
                .map_err(|_| CliError::fatal(format!("object not found: {hex}")))?;
            if obj.kind == ObjectKind::Commit {
                self.shown.insert(oid);
            }
            self.marks.insert(oid, mark);
            self.last_mark = self.last_mark.max(mark);
        }
        Ok(())
    }

    fn export_marks(&self, file: &str) -> CliResult<()> {
        let mut v: Vec<(u32, Oid)> = Vec::new();
        for (oid, m) in &self.marks {
            if self
                .repo
                .get_object(oid)
                .is_ok_and(|o| o.kind == ObjectKind::Commit)
            {
                v.push((*m, *oid));
            }
        }
        v.sort_by_key(|(m, _)| *m);
        let text: String = v
            .iter()
            .map(|(m, oid)| format!(":{m} {}\n", oid.to_hex()))
            .collect();
        fs::write(file, text)
            .map_err(|e| CliError::fatal(format!("unable to write marks file {file}: {e}")))
    }

    fn next_mark(&mut self, oid: Oid) -> u32 {
        self.last_mark += 1;
        self.marks.insert(oid, self.last_mark);
        self.last_mark
    }

    fn progress(&mut self) {
        self.objects += 1;
        if let Some(n) = self.o.progress {
            if self.objects % n == 0 {
                self.out
                    .extend_from_slice(format!("progress {} objects\n", self.objects).as_bytes());
            }
        }
    }

    /// The ref name a label becomes after `--refspec` and `--anonymize`.
    fn export_ref(&mut self, name: &str) -> String {
        let mut name = name.to_string();
        for (src, dst) in &self.o.refspecs {
            if let Some((pre, post)) = src.split_once('*') {
                if let Some(mid) = name
                    .strip_prefix(pre)
                    .and_then(|r| r.strip_suffix(post))
                    .map(str::to_string)
                {
                    name = dst.replacen('*', &mid, 1);
                    break;
                }
            } else if &name == src {
                name = dst.clone();
                break;
            }
        }
        if self.o.anonymize {
            name = self.anon.refname(&name);
        }
        name
    }

    fn emit_blob(&mut self, oid: &Oid) -> CliResult<()> {
        if self.marks.contains_key(oid) {
            return Ok(());
        }
        let obj = self.repo.get_object(oid)?;
        let mark = self.next_mark(*oid);
        self.out
            .extend_from_slice(format!("blob\nmark :{mark}\n").as_bytes());
        if self.o.original_ids {
            self.out
                .extend_from_slice(format!("original-oid {}\n", oid.to_hex()).as_bytes());
        }
        let data = if self.o.anonymize {
            let n = self.anon.blobs.len();
            self.anon
                .blobs
                .entry(*oid)
                .or_insert_with(|| format!("anonymous blob {n}"))
                .clone()
                .into_bytes()
        } else {
            obj.payload
        };
        self.out
            .extend_from_slice(format!("data {}\n", data.len()).as_bytes());
        self.out.extend_from_slice(&data);
        self.out.push(b'\n');
        self.progress();
        Ok(())
    }

    fn tree_map(&self, tree: Option<&Oid>) -> CliResult<BTreeMap<String, (u32, Oid)>> {
        let mut m = BTreeMap::new();
        if let Some(t) = tree {
            flatten(self.repo, t, "", &mut m)?;
        }
        m.retain(|path, _| in_specs(&self.specs, path));
        Ok(m)
    }

    /// Commit labels: each commit takes the name of the first tip it was
    /// reached from, walking newest-first by committer date.
    fn sources(&self, rev: &RevOpts, tips: &revwalk::Tips) -> CliResult<HashMap<Oid, String>> {
        let mut hidden: HashSet<Oid> = HashSet::new();
        for x in &tips.exclude {
            hidden.extend(arks_repo::ancestor_set(self.repo, *x)?);
        }
        let mut src: HashMap<Oid, String> = HashMap::new();
        let mut heap: BinaryHeap<(i64, std::cmp::Reverse<u64>)> = BinaryHeap::new();
        let mut by_seq: HashMap<u64, Oid> = HashMap::new();
        let mut seq = 0u64;
        let mut seen: HashSet<Oid> = HashSet::new();
        for t in &tips.include {
            let name = tips.names.get(t).cloned().unwrap_or_else(|| t.to_hex());
            src.entry(*t)
                .or_insert_with(|| full_ref_name(self.repo, &name));
            if seen.insert(*t) {
                let c = crate::cmd::log_format::read_commit(self.repo, t)?;
                heap.push((c.committer.timestamp, std::cmp::Reverse(seq)));
                by_seq.insert(seq, *t);
                seq += 1;
            }
        }
        while let Some((_, std::cmp::Reverse(k))) = heap.pop() {
            let oid = by_seq[&k];
            if hidden.contains(&oid) {
                continue;
            }
            let c = crate::cmd::log_format::read_commit(self.repo, &oid)?;
            let name = src.get(&oid).cloned().unwrap_or_default();
            let parents: Vec<Oid> = if rev.first_parent {
                c.parents.iter().take(1).copied().collect()
            } else {
                c.parents.clone()
            };
            for p in parents {
                src.entry(p).or_insert_with(|| name.clone());
                if seen.insert(p) {
                    if let Ok(pc) = crate::cmd::log_format::read_commit(self.repo, &p) {
                        heap.push((pc.committer.timestamp, std::cmp::Reverse(seq)));
                        by_seq.insert(seq, p);
                        seq += 1;
                    }
                }
            }
        }
        Ok(src)
    }

    fn run(&mut self, rev: &RevOpts) -> CliResult<()> {
        let repo = self.repo;
        let tips = revwalk::resolve_tips(repo, rev, None)?;
        // Tag objects among the positive tips: warn early about tree tags.
        let tip_refs = positive_tip_refs(repo, rev)?;
        for (_, oid) in &tip_refs {
            if let Ok(obj) = repo.get_object(oid) {
                if obj.kind == ObjectKind::Tag {
                    if let Some(k) = tag_target_kind(repo, oid) {
                        if k == ObjectKind::Tree {
                            eprintln!(
                                "warning: Tag points to object of unexpected type tree, skipping."
                            );
                        }
                    }
                }
            }
        }
        // Sort the whole limited history first, then apply the parent-count
        // filters and the count limits; the stream is oldest first.
        let mut full = rev.clone();
        full.min_parents = None;
        full.max_parents = None;
        full.skip = 0;
        full.max_count = None;
        full.reverse = false;
        let mut list: Vec<revwalk::Walked> = revwalk::walk(repo, &full, &tips)?
            .into_iter()
            .filter(|w| {
                let n = w.commit.parents.len();
                rev.min_parents.is_none_or(|m| n >= m) && rev.max_parents.is_none_or(|m| n <= m)
            })
            .skip(rev.skip)
            .take(rev.max_count.unwrap_or(usize::MAX))
            .collect();
        list.reverse();
        let sources = self.sources(rev, &tips)?;
        if self.o.done {
            self.out.extend_from_slice(b"feature done\n");
        }
        let rewritten: HashMap<Oid, Vec<Oid>> =
            list.iter().map(|w| (w.oid, w.parents.clone())).collect();
        for w in &list {
            if self.shown.contains(&w.oid) {
                continue;
            }
            let label = sources.get(&w.oid).cloned().unwrap_or_default();
            self.export_commit(&w.oid, &w.parents, &label)?;
        }
        // Refs whose tip carries another label, then annotated tags; each
        // list last argument first.
        let mut tags: Vec<(String, Oid, Vec<u8>)> = Vec::new();
        for (name, oid) in tip_refs.iter().rev() {
            let obj = repo.get_object(oid)?;
            if obj.kind == ObjectKind::Tag {
                tags.push((name.clone(), *oid, obj.payload));
                continue;
            }
            let Ok(c) = peel_tags(repo, oid) else {
                continue;
            };
            if !name.starts_with("refs/") {
                continue;
            }
            // The exported commit standing for the tip: itself, or its nearest
            // exported ancestor through the rewritten history.
            let target = if self.marks.contains_key(&c) {
                Some(c)
            } else {
                rewrite_target(repo, &c, &rewritten, &self.marks)
            };
            let r = self.export_ref(name);
            match target {
                Some(t) => {
                    let label = sources.get(&t).cloned().unwrap_or_default();
                    if &label != name {
                        let m = self.marks[&t];
                        self.out
                            .extend_from_slice(format!("reset {r}\nfrom :{m}\n\n").as_bytes());
                    }
                }
                None => self.out.extend_from_slice(
                    format!("reset {r}\nfrom {}\n\n", "0".repeat(40)).as_bytes(),
                ),
            }
        }
        for (name, oid, payload) in &tags {
            self.export_tag(name, oid, payload, &rewritten)?;
        }
        if self.o.done {
            self.out.extend_from_slice(b"done\n");
        }
        Ok(())
    }

    fn export_commit(&mut self, oid: &Oid, parents: &[Oid], label: &str) -> CliResult<()> {
        let repo = self.repo;
        let obj = repo.get_object(oid)?;
        let rc = parse_raw_commit(&obj.payload)?;
        let first = parents.first();
        let base_tree = match first {
            Some(p) if self.marks.contains_key(p) || self.o.ref_excluded => {
                Some(crate::cmd::log_format::read_commit(repo, p)?.tree)
            }
            _ => None,
        };
        let old = if self.o.full_tree {
            BTreeMap::new()
        } else {
            self.tree_map(base_tree.as_ref())?
        };
        let new = self.tree_map(Some(&rc.tree))?;
        // Changes in tree order.
        let mut changes: Vec<(String, Option<(u32, Oid)>)> = Vec::new();
        let mut keys: Vec<&String> = old.keys().chain(new.keys()).collect();
        keys.sort();
        keys.dedup();
        for k in keys {
            let (a, b) = (old.get(k), new.get(k));
            if a == b {
                continue;
            }
            changes.push((k.clone(), b.copied()));
        }
        if !self.o.no_data {
            for (_, n) in &changes {
                if let Some((mode, boid)) = n {
                    if *mode != 0o160000 {
                        self.emit_blob(boid)?;
                    }
                }
            }
        }
        if let Some(enc) = &rc.encoding {
            if self.o.reencode == Reencode::Abort
                && !self.o.anonymize
                && !enc.eq_ignore_ascii_case("utf-8")
            {
                return Err(CliError::fatal(format!(
                    "Encountered commit-specific encoding {enc} in commit {}; use --reencode=[yes|no] to handle it",
                    oid.to_hex()
                )));
            }
        }
        changes.sort_by(|a, b| depth_first(&a.0, &b.0));
        let refname = self.export_ref(label);
        if parents.is_empty() {
            self.out
                .extend_from_slice(format!("reset {refname}\n").as_bytes());
        }
        let mark = self.next_mark(*oid);
        self.out
            .extend_from_slice(format!("commit {refname}\nmark :{mark}\n").as_bytes());
        if self.o.original_ids {
            self.out
                .extend_from_slice(format!("original-oid {}\n", oid.to_hex()).as_bytes());
        }
        let (mut author, mut committer) = (
            rc.author.clone().unwrap_or_default(),
            rc.committer.clone().unwrap_or_default(),
        );
        if self.o.anonymize {
            committer = self.anon.ident(&committer);
            author = self.anon.ident(&author);
        }
        if rc.author.is_some() {
            self.out
                .extend_from_slice(format!("author {author}\n").as_bytes());
        }
        self.out
            .extend_from_slice(format!("committer {committer}\n").as_bytes());
        if let (Some(enc), Reencode::No) = (&rc.encoding, self.o.reencode) {
            self.out
                .extend_from_slice(format!("encoding {enc}\n").as_bytes());
        }
        let msg = if self.o.anonymize {
            let n = self.anon.subjects.len();
            self.anon
                .subjects
                .entry(rc.message.clone())
                .or_insert_with(|| format!("subject {n}\n\nbody\n"))
                .clone()
                .into_bytes()
        } else {
            rc.message.clone()
        };
        self.out
            .extend_from_slice(format!("data {}\n", msg.len()).as_bytes());
        self.out.extend_from_slice(&msg);
        let mut first_line = true;
        for p in parents {
            let r = match self.marks.get(p) {
                Some(m) => format!(":{m}"),
                None if self.o.ref_excluded => {
                    if self.o.anonymize {
                        self.anon.oid(p)
                    } else {
                        p.to_hex()
                    }
                }
                None => continue,
            };
            let kw = if first_line { "from" } else { "merge" };
            first_line = false;
            self.out.extend_from_slice(format!("{kw} {r}\n").as_bytes());
        }
        if self.o.full_tree {
            self.out.extend_from_slice(b"deleteall\n");
        }
        for (path, n) in &changes {
            let shown = if self.o.anonymize {
                self.anon.path(path)
            } else {
                path.clone()
            };
            let q = quote(&shown);
            match n {
                None => self.out.extend_from_slice(format!("D {q}\n").as_bytes()),
                Some((mode, boid)) => {
                    let r = if *mode == 0o160000 {
                        if self.o.anonymize {
                            self.anon.oid(boid)
                        } else {
                            boid.to_hex()
                        }
                    } else if self.o.no_data {
                        boid.to_hex()
                    } else {
                        format!(":{}", self.marks[boid])
                    };
                    self.out
                        .extend_from_slice(format!("M {mode:06o} {r} {q}\n").as_bytes());
                }
            }
        }
        self.out.push(b'\n');
        self.progress();
        Ok(())
    }

    fn export_tag(
        &mut self,
        refname: &str,
        tag_oid: &Oid,
        payload: &[u8],
        rewritten: &HashMap<Oid, Vec<Oid>>,
    ) -> CliResult<()> {
        let repo = self.repo;
        let end = payload
            .windows(2)
            .position(|w| w == b"\n\n")
            .map(|p| p + 1)
            .unwrap_or(payload.len());
        let header = String::from_utf8_lossy(&payload[..end]).into_owned();
        let mut message = payload.get(end + 1..).unwrap_or(&[]).to_vec();
        let mut tagger: Option<String> = None;
        for line in header.lines() {
            if let Some(v) = line.strip_prefix("tagger ") {
                tagger = Some(v.to_string());
            }
        }
        if tag_target_kind(repo, tag_oid) == Some(ObjectKind::Tree) {
            eprintln!(
                "warning: Omitting tag {},\nsince tags of trees (or tags of tags of trees, etc.) are not supported.",
                tag_oid.to_hex()
            );
            return Ok(());
        }
        if let Some(pos) = has_signature(&message) {
            match self.o.signed {
                Signed::Abort => {
                    return Err(CliError::fatal(format!(
                        "encountered signed tag {}; use --signed-tags=<mode> to handle it",
                        tag_oid.to_hex()
                    )));
                }
                Signed::Warn => eprintln!("warning: exporting signed tag {}", tag_oid.to_hex()),
                Signed::WarnStrip => {
                    eprintln!("warning: stripping signature from tag {}", tag_oid.to_hex());
                    message.truncate(pos);
                }
                Signed::Strip => message.truncate(pos),
                Signed::Verbatim => {}
            }
        }
        // A tag of a tag: the inner tag goes first under the same name.
        let direct = crate::cmd::shared::tag_target(payload)
            .ok_or_else(|| CliError::fatal(format!("bad tag {}", tag_oid.to_hex())))?;
        let direct_obj = repo.get_object(&direct)?;
        let target = if direct_obj.kind == ObjectKind::Tag {
            self.export_tag(refname, &direct, &direct_obj.payload, rewritten)?;
            direct
        } else {
            peel_tags(repo, tag_oid)?
        };
        let from = match self.marks.get(&target) {
            Some(m) => format!(":{m}"),
            None => match self.o.filtered {
                Filtered::Abort => {
                    return Err(CliError::fatal(format!(
                        "tag {} tags unexported object; use --tag-of-filtered-object=<mode> to handle it",
                        tag_oid.to_hex()
                    )));
                }
                Filtered::Drop => return Ok(()),
                Filtered::Rewrite => match rewrite_target(repo, &target, rewritten, &self.marks) {
                    Some(a) => match self.marks.get(&a) {
                        Some(m) => format!(":{m}"),
                        None => a.to_hex(),
                    },
                    None => target.to_hex(),
                },
            },
        };
        let short = refname
            .strip_prefix("refs/tags/")
            .unwrap_or(refname)
            .to_string();
        let mut name = self.export_ref(&format!("refs/tags/{short}"));
        name = name.strip_prefix("refs/tags/").unwrap_or(&name).to_string();
        self.out
            .extend_from_slice(format!("tag {name}\n").as_bytes());
        if self.o.mark_tags {
            let m = self.next_mark(*tag_oid);
            self.out
                .extend_from_slice(format!("mark :{m}\n").as_bytes());
        }
        self.out
            .extend_from_slice(format!("from {from}\n").as_bytes());
        if self.o.original_ids {
            self.out
                .extend_from_slice(format!("original-oid {}\n", tag_oid.to_hex()).as_bytes());
        }
        match tagger {
            Some(t) => {
                let t = if self.o.anonymize {
                    self.anon.ident(&t)
                } else {
                    t
                };
                self.out
                    .extend_from_slice(format!("tagger {t}\n").as_bytes());
            }
            None if self.o.fake_tagger => self
                .out
                .extend_from_slice(b"tagger Unspecified Tagger <unspecified-tagger> 0 +0000\n"),
            None => {}
        }
        if self.o.anonymize {
            let n = self.anon.tag_msgs.len();
            message = self
                .anon
                .tag_msgs
                .entry(message)
                .or_insert_with(|| format!("tag message {n}"))
                .clone()
                .into_bytes();
        }
        self.out
            .extend_from_slice(format!("data {}\n", message.len()).as_bytes());
        self.out.extend_from_slice(&message);
        self.out.push(b'\n');
        self.progress();
        Ok(())
    }
}

/// `rev-parse --symbolic-full-name` of a tip name; other names stay as given.
fn full_ref_name(repo: &Repository, name: &str) -> String {
    if name.starts_with("refs/") {
        return name.to_string();
    }
    if name == "HEAD" {
        if let Ok(arks_refs::RefTarget::Symbolic(t)) = repo.head_target() {
            return t;
        }
        return name.to_string();
    }
    for c in [
        format!("refs/{name}"),
        format!("refs/tags/{name}"),
        format!("refs/heads/{name}"),
        format!("refs/remotes/{name}"),
        format!("refs/remotes/{name}/HEAD"),
    ] {
        if repo.refs().resolve(&c).is_ok() {
            return c;
        }
    }
    name.to_string()
}

fn tag_target_kind(repo: &Repository, tag: &Oid) -> Option<ObjectKind> {
    let t = peel_tags(repo, tag).ok()?;
    repo.get_object(&t).ok().map(|o| o.kind)
}

/// The positive revision arguments that name refs, as (full name, object).
fn positive_tip_refs(repo: &Repository, rev: &RevOpts) -> CliResult<Vec<(String, Oid)>> {
    let mut out: Vec<(String, Oid)> = Vec::new();
    for r in &rev.revs {
        if r.starts_with('^') {
            continue;
        }
        if let Some(code) = r.strip_prefix('\u{0}') {
            for name in revwalk::select_refs(repo, code)? {
                if name == "HEAD" {
                    continue;
                }
                if let Ok(oid) = repo.refs().resolve(&name) {
                    out.push((name, oid));
                }
            }
            continue;
        }
        let names: Vec<&str> = match r.split_once("...") {
            Some((a, b)) => vec![a, b],
            None => match r.split_once("..") {
                Some((_, b)) => vec![b],
                None => vec![r.as_str()],
            },
        };
        for name in names {
            let name = if name.is_empty() { "HEAD" } else { name };
            let full = full_ref_name(repo, name);
            if !full.starts_with("refs/") {
                continue;
            }
            if let Ok(oid) = repo.refs().resolve(&full) {
                if !out.iter().any(|(n, _)| n == &full) {
                    out.push((full, oid));
                }
            }
        }
    }
    Ok(out)
}

/// The nearest exported commit at or below `c` through the walk's
/// (rewritten) parents, following first parents.
fn rewrite_target(
    repo: &Repository,
    c: &Oid,
    rewritten: &HashMap<Oid, Vec<Oid>>,
    marks: &HashMap<Oid, u32>,
) -> Option<Oid> {
    let mut cur = *c;
    for _ in 0..1_000_000 {
        if marks.contains_key(&cur) {
            return Some(cur);
        }
        let parents = match rewritten.get(&cur) {
            Some(p) => p.clone(),
            None => {
                crate::cmd::log_format::read_commit(repo, &cur)
                    .ok()?
                    .parents
            }
        };
        cur = *parents.first()?;
    }
    None
}
