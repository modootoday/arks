//! `git submodule`: add, status, init, deinit, update, set-branch, set-url,
//! summary, foreach, sync and absorbgitdirs, with stock git's option
//! spellings (the subcommands take separate-word values, as git's own
//! front end does; `--branch=x` and bundled shorts are usage errors there).
//!
//! Cite: https://git-scm.com/docs/git-submodule
//! Cite: https://git-scm.com/docs/gitmodules
//! Cite: https://git-scm.com/docs/gitsubmodules
//! Plan: `.spec/plan/20261120000000-argv-symmetry-ladder.plan.md` (rung A2)
//!
//! Partial: `status` names commits with a describe-like search over tags and
//! refs (nearest by breadth-first distance); clones of non-local URLs keep the
//! arks-remote transports; `update --remote` and fetches use local transports.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree, write_index};
use arks_index::{IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: git submodule [--quiet] [--cached]
   or: git submodule [--quiet] add [-b <branch>] [-f|--force] [--name <name>] [--reference <repository>] [--] <repository> [<path>]
   or: git submodule [--quiet] status [--cached] [--recursive] [--] [<path>...]
   or: git submodule [--quiet] init [--] [<path>...]
   or: git submodule [--quiet] deinit [-f|--force] (--all| [--] <path>...)
   or: git submodule [--quiet] update [--init [--filter=<filter-spec>]] [--remote] [-N|--no-fetch] [-f|--force] [--checkout|--merge|--rebase] [--[no-]recommend-shallow] [--reference <repository>] [--recursive] [--[no-]single-branch] [--] [<path>...]
   or: git submodule [--quiet] set-branch (--default|--branch <branch>) [--] <path>
   or: git submodule [--quiet] set-url [--] <path> <newurl>
   or: git submodule [--quiet] summary [--cached|--files] [--summary-limit <n>] [commit] [--] [<path>...]
   or: git submodule [--quiet] foreach [--recursive] <command>
   or: git submodule [--quiet] sync [--recursive] [--] [<path>...]
   or: git submodule [--quiet] absorbgitdirs [--] [<path>...]";

const COMMANDS: &[&str] = &[
    "add",
    "foreach",
    "init",
    "deinit",
    "update",
    "set-branch",
    "set-url",
    "status",
    "summary",
    "sync",
    "absorbgitdirs",
];

fn usage() -> CliError {
    CliError::Exit(1, USAGE.into())
}

pub fn cmd_submodule(args: &[String]) -> CliResult<()> {
    let mut quiet = false;
    let mut cached = false;
    let mut command: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-q" | "--quiet" => quiet = true,
            "--cached" => cached = true,
            "--" => {
                i += 1;
                break;
            }
            a if COMMANDS.contains(&a) => {
                command = Some(COMMANDS[COMMANDS.iter().position(|c| *c == a).unwrap_or(0)]);
                i += 1;
                break;
            }
            a if a.starts_with('-') => return Err(usage()),
            _ => break,
        }
        i += 1;
    }
    let rest = &args[i..];
    let command = match command {
        Some(c) => c,
        None if rest.is_empty() => "status",
        None => return Err(usage()),
    };
    if cached && !matches!(command, "status" | "summary") {
        return Err(usage());
    }
    let mut lead: Vec<String> = Vec::new();
    if quiet {
        lead.push("--quiet".into());
    }
    if cached {
        lead.push("--cached".into());
    }
    lead.extend(rest.iter().cloned());
    let rest = &lead[..];
    match command {
        "add" => sm_add(rest),
        "init" => sm_init(rest),
        "update" => sm_update(rest),
        "status" => sm_status(rest),
        "deinit" => sm_deinit(rest),
        "sync" => sm_sync(rest),
        "summary" => sm_summary(rest),
        "foreach" => sm_foreach(rest),
        "set-branch" => sm_set_branch(rest),
        "set-url" => sm_set_url(rest),
        _ => sm_absorbgitdirs(rest),
    }
}

// ---------------------------------------------------------------- context --

/// The superproject a subcommand runs in.
struct Super {
    repo: Repository,
    work: PathBuf,
    /// Current directory relative to the work tree (`""` or `dir/`).
    prefix: String,
}

impl Super {
    fn open() -> CliResult<Self> {
        let repo = open_repo()?;
        let work = canonical(&work_tree(&repo)?);
        let cwd = canonical(&std::env::current_dir()?);
        let prefix = cwd
            .strip_prefix(&work)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .filter(|p| !p.is_empty())
            .map(|p| format!("{p}/"))
            .unwrap_or_default();
        Ok(Self { repo, work, prefix })
    }

    fn config_path(&self) -> PathBuf {
        self.repo.common_dir().join("config")
    }

    fn config(&self) -> arks_config::ConfigSet {
        arks_config::ConfigSet::load(Some(self.repo.common_dir()))
    }

    fn gitmodules_path(&self) -> PathBuf {
        self.work.join(".gitmodules")
    }

    fn gitmodules(&self) -> Vec<Module> {
        parse_gitmodules(&fs::read_to_string(self.gitmodules_path()).unwrap_or_default())
    }

    /// Path shown to the user: relative to the current directory.
    fn display(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            return path.to_string();
        }
        if let Some(rest) = path.strip_prefix(&self.prefix) {
            return rest.to_string();
        }
        let ups = self.prefix.matches('/').count();
        format!("{}{path}", "../".repeat(ups))
    }

    /// `.git/modules/<name>`.
    fn module_dir(&self, name: &str) -> PathBuf {
        self.repo.common_dir().join("modules").join(name)
    }

    /// Base for relative submodule URLs: the default remote's URL, else the
    /// superproject's work tree.
    fn url_base(&self) -> String {
        let cfg = self.config();
        let branch = match self.repo.head_target() {
            Ok(RefTarget::Symbolic(r)) => r.strip_prefix("refs/heads/").map(String::from),
            _ => None,
        };
        let remote = branch
            .and_then(|b| cfg.get(&format!("branch.{b}.remote")))
            .unwrap_or_else(|| "origin".into());
        cfg.get(&format!("remote.{remote}.url"))
            .unwrap_or_else(|| self.work.to_string_lossy().replace('\\', "/"))
    }

    fn resolve_url(&self, url: &str) -> String {
        if is_relative_url(url) {
            resolve_relative_url(&self.url_base(), url)
        } else {
            url.to_string()
        }
    }
}

fn canonical(p: &Path) -> PathBuf {
    strip_verbatim(fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()))
}

fn strip_verbatim(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => path,
    }
}

/// One `[submodule "<name>"]` section of `.gitmodules`.
#[derive(Debug, Clone, Default)]
struct Module {
    name: String,
    path: String,
    url: String,
    branch: Option<String>,
    update: Option<String>,
}

fn parse_gitmodules(text: &str) -> Vec<Module> {
    let mut out: Vec<Module> = Vec::new();
    let mut cur: Option<usize> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            cur = parse_submodule_section(t).map(|name| {
                match out.iter().position(|m| m.name == name) {
                    Some(i) => i,
                    None => {
                        out.push(Module {
                            name,
                            ..Default::default()
                        });
                        out.len() - 1
                    }
                }
            });
            continue;
        }
        let (Some(i), Some((k, v))) = (cur, t.split_once('=')) else {
            continue;
        };
        let v = unquote(v.trim());
        match k.trim().to_ascii_lowercase().as_str() {
            "path" => out[i].path = v,
            "url" => out[i].url = v,
            "branch" => out[i].branch = Some(v),
            "update" => out[i].update = Some(v),
            _ => {}
        }
    }
    out.retain(|m| !m.path.is_empty());
    out
}

fn unquote(v: &str) -> String {
    v.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(v)
        .to_string()
}

fn parse_submodule_section(header: &str) -> Option<String> {
    let inner = header.trim().strip_prefix('[')?.strip_suffix(']')?;
    let rest = inner.strip_prefix("submodule")?.trim();
    let name = rest.strip_prefix('"')?.strip_suffix('"')?;
    (!name.is_empty()).then(|| name.to_string())
}

fn is_relative_url(url: &str) -> bool {
    url.starts_with("./") || url.starts_with("../")
}

/// Resolve `./x` / `../x` against `base` (git-submodule: relative URLs are
/// relative to the superproject's default remote).
fn resolve_relative_url(base: &str, url: &str) -> String {
    let mut base = base.trim_end_matches('/').to_string();
    let mut rel = url;
    loop {
        if let Some(r) = rel.strip_prefix("../") {
            match base.rfind('/') {
                Some(i) => base.truncate(i),
                None => base = ".".into(),
            }
            rel = r;
        } else if let Some(r) = rel.strip_prefix("./") {
            rel = r;
        } else {
            break;
        }
    }
    format!("{base}/{rel}")
}

// --------------------------------------------------------- config editing --

fn section_name(section: &str) -> String {
    match section.split_once('.') {
        Some((s, sub)) => format!("[{s} \"{sub}\"]"),
        None => format!("[{section}]"),
    }
}

fn header_matches(line: &str, header: &str) -> bool {
    let t = line.trim();
    if t == header {
        return true;
    }
    // Case-insensitive section name, exact subsection.
    match (t.split_once(' '), header.split_once(' ')) {
        (Some((a, x)), Some((b, y))) => a.eq_ignore_ascii_case(b) && x == y,
        (None, None) => t.eq_ignore_ascii_case(header),
        _ => false,
    }
}

fn line_key(line: &str) -> Option<String> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with(';') || t.starts_with('[') {
        return None;
    }
    let k = t.split_once('=').map(|(k, _)| k).unwrap_or(t);
    Some(k.trim().to_ascii_lowercase())
}

/// Set `key` in `section` (`submodule.<name>` / `core`), replacing the last
/// existing value or appending to the section (or a new section).
fn cfg_set(text: &str, section: &str, key: &str, value: &str) -> String {
    let header = section_name(section);
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut in_sec = false;
    let mut last_key: Option<usize> = None;
    let mut sec_end: Option<usize> = None;
    for (i, l) in lines.iter().enumerate() {
        if l.trim().starts_with('[') {
            in_sec = header_matches(l, &header);
            if in_sec {
                sec_end = Some(i + 1);
            }
            continue;
        }
        if in_sec {
            sec_end = Some(i + 1);
            if line_key(l).as_deref() == Some(&key.to_ascii_lowercase()) {
                last_key = Some(i);
            }
        }
    }
    let new_line = format!("\t{key} = {}\n", quote_value(value));
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    if let Some(i) = last_key {
        out[i] = new_line;
    } else if let Some(end) = sec_end {
        if end > 0 && !out[end - 1].ends_with('\n') {
            out[end - 1].push('\n');
        }
        out.insert(end, new_line);
    } else {
        if let Some(last) = out.last_mut() {
            if !last.ends_with('\n') {
                last.push('\n');
            }
        }
        out.push(format!("{header}\n"));
        out.push(new_line);
    }
    out.concat()
}

fn quote_value(v: &str) -> String {
    if v.starts_with(' ') || v.ends_with(' ') || v.contains(['#', ';']) {
        format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        v.to_string()
    }
}

/// Remove `key` from `section`; `None` when it was not set.
fn cfg_unset(text: &str, section: &str, key: &str) -> Option<String> {
    let header = section_name(section);
    let mut in_sec = false;
    let mut hit = false;
    let mut out = String::new();
    for l in text.split_inclusive('\n') {
        if l.trim().starts_with('[') {
            in_sec = header_matches(l, &header);
        } else if in_sec && line_key(l).as_deref() == Some(&key.to_ascii_lowercase()) {
            hit = true;
            continue;
        }
        out.push_str(l);
    }
    hit.then_some(out)
}

fn cfg_remove_section(text: &str, section: &str) -> String {
    let header = section_name(section);
    let mut skip = false;
    let mut out = String::new();
    for l in text.split_inclusive('\n') {
        if l.trim().starts_with('[') {
            skip = header_matches(l, &header);
        }
        if !skip {
            out.push_str(l);
        }
    }
    out
}

/// Rewrite a config file under git's `<file>.lock` protocol: a held lock
/// (another writer, e.g. SUB-009 sync vs update) is refused, not raced.
fn edit_file(path: &Path, f: impl FnOnce(&str) -> String) -> CliResult<()> {
    let mut lock_name = path.as_os_str().to_owned();
    lock_name.push(".lock");
    let lock = PathBuf::from(lock_name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|_| {
            CliError::Exit(
                255,
                format!(
                    "error: could not lock config file {}: File exists",
                    path.display()
                ),
            )
        })?;
    let text = fs::read_to_string(path).unwrap_or_default();
    let res = std::io::Write::write_all(&mut file, f(&text).as_bytes())
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&lock, path));
    if res.is_err() {
        let _ = fs::remove_file(&lock);
    }
    res?;
    Ok(())
}

// ----------------------------------------------------------- module list --

/// A gitlink in the index together with its `.gitmodules` entry.
#[derive(Debug, Clone)]
struct Entry {
    path: String,
    oid: Oid,
    stage: u16,
    module: Option<Module>,
}

/// Gitlinks matching `specs` (cwd-relative pathspecs), in index order.
/// A pathspec that matches nothing in the index is git's error (exit 1).
fn module_list(sp: &Super, specs: &[String]) -> CliResult<Vec<Entry>> {
    let index = read_index(&sp.repo)?;
    let mods = sp.gitmodules();
    let full: Vec<String> = specs
        .iter()
        .map(|s| normalize(&format!("{}{s}", sp.prefix)))
        .collect();
    let hit = |p: &str, f: &str| f.is_empty() || p == f || p.starts_with(&format!("{f}/"));
    for (spec, f) in specs.iter().zip(&full) {
        if !index.entries.iter().any(|e| hit(&e.path, f)) {
            return Err(CliError::Exit(
                1,
                format!("error: pathspec '{spec}' did not match any file(s) known to git"),
            ));
        }
    }
    let mut out: Vec<Entry> = Vec::new();
    for e in index.entries.iter().filter(|e| e.mode == 0o160000) {
        if !full.is_empty() && !full.iter().any(|f| hit(&e.path, f)) {
            continue;
        }
        if out.iter().any(|o| o.path == e.path) {
            continue;
        }
        out.push(Entry {
            path: e.path.clone(),
            oid: e.oid,
            stage: e.stage(),
            module: mods.iter().find(|m| m.path == e.path).cloned(),
        });
    }
    Ok(out)
}

fn normalize(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for c in path.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

fn is_active(cfg: &arks_config::ConfigSet, name: &str) -> bool {
    match cfg.get_bool(&format!("submodule.{name}.active")) {
        Some(b) => b,
        None => cfg.get(&format!("submodule.{name}.url")).is_some(),
    }
}

/// Opened submodule repository when `path` is a populated checkout.
fn open_populated(sp: &Super, path: &str) -> Option<Repository> {
    let abs = sp.work.join(path);
    if !abs.join(".git").exists() {
        return None;
    }
    Repository::discover(&abs).ok().filter(|r| {
        r.work_tree()
            .map(|w| canonical(w) == canonical(&abs))
            .unwrap_or(false)
    })
}

/// Run `f` with the current directory switched to `dir`.
fn in_dir<T>(dir: &Path, f: impl FnOnce() -> CliResult<T>) -> CliResult<T> {
    let prev = std::env::current_dir()?;
    std::env::set_current_dir(dir)?;
    let r = f();
    let _ = std::env::set_current_dir(prev);
    r
}

// ------------------------------------------------------------------ opts --

/// Hand-rolled option walk for one subcommand: `flags` are exact words,
/// `valued` take the next word (or `--x=<v>` when listed with `=`).
struct Opts {
    seen: Vec<(String, Option<String>)>,
    args: Vec<String>,
}

impl Opts {
    fn parse(args: &[String], flags: &[&str], valued: &[&str]) -> CliResult<Self> {
        let mut seen = Vec::new();
        let mut rest = Vec::new();
        let mut i = 0;
        while i < args.len() {
            let a = args[i].as_str();
            if a == "--" {
                rest.extend(args[i + 1..].iter().cloned());
                break;
            }
            if !a.starts_with('-') || a == "-" {
                rest.extend(args[i..].iter().cloned());
                break;
            }
            if flags.contains(&a) {
                seen.push((a.to_string(), None));
            } else if valued.contains(&a) {
                let v = args.get(i + 1).cloned().ok_or_else(usage)?;
                seen.push((a.to_string(), Some(v)));
                i += 1;
            } else if let Some((k, v)) = a
                .split_once('=')
                .filter(|(k, _)| valued.contains(&format!("{k}=").as_str()))
            {
                seen.push((k.to_string(), Some(v.to_string())));
            } else {
                return Err(usage());
            }
            i += 1;
        }
        Ok(Self { seen, args: rest })
    }

    fn on(&self, names: &[&str]) -> bool {
        self.seen.iter().any(|(k, _)| names.contains(&k.as_str()))
    }

    fn value(&self, names: &[&str]) -> Option<String> {
        self.seen
            .iter()
            .rev()
            .find(|(k, _)| names.contains(&k.as_str()))
            .and_then(|(_, v)| v.clone())
    }
}

// ---------------------------------------------------------------- status --

fn sm_status(args: &[String]) -> CliResult<()> {
    let o = Opts::parse(args, &["-q", "--quiet", "--cached", "--recursive"], &[])?;
    let sp = Super::open()?;
    status_in(
        &sp,
        &o.args,
        o.on(&["--cached"]),
        o.on(&["--recursive"]),
        o.on(&["-q", "--quiet"]),
        "",
    )
}

fn status_in(
    sp: &Super,
    specs: &[String],
    cached: bool,
    recursive: bool,
    quiet: bool,
    outer: &str,
) -> CliResult<()> {
    let cfg = sp.config();
    for e in module_list(sp, specs)? {
        let display = format!("{outer}{}", sp.display(&e.path));
        if e.stage != 0 {
            if !quiet {
                println!("U{} {display}", "0".repeat(e.oid.to_hex().len()));
            }
            continue;
        }
        let Some(m) = &e.module else { continue };
        let sub = open_populated(sp, &e.path).filter(|_| is_active(&cfg, &m.name));
        let Some(sub) = sub else {
            if !quiet {
                println!("-{} {display}", e.oid.to_hex());
            }
            continue;
        };
        let head = sub.head_oid().ok();
        let flag = if head == Some(e.oid) { ' ' } else { '+' };
        let shown = if cached { e.oid } else { head.unwrap_or(e.oid) };
        if !quiet {
            match describe(&sub, &shown) {
                Some(d) => println!("{flag}{} {display} ({d})", shown.to_hex()),
                None => println!("{flag}{} {display}", shown.to_hex()),
            }
        }
        if recursive {
            let abs = sp.work.join(&e.path);
            let nested_outer = format!("{display}/");
            in_dir(&abs, || {
                let inner = Super::open()?;
                status_in(&inner, &[], cached, true, quiet, &nested_outer)
            })?;
        }
    }
    Ok(())
}

/// git-submodule status names a commit by `describe`, then `describe
/// --tags`, `describe --contains`, `describe --all --always`.
fn describe(repo: &Repository, oid: &Oid) -> Option<String> {
    let refs = repo.refs().list_refs("refs/").ok()?;
    let peel = |o: &Oid| crate::cmd::remote_local::peel(repo, o);
    let annotated = |o: &Oid| {
        repo.get_object(o)
            .map(|x| x.kind == ObjectKind::Tag)
            .unwrap_or(false)
    };
    let tags: Vec<(String, Oid, bool)> = refs
        .iter()
        .filter_map(|(n, o)| {
            n.strip_prefix("refs/tags/")
                .map(|t| (t.to_string(), peel(o), annotated(o)))
        })
        .collect();
    let abbrev = crate::cmd::shared::abbrev_hex(repo, oid, 7);
    let nearest = |names: &[(String, Oid)]| -> Option<String> {
        let mut seen = HashSet::new();
        let mut queue = VecDeque::from([(*oid, 0usize)]);
        while let Some((c, depth)) = queue.pop_front() {
            if !seen.insert(c) {
                continue;
            }
            let mut hits: Vec<&String> = names
                .iter()
                .filter(|(_, o)| *o == c)
                .map(|(n, _)| n)
                .collect();
            hits.sort();
            if let Some(n) = hits.first() {
                return Some(if depth == 0 {
                    (*n).clone()
                } else {
                    format!("{n}-{depth}-g{abbrev}")
                });
            }
            for p in parents(repo, &c) {
                queue.push_back((p, depth + 1));
            }
        }
        None
    };
    let ann: Vec<(String, Oid)> = tags
        .iter()
        .filter(|t| t.2)
        .map(|t| (t.0.clone(), t.1))
        .collect();
    let all: Vec<(String, Oid)> = tags.iter().map(|t| (t.0.clone(), t.1)).collect();
    if let Some(d) = nearest(&ann).or_else(|| nearest(&all)) {
        return Some(d);
    }
    // --contains: a tag whose first-parent chain reaches the commit.
    let mut best: Option<(usize, String)> = None;
    for (name, tip) in &all {
        let mut cur = Some(*tip);
        let mut n = 0;
        while let Some(c) = cur {
            if c == *oid {
                if best.as_ref().is_none_or(|b| n < b.0) {
                    best = Some((n, format!("{name}~{n}")));
                }
                break;
            }
            cur = parents(repo, &c).first().copied();
            n += 1;
        }
    }
    if let Some((_, d)) = best {
        return Some(d);
    }
    let named: Vec<(String, Oid)> = refs
        .iter()
        .map(|(n, o)| (n.strip_prefix("refs/").unwrap_or(n).to_string(), peel(o)))
        .collect();
    nearest(&named).or(Some(abbrev))
}

fn parents(repo: &Repository, oid: &Oid) -> Vec<Oid> {
    repo.get_object(oid)
        .ok()
        .and_then(|o| Commit::decode(&o.payload).ok())
        .map(|c| c.parents)
        .unwrap_or_default()
}

// ------------------------------------------------------------------ init --

fn sm_init(args: &[String]) -> CliResult<()> {
    let o = Opts::parse(args, &["-q", "--quiet"], &[])?;
    let sp = Super::open()?;
    init_modules(&sp, &o.args, o.on(&["-q", "--quiet"]))
}

/// Register `.gitmodules` URLs in the repository config (`active` + `url`).
fn init_modules(sp: &Super, specs: &[String], quiet: bool) -> CliResult<()> {
    let list = module_list(sp, specs)?;
    let cfg_path = sp.config_path();
    for e in list {
        let Some(m) = &e.module else { continue };
        let cfg = sp.config();
        if cfg.get(&format!("submodule.{}.url", m.name)).is_some() {
            continue;
        }
        let url = sp.resolve_url(&m.url);
        let sec = format!("submodule.{}", m.name);
        edit_file(&cfg_path, |t| {
            let t = if cfg.get(&format!("submodule.{}.active", m.name)).is_none()
                && cfg.get("submodule.active").is_none()
            {
                cfg_set(t, &sec, "active", "true")
            } else {
                t.to_string()
            };
            cfg_set(&t, &sec, "url", &url)
        })?;
        if let Some(u) = m.update.as_deref().filter(|u| !u.starts_with('!')) {
            edit_file(&cfg_path, |t| cfg_set(t, &sec, "update", u))?;
        }
        if !quiet {
            eprintln!(
                "Submodule '{}' ({url}) registered for path '{}'",
                m.name,
                sp.display(&e.path)
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- update --

struct UpdateOpts {
    quiet: bool,
    /// `--init`: nested submodules are initialised too under `--recursive`.
    init: bool,
    force: bool,
    remote: bool,
    no_fetch: bool,
    mode: Option<String>,
    recursive: bool,
    depth: Option<String>,
    filter: Option<String>,
    single_branch: Option<bool>,
}

fn sm_update(args: &[String]) -> CliResult<()> {
    let o = Opts::parse(
        args,
        &[
            "-q",
            "--quiet",
            "--progress",
            "-i",
            "--init",
            "--require-init",
            "--remote",
            "-N",
            "--no-fetch",
            "-f",
            "--force",
            "-r",
            "--rebase",
            "--dissociate",
            "-m",
            "--merge",
            "--recursive",
            "--checkout",
            "--recommend-shallow",
            "--no-recommend-shallow",
            "--single-branch",
            "--no-single-branch",
        ],
        &[
            "--reference",
            "--reference=",
            "--depth",
            "--depth=",
            "-j",
            "--jobs",
            "--jobs=",
            "--filter",
            "--filter=",
        ],
    )?;
    let init = o.on(&["-i", "--init", "--require-init"]);
    if o.value(&["--filter"]).is_some() && !init {
        return Err(CliError::Exit(
            129,
            "usage: git submodule [--quiet] update [--init [--filter=<filter-spec>]] [--remote] [-N|--no-fetch] [-f|--force] [--checkout|--merge|--rebase] [--[no-]recommend-shallow] [--reference <repository>] [--recursive] [--[no-]single-branch] [--] [<path>...]".into(),
        ));
    }
    if let Some(j) = o.value(&["-j", "--jobs"]) {
        j.parse::<i64>().map_err(|_| {
            CliError::Exit(129, "error: option `jobs' expects a numerical value".into())
        })?;
    }
    let mode = if o.on(&["-r", "--rebase"]) {
        Some("rebase".to_string())
    } else if o.on(&["-m", "--merge"]) {
        Some("merge".to_string())
    } else if o.on(&["--checkout"]) {
        Some("checkout".to_string())
    } else {
        None
    };
    let uo = UpdateOpts {
        quiet: o.on(&["-q", "--quiet"]),
        init,
        force: o.on(&["-f", "--force"]),
        remote: o.on(&["--remote"]),
        no_fetch: o.on(&["-N", "--no-fetch"]),
        mode,
        recursive: o.on(&["--recursive"]),
        depth: o.value(&["--depth"]),
        filter: o.value(&["--filter"]),
        single_branch: if o.on(&["--single-branch"]) {
            Some(true)
        } else if o.on(&["--no-single-branch"]) {
            Some(false)
        } else {
            None
        },
    };
    let sp = Super::open()?;
    if init {
        init_modules(&sp, &o.args, uo.quiet)?;
    }
    update_in(&sp, &o.args, &uo, "")
}

fn update_in(sp: &Super, specs: &[String], o: &UpdateOpts, outer: &str) -> CliResult<()> {
    let cfg = sp.config();
    for e in module_list(sp, specs)? {
        let display = format!("{outer}{}", sp.display(&e.path));
        if e.stage != 0 {
            eprintln!("Skipping unmerged submodule {display}");
            continue;
        }
        let Some(m) = &e.module else { continue };
        if !is_active(&cfg, &m.name) {
            continue;
        }
        let mode = o
            .mode
            .clone()
            .or_else(|| cfg.get(&format!("submodule.{}.update", m.name)))
            .unwrap_or_else(|| "checkout".into());
        if mode == "none" {
            if !o.quiet {
                println!("Skipping submodule '{display}'");
            }
            continue;
        }
        let url = cfg
            .get(&format!("submodule.{}.url", m.name))
            .unwrap_or_else(|| sp.resolve_url(&m.url));
        let abs = sp.work.join(&e.path);
        let mut just_cloned = false;
        if open_populated(sp, &e.path).is_none() {
            clone_submodule(sp, m, &e.path, &url, None, o, &display)?;
            just_cloned = true;
        }
        let sub = open_populated(sp, &e.path)
            .ok_or_else(|| CliError::fatal(format!("Unable to find submodule '{display}'")))?;
        let head = sub.head_oid().ok();
        if o.remote && !o.no_fetch && !just_cloned {
            in_dir(&abs, || crate::cmd::fetch::cmd_fetch(&["-q".to_string()])).map_err(|_| {
                CliError::fatal(format!("Unable to fetch in submodule path '{display}'"))
            })?;
        }
        let target = if o.remote {
            remote_tip(sp, &sub, m)?
        } else {
            e.oid
        };
        let sub = Repository::discover(&abs)?;
        if sub.get_object(&target).is_err() && !o.no_fetch {
            in_dir(&abs, || crate::cmd::fetch::cmd_fetch(&["-q".to_string()]))?;
        }
        let sub = Repository::discover(&abs)?;
        if sub.get_object(&target).is_err() {
            return Err(CliError::fatal(format!(
                "Fetched in submodule path '{display}', but it did not contain {}. Direct fetching of that commit failed.",
                target.to_hex()
            )));
        }
        // A fresh clone is always checked out, whatever the update mode.
        let mode = if just_cloned {
            "checkout".to_string()
        } else {
            mode
        };
        if head != Some(target) || o.force || just_cloned {
            match mode.as_str() {
                "merge" => {
                    in_dir(&abs, || {
                        let mut a = vec![target.to_hex()];
                        if o.quiet {
                            a.insert(0, "-q".into());
                        }
                        crate::cmd::merge::cmd_merge(&a)
                    })
                    .map_err(|_| {
                        CliError::fatal(format!(
                            "Unable to merge '{}' in submodule path '{display}'",
                            target.to_hex()
                        ))
                    })?;
                    if !o.quiet {
                        println!(
                            "Submodule path '{display}': merged in '{}'",
                            target.to_hex()
                        );
                    }
                }
                "rebase" => {
                    in_dir(&abs, || match (head, sub.head_target()) {
                        // Upstream already in history: git-rebase reports
                        // the branch up to date and changes nothing.
                        (Some(h), Ok(RefTarget::Symbolic(r)))
                            if crate::cmd::shared::is_ancestor(&sub, &target, &h)? =>
                        {
                            if !o.quiet {
                                println!(
                                    "Current branch {} is up to date.",
                                    r.trim_start_matches("refs/heads/")
                                );
                            }
                            Ok(())
                        }
                        _ => crate::cmd::replay::cmd_rebase(&[target.to_hex()]),
                    })
                    .map_err(|_| {
                        CliError::fatal(format!(
                            "Unable to rebase '{}' in submodule path '{display}'",
                            target.to_hex()
                        ))
                    })?;
                    if !o.quiet {
                        println!(
                            "Submodule path '{display}': rebased into '{}'",
                            target.to_hex()
                        );
                    }
                }
                _ => {
                    detach_checkout(&sub, &abs, head.as_ref(), &target, o.force || just_cloned)
                        .map_err(|err| {
                            eprintln!("{err}");
                            CliError::fatal(format!(
                                "Unable to checkout '{}' in submodule path '{display}'",
                                target.to_hex()
                            ))
                        })?;
                    if !o.quiet {
                        println!(
                            "Submodule path '{display}': checked out '{}'",
                            target.to_hex()
                        );
                    }
                }
            }
        }
        if o.recursive {
            let nested_outer = format!("{display}/");
            in_dir(&abs, || {
                let inner = Super::open()?;
                if o.init {
                    init_modules(&inner, &[], o.quiet)?;
                }
                update_in(&inner, &[], o, &nested_outer)
            })?;
        }
    }
    Ok(())
}

/// `update --remote`: the submodule's remote-tracking branch named by
/// `submodule.<name>.branch` (config, then `.gitmodules`), default the remote HEAD.
fn remote_tip(sp: &Super, sub: &Repository, m: &Module) -> CliResult<Oid> {
    let cfg = sp.config();
    let branch = cfg
        .get(&format!("submodule.{}.branch", m.name))
        .or_else(|| m.branch.clone())
        .unwrap_or_else(|| "HEAD".into());
    let branch = if branch == "." {
        match sp.repo.head_target()? {
            RefTarget::Symbolic(r) => r.trim_start_matches("refs/heads/").to_string(),
            RefTarget::Direct(_) => "HEAD".into(),
        }
    } else {
        branch
    };
    let scfg = arks_config::ConfigSet::load(Some(sub.common_dir()));
    let remote = match sub.head_target() {
        Ok(RefTarget::Symbolic(r)) => r
            .strip_prefix("refs/heads/")
            .and_then(|b| scfg.get(&format!("branch.{b}.remote"))),
        _ => None,
    }
    .unwrap_or_else(|| "origin".into());
    let full = format!("refs/remotes/{remote}/{branch}");
    crate::cmd::shared::resolve_rev(sub, &full).map_err(|_| {
        CliError::fatal(format!(
            "Unable to find current {remote}/{branch} revision in submodule path '{}'",
            sp.display(&m.path)
        ))
    })
}

/// Detach the submodule's HEAD at `to`, carrying the work tree (forced: overwrite).
fn detach_checkout(
    sub: &Repository,
    abs: &Path,
    from: Option<&Oid>,
    to: &Oid,
    force: bool,
) -> CliResult<()> {
    if force || from.is_none() {
        arks_remote::checkout_commit(sub, abs, to).map_err(|e| CliError::Msg(e.to_string()))?;
    } else {
        crate::cmd::switch_tree::two_way_switch(sub, abs, from, to)?;
    }
    fs::write(sub.git_dir().join("HEAD"), format!("{}\n", to.to_hex()))?;
    Ok(())
}

/// Clone `url` into `.git/modules/<name>` with its work tree at `path`
/// (relative `gitdir:` file and `core.worktree`, as git writes them).
fn clone_submodule(
    sp: &Super,
    m: &Module,
    path: &str,
    url: &str,
    branch: Option<&str>,
    o: &UpdateOpts,
    display: &str,
) -> CliResult<()> {
    let abs = sp.work.join(path);
    let gitdir = sp.module_dir(&m.name);
    let failed = || {
        CliError::fatal(format!(
            "clone of '{url}' into submodule path '{}' failed",
            abs.display()
        ))
    };
    if gitdir.join("HEAD").is_file() {
        // Re-activate an existing module directory.
        fs::create_dir_all(&abs)?;
        link_worktree(&abs, &gitdir)?;
        return Ok(());
    }
    if abs.is_dir() && fs::read_dir(&abs)?.next().is_some() {
        return Err(CliError::fatal(format!(
            "destination path '{}' already exists and is not an empty directory.",
            abs.display()
        )));
    }
    let _ = fs::remove_dir(&abs);
    if let Some(parent) = gitdir.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut args: Vec<String> = vec!["--no-checkout".into()];
    if o.quiet {
        args.push("-q".into());
    }
    if let Some(b) = branch {
        args.push("-b".into());
        args.push(b.into());
    }
    if let Some(d) = &o.depth {
        args.push(format!("--depth={d}"));
    }
    if let Some(f) = &o.filter {
        args.push(format!("--filter={f}"));
    }
    match o.single_branch {
        Some(true) => args.push("--single-branch".into()),
        Some(false) => args.push("--no-single-branch".into()),
        None => {}
    }
    args.push(format!("--separate-git-dir={}", gitdir.display()));
    args.push(url.to_string());
    args.push(abs.to_string_lossy().into_owned());
    crate::cmd::clone::cmd_clone(&args).map_err(|e| {
        let msg = e.to_string();
        if !msg.trim().is_empty() {
            eprintln!("{}", msg.trim_end());
        }
        failed()
    })?;
    let _ = display;
    link_worktree(&abs, &gitdir)
}

/// Point `abs/.git` at `gitdir` and `gitdir`'s `core.worktree` back, both
/// relative.
fn link_worktree(abs: &Path, gitdir: &Path) -> CliResult<()> {
    let abs_c = canonical(abs);
    let gd_c = canonical(gitdir);
    fs::write(
        abs.join(".git"),
        format!("gitdir: {}\n", relative_path(&abs_c, &gd_c)),
    )?;
    edit_file(&gitdir.join("config"), |t| {
        cfg_set(t, "core", "worktree", &relative_path(&gd_c, &abs_c))
    })
}

/// `to` written relative to directory `from` with `/` separators.
fn relative_path(from: &Path, to: &Path) -> String {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = vec!["..".into(); a.len() - common];
    parts.extend(
        b[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    }
}

// ----------------------------------------------------------------- deinit --

fn sm_deinit(args: &[String]) -> CliResult<()> {
    let o = Opts::parse(args, &["-q", "--quiet", "-f", "--force", "--all"], &[])?;
    let all = o.on(&["--all"]);
    if all && !o.args.is_empty() {
        return Err(CliError::fatal("paths are ignored with --all"));
    }
    if !all && o.args.is_empty() {
        return Err(CliError::fatal(
            "Use '--all' if you really want to deinitialize all submodules",
        ));
    }
    let quiet = o.on(&["-q", "--quiet"]);
    let force = o.on(&["-f", "--force"]);
    let sp = Super::open()?;
    let list = module_list(&sp, &o.args)?;
    for e in &list {
        let Some(m) = &e.module else { continue };
        let display = sp.display(&e.path);
        let abs = sp.work.join(&e.path);
        if abs.join(".git").is_dir() {
            return Err(CliError::fatal(format!(
                "Submodule work tree '{display}' contains a .git directory. This will be replaced with a .git file by using absorbgitdirs."
            )));
        }
        if !force
            && let Some(sub) = open_populated(&sp, &e.path)
            && (sub.head_oid().ok() != Some(e.oid) || submodule_dirty(&sub)?)
        {
            eprintln!(
                "error: the following file has local modifications:\n    {display}\n(use --cached to keep the file, or -f to force removal)"
            );
            return Err(CliError::fatal(format!(
                "Submodule work tree '{display}' contains local modifications; use '-f' to discard them"
            )));
        }
        if abs.is_dir() {
            for entry in fs::read_dir(&abs)? {
                let p = entry?.path();
                if p.is_dir() && !p.is_symlink() {
                    fs::remove_dir_all(&p)?;
                } else {
                    fs::remove_file(&p)?;
                }
            }
            if !quiet {
                println!("Cleared directory '{display}'");
            }
            let mcfg = sp.module_dir(&m.name).join("config");
            match fs::read_to_string(&mcfg) {
                Ok(t) => {
                    if let Some(n) = cfg_unset(&t, "core", "worktree") {
                        fs::write(&mcfg, n)?;
                    }
                }
                Err(_) => {
                    eprintln!(
                        "error: could not lock config file .git/modules/{}/config: No such file or directory",
                        m.name
                    );
                    eprintln!(
                        "warning: Could not unset core.worktree setting in submodule '{display}'"
                    );
                }
            }
        }
        let cfg = sp.config();
        if let Some(url) = cfg.get(&format!("submodule.{}.url", m.name)) {
            let _ = url;
            edit_file(&sp.config_path(), |t| {
                cfg_remove_section(t, &format!("submodule.{}", m.name))
            })?;
            if !quiet {
                println!(
                    "Submodule '{}' ({}) unregistered for path '{display}'",
                    m.name, m.url
                );
            }
        }
    }
    Ok(())
}

/// Tracked or untracked changes in a submodule work tree.
fn submodule_dirty(sub: &Repository) -> CliResult<bool> {
    let Ok(work) = work_tree(sub) else {
        return Ok(false);
    };
    let index = read_index(sub)?;
    let tracked: HashSet<&str> = index.entries.iter().map(|e| e.path.as_str()).collect();
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let p = work.join(&e.path);
        if e.mode == 0o160000 {
            continue;
        }
        let Ok(bytes) = fs::read(&p) else {
            return Ok(true);
        };
        let digest = arks_hash::hash_object(arks_hash::ObjectFormat::Sha1, "blob", &bytes)
            .map_err(|err| CliError::Msg(err.to_string()))?;
        let oid = Oid::from_bytes(sub.oid_kind(), digest.as_bytes())
            .map_err(|err| CliError::Msg(err.to_string()))?;
        if oid != e.oid {
            return Ok(true);
        }
    }
    let mut stack = vec![work.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let p = entry?.path();
            if p.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            let rel = p
                .strip_prefix(&work)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if p.is_dir() {
                if !tracked.iter().any(|t| t.starts_with(&format!("{rel}/"))) {
                    return Ok(true);
                }
                stack.push(p);
            } else if !tracked.contains(rel.as_str()) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

// ------------------------------------------------------------------- sync --

fn sm_sync(args: &[String]) -> CliResult<()> {
    let o = Opts::parse(args, &["-q", "--quiet", "--recursive"], &[])?;
    let sp = Super::open()?;
    sync_in(
        &sp,
        &o.args,
        o.on(&["-q", "--quiet"]),
        o.on(&["--recursive"]),
        "",
    )
}

/// Copy `.gitmodules` URLs (resolved) into the repository config of
/// initialised submodules and into each checkout's default remote.
fn sync_in(
    sp: &Super,
    specs: &[String],
    quiet: bool,
    recursive: bool,
    outer: &str,
) -> CliResult<()> {
    for e in module_list(sp, specs)? {
        let Some(m) = &e.module else { continue };
        if m.url.is_empty() {
            continue;
        }
        let display = format!("{outer}{}", sp.display(&e.path));
        let url = sp.resolve_url(&m.url);
        let cfg = sp.config();
        if cfg.get(&format!("submodule.{}.url", m.name)).is_none() {
            continue;
        }
        if !quiet {
            println!("Synchronizing submodule url for '{display}'");
        }
        edit_file(&sp.config_path(), |t| {
            cfg_set(t, &format!("submodule.{}", m.name), "url", &url)
        })?;
        if let Some(sub) = open_populated(sp, &e.path) {
            let scfg = arks_config::ConfigSet::load(Some(sub.common_dir()));
            let remote = match sub.head_target() {
                Ok(RefTarget::Symbolic(r)) => r
                    .strip_prefix("refs/heads/")
                    .and_then(|b| scfg.get(&format!("branch.{b}.remote"))),
                _ => None,
            }
            .unwrap_or_else(|| "origin".into());
            edit_file(&sub.common_dir().join("config"), |t| {
                cfg_set(t, &format!("remote.{remote}"), "url", &url)
            })?;
            if recursive {
                let abs = sp.work.join(&e.path);
                let nested_outer = format!("{display}/");
                in_dir(&abs, || {
                    let inner = Super::open()?;
                    sync_in(&inner, &[], quiet, true, &nested_outer)
                })?;
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------- set-branch / url --

fn module_by_path(sp: &Super, path: &str) -> CliResult<Module> {
    let full = normalize(&format!("{}{path}", sp.prefix));
    sp.gitmodules()
        .into_iter()
        .find(|m| m.path == full)
        .ok_or_else(|| {
            CliError::fatal(format!(
                "no submodule mapping found in .gitmodules for path '{path}'"
            ))
        })
}

fn sm_set_branch(args: &[String]) -> CliResult<()> {
    let o = Opts::parse(
        args,
        &["-q", "--quiet", "-d", "--default"],
        &["-b", "--branch"],
    )?;
    let helper_usage = || {
        CliError::Exit(
            129,
            "usage: git submodule set-branch [-q|--quiet] (-d|--default) <path>\n   or: git submodule set-branch [-q|--quiet] (-b|--branch) <branch> <path>".into(),
        )
    };
    let [path] = o.args.as_slice() else {
        return Err(helper_usage());
    };
    let default = o.on(&["-d", "--default"]);
    let branch = o.value(&["-b", "--branch"]);
    if default && branch.is_some() {
        return Err(CliError::fatal(
            "--branch and --default are mutually exclusive",
        ));
    }
    if !default && branch.is_none() {
        return Err(CliError::fatal("--branch or --default required"));
    }
    let sp = Super::open()?;
    let m = module_by_path(&sp, path)?;
    let gm = sp.gitmodules_path();
    let sec = format!("submodule.{}", m.name);
    let text = fs::read_to_string(&gm).unwrap_or_default();
    match branch {
        Some(b) => edit_file(&gm, |t| cfg_set(t, &sec, "branch", &b)),
        None if cfg_unset(&text, &sec, "branch").is_none() => Err(CliError::silent(1)),
        None => edit_file(&gm, |t| {
            cfg_unset(t, &sec, "branch").unwrap_or_else(|| t.to_string())
        }),
    }
}

fn sm_set_url(args: &[String]) -> CliResult<()> {
    let o = Opts::parse(args, &["-q", "--quiet"], &[])?;
    let [path, url] = o.args.as_slice() else {
        return Err(CliError::Exit(
            129,
            "usage: git submodule set-url [--quiet] <path> <newurl>".into(),
        ));
    };
    let sp = Super::open()?;
    let m = module_by_path(&sp, path)?;
    edit_file(&sp.gitmodules_path(), |t| {
        cfg_set(t, &format!("submodule.{}", m.name), "url", url)
    })?;
    sync_in(
        &sp,
        std::slice::from_ref(path),
        o.on(&["-q", "--quiet"]),
        false,
        "",
    )
}

// ---------------------------------------------------------------- summary --

fn sm_summary(args: &[String]) -> CliResult<()> {
    let o = Opts::parse(
        args,
        &["-q", "--quiet", "--cached", "--files", "--for-status"],
        &["-n", "--summary-limit", "--summary-limit="],
    )?;
    let cached = o.on(&["--cached"]);
    let files = o.on(&["--files"]);
    if cached && files {
        return Err(CliError::fatal(
            "options '--cached' and '--files' cannot be used together",
        ));
    }
    let limit: Option<usize> = match o.value(&["-n", "--summary-limit"]) {
        Some(v) => {
            let n: i64 = v.parse().map_err(|_| usage())?;
            if n == 0 {
                return Ok(());
            }
            (n > 0).then_some(n as usize)
        }
        None => None,
    };
    let sp = Super::open()?;
    // An optional leading commit; the rest are pathspecs.
    let mut specs = o.args.clone();
    let commit = match specs.first() {
        Some(c) if !files && crate::cmd::shared::resolve_rev(&sp.repo, c).is_ok() => {
            let c = specs.remove(0);
            Some(crate::cmd::shared::resolve_rev(&sp.repo, &c)?)
        }
        _ if files => None,
        _ => sp.repo.head_oid().ok(),
    };
    let base: HashMap<String, Oid> = match commit {
        Some(c) => gitlinks_of_commit(&sp.repo, &c)?,
        None => HashMap::new(),
    };
    let index = read_index(&sp.repo)?;
    let mut paths: Vec<String> = base.keys().cloned().collect();
    for e in index.entries.iter().filter(|e| e.mode == 0o160000) {
        if !paths.contains(&e.path) {
            paths.push(e.path.clone());
        }
    }
    paths.sort();
    let full: Vec<String> = specs
        .iter()
        .map(|s| normalize(&format!("{}{s}", sp.prefix)))
        .collect();
    for path in paths {
        if !full.is_empty()
            && !full
                .iter()
                .any(|f| f.is_empty() || path == *f || path.starts_with(&format!("{f}/")))
        {
            continue;
        }
        let idx = index
            .entries
            .iter()
            .find(|e| e.path == path && e.stage() == 0 && e.mode == 0o160000)
            .map(|e| e.oid);
        let sub = open_populated(&sp, &path);
        let head = sub.as_ref().and_then(|s| s.head_oid().ok());
        let (src, dst) = if files {
            (idx, head.or(idx))
        } else if cached {
            (base.get(&path).copied(), idx)
        } else {
            (base.get(&path).copied(), head.or(idx))
        };
        if src == dst {
            continue;
        }
        let Some(sub) = sub else { continue };
        print_summary_block(&sp, &sub, &path, src, dst, limit)?;
    }
    Ok(())
}

fn gitlinks_of_commit(repo: &Repository, commit: &Oid) -> CliResult<HashMap<String, Oid>> {
    let files = crate::cmd::switch_tree::commit_files(repo, commit)?;
    Ok(files
        .into_iter()
        .filter(|(_, f)| f.mode == 0o160000)
        .map(|(p, f)| (p, f.oid))
        .collect())
}

fn print_summary_block(
    sp: &Super,
    sub: &Repository,
    path: &str,
    src: Option<Oid>,
    dst: Option<Oid>,
    limit: Option<usize>,
) -> CliResult<()> {
    let zero = "0000000".to_string();
    let short = |o: &Option<Oid>| {
        o.as_ref()
            .map(|o| crate::cmd::shared::abbrev_hex(sub, o, 7))
            .unwrap_or_else(|| zero.clone())
    };
    let display = sp.display(path);
    let subject = |o: &Oid| -> (i64, String) {
        sub.get_object(o)
            .ok()
            .and_then(|x| Commit::decode(&x.payload).ok())
            .map(|c| {
                (
                    c.committer.timestamp,
                    c.message.lines().next().unwrap_or("").to_string(),
                )
            })
            .unwrap_or_default()
    };
    let ancestors = |tip: &Oid| -> HashSet<Oid> {
        let mut seen = HashSet::new();
        let mut stack = vec![*tip];
        while let Some(c) = stack.pop() {
            if seen.insert(c) {
                stack.extend(parents(sub, &c));
            }
        }
        seen
    };
    let mut lines: Vec<(i64, usize, char, String)> = Vec::new();
    let total;
    match (src, dst) {
        (Some(s), Some(d)) => {
            if sub.get_object(&s).is_err() || sub.get_object(&d).is_err() {
                return Ok(());
            }
            let left = ancestors(&s);
            let right = ancestors(&d);
            let mut n = 0;
            for (side, set, other, mark) in [(0, &right, &left, '>'), (1, &left, &right, '<')] {
                for c in set.difference(other) {
                    let (t, s) = subject(c);
                    lines.push((t, side, mark, s));
                    n += 1;
                }
            }
            total = n;
            lines.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        }
        (None, Some(d)) | (Some(d), None) => {
            if sub.get_object(&d).is_err() {
                return Ok(());
            }
            total = ancestors(&d).len();
            let mark = if src.is_none() { '>' } else { '<' };
            let (t, s) = subject(&d);
            lines.push((t, 0, mark, s));
        }
        (None, None) => return Ok(()),
    }
    println!("* {display} {}...{} ({total}):", short(&src), short(&dst));
    for (_, _, mark, s) in lines.iter().take(limit.unwrap_or(usize::MAX)) {
        println!("  {mark} {s}");
    }
    println!();
    Ok(())
}

// ---------------------------------------------------------------- foreach --

fn sm_foreach(args: &[String]) -> CliResult<()> {
    let mut quiet = false;
    let mut recursive = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-q" | "--quiet" => quiet = true,
            "--recursive" => recursive = true,
            "--" => {
                i += 1;
                break;
            }
            a if a.starts_with('-') => return Err(usage()),
            _ => break,
        }
        i += 1;
    }
    let sp = Super::open()?;
    foreach_in(&sp, &args[i..], quiet, recursive, "")
}

fn foreach_in(
    sp: &Super,
    cmd: &[String],
    quiet: bool,
    recursive: bool,
    outer: &str,
) -> CliResult<()> {
    use std::io::Write;
    let cfg = sp.config();
    let toplevel = sp.work.to_string_lossy().into_owned();
    for e in module_list(sp, &[])? {
        let Some(m) = &e.module else { continue };
        if !is_active(&cfg, &m.name) || open_populated(sp, &e.path).is_none() {
            continue;
        }
        let display = format!("{outer}{}", sp.display(&e.path));
        if !quiet {
            println!("Entering '{display}'");
        }
        std::io::stdout().flush()?;
        let abs = sp.work.join(&e.path);
        if !cmd.is_empty() {
            let mut c = std::process::Command::new("sh");
            if cmd.len() == 1 {
                c.arg("-c").arg(&cmd[0]);
            } else {
                c.arg("-c").arg("\"$@\"").arg("sh").args(cmd);
            }
            let status = c
                .current_dir(&abs)
                .env("name", &m.name)
                .env("sm_path", &e.path)
                .env("path", &e.path)
                .env("displaypath", &display)
                .env("sha1", e.oid.to_hex())
                .env("toplevel", &toplevel)
                .status()?;
            if !status.success() {
                return Err(CliError::fatal(format!(
                    "run_command returned non-zero status for {display}\n."
                )));
            }
        }
        if recursive {
            let nested_outer = format!("{display}/");
            in_dir(&abs, || {
                let inner = Super::open()?;
                foreach_in(&inner, cmd, quiet, true, &nested_outer)
            })?;
        }
    }
    Ok(())
}

// --------------------------------------------------------- absorbgitdirs --

fn sm_absorbgitdirs(args: &[String]) -> CliResult<()> {
    let mut specs: Vec<String> = Vec::new();
    let mut opts_done = false;
    for a in args {
        if !opts_done && a == "--" {
            opts_done = true;
        } else if !opts_done && a.starts_with("--") {
            return Err(CliError::Exit(
                129,
                format!(
                    "error: unknown option `{}'\nusage: git submodule absorbgitdirs [<options>] [<path>...]",
                    &a[2..]
                ),
            ));
        } else if !opts_done && a.starts_with('-') && a.len() > 1 {
            return Err(CliError::Exit(
                129,
                format!(
                    "error: unknown switch `{}'\nusage: git submodule absorbgitdirs [<options>] [<path>...]",
                    &a[1..2]
                ),
            ));
        } else {
            specs.push(a.clone());
        }
    }
    let sp = Super::open()?;
    for e in module_list(&sp, &specs)? {
        let Some(m) = &e.module else { continue };
        let abs = sp.work.join(&e.path);
        let inner = abs.join(".git");
        if !inner.is_dir() {
            continue;
        }
        let dest = sp.module_dir(&m.name);
        if dest.exists() {
            return Err(CliError::fatal(format!(
                "could not migrate git directory from '{}' to '{}'",
                inner.display(),
                dest.display()
            )));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        eprintln!(
            "Migrating git directory of '{}' from '{}' to '{}'",
            sp.display(&e.path),
            canonical(&inner).display(),
            dest.display()
        );
        fs::rename(&inner, &dest)?;
        link_worktree(&abs, &dest)?;
    }
    Ok(())
}

// -------------------------------------------------------------------- add --

fn sm_add(args: &[String]) -> CliResult<()> {
    let o = Opts::parse(
        args,
        &[
            "-q",
            "--quiet",
            "-f",
            "--force",
            "--progress",
            "--dissociate",
        ],
        &[
            "-b",
            "--branch",
            "--reference",
            "--reference=",
            "--name",
            "--depth",
            "--depth=",
        ],
    )?;
    let quiet = o.on(&["-q", "--quiet"]);
    let (repo_arg, path_arg) = match o.args.as_slice() {
        [r] => (r.clone(), None),
        [r, p] => (r.clone(), Some(p.clone())),
        _ => return Err(usage()),
    };
    if let Some(d) = o.value(&["--depth"]) {
        if !d.chars().all(|c| c.is_ascii_digit()) || d.is_empty() {
            return Err(CliError::fatal(format!("'{d}' is not a valid depth")));
        }
    }
    let sp = Super::open()?;
    let real = if is_relative_url(&repo_arg) {
        sp.resolve_url(&repo_arg)
    } else if arks_remote::is_local_remote(&repo_arg) && !repo_arg.contains("://") {
        let p = Path::new(&repo_arg);
        if p.is_absolute() {
            repo_arg.clone()
        } else {
            std::env::current_dir()?
                .join(p)
                .to_string_lossy()
                .into_owned()
        }
    } else {
        repo_arg.clone()
    };
    let path = match path_arg {
        Some(p) => normalize(&format!("{}{p}", sp.prefix)),
        None => {
            let base = repo_arg
                .trim_end_matches('/')
                .rsplit(['/', ':'])
                .next()
                .unwrap_or("")
                .trim_end_matches(".git")
                .to_string();
            normalize(&format!("{}{base}", sp.prefix))
        }
    };
    if path.is_empty() {
        return Err(usage());
    }
    let index = read_index(&sp.repo)?;
    if index.entries.iter().any(|e| e.path == path) {
        return Err(CliError::fatal(format!(
            "'{}' already exists in the index",
            sp.display(&path)
        )));
    }
    let name = o.value(&["--name"]).unwrap_or_else(|| path.clone());
    let abs = sp.work.join(&path);
    let module = Module {
        name: name.clone(),
        path: path.clone(),
        url: repo_arg.clone(),
        branch: o.value(&["-b", "--branch"]),
        update: None,
    };
    let existing = abs.join(".git").exists();
    if existing {
        if !quiet {
            println!(
                "Adding existing repo at '{}' to the index",
                sp.display(&path)
            );
        }
    } else {
        let src = Path::new(&real);
        if arks_remote::is_local_remote(&real)
            && !real.contains("://")
            && !src.join("HEAD").is_file()
            && !src.join(".git").exists()
        {
            eprintln!("fatal: repository '{real}' does not exist");
            return Err(CliError::fatal(format!(
                "clone of '{real}' into submodule path '{}' failed",
                abs.display()
            )));
        }
        let uo = UpdateOpts {
            quiet,
            init: false,
            force: false,
            remote: false,
            no_fetch: false,
            mode: None,
            recursive: false,
            depth: o.value(&["--depth"]),
            filter: None,
            single_branch: None,
        };
        clone_submodule(&sp, &module, &path, &real, None, &uo, &path)?;
        // `add` leaves the submodule on a branch with the work tree checked
        // out; `-b <branch>` makes a local branch tracking origin/<branch>.
        let mut sub = Repository::discover(&abs)?;
        if let Some(b) = &module.branch {
            let tip = crate::cmd::shared::resolve_rev(&sub, &format!("refs/remotes/origin/{b}"))
                .map_err(|_| {
                    CliError::fatal(format!(
                        "'origin/{b}' is not a commit and a branch '{b}' cannot be created from it"
                    ))
                })?;
            sub.refs_mut()
                .update(&format!("refs/heads/{b}"), &tip, None)?;
            fs::write(sub.git_dir().join("HEAD"), format!("ref: refs/heads/{b}\n"))?;
            let sec = format!("branch.{b}");
            edit_file(&sub.common_dir().join("config"), |t| {
                let t = cfg_set(t, &sec, "remote", "origin");
                cfg_set(&t, &sec, "merge", &format!("refs/heads/{b}"))
            })?;
        }
        let sub = Repository::discover(&abs)?;
        let head = sub.head_oid()?;
        arks_remote::checkout_commit(&sub, &abs, &head)
            .map_err(|e| CliError::Msg(e.to_string()))?;
    }
    let sub = Repository::discover(&abs)?;
    let head = sub.head_oid()?;

    // .gitmodules: path, url (as given), branch.
    let sec = format!("submodule.{name}");
    edit_file(&sp.gitmodules_path(), |t| {
        let t = cfg_set(t, &sec, "path", &path);
        let t = cfg_set(&t, &sec, "url", &repo_arg);
        match &module.branch {
            Some(b) => cfg_set(&t, &sec, "branch", b),
            None => t,
        }
    })?;
    // Repository config: url (resolved) and active.
    edit_file(&sp.config_path(), |t| {
        let t = cfg_set(t, &sec, "url", &real);
        cfg_set(&t, &sec, "active", "true")
    })?;

    // Stage the gitlink and .gitmodules.
    let mut repo = open_repo()?;
    let mut index = read_index(&repo)?;
    index
        .entries
        .retain(|e| e.path != path && e.path != ".gitmodules");
    let gm_bytes = fs::read(sp.gitmodules_path())?;
    let format = repo.object_format();
    let gm_oid = repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Blob, gm_bytes.clone()))?;
    let gm_meta = fs::metadata(sp.gitmodules_path())?;
    index
        .entries
        .push(index_entry(&path, 0o160000, head, 0, None));
    index.entries.push(index_entry(
        ".gitmodules",
        0o100644,
        gm_oid,
        gm_bytes.len(),
        Some(&gm_meta),
    ));
    sort_entries(&mut index.entries);
    write_index(&repo, &index)?;
    Ok(())
}

fn index_entry(
    path: &str,
    mode: u32,
    oid: Oid,
    size: usize,
    meta: Option<&fs::Metadata>,
) -> IndexEntry {
    let (mtime_sec, mtime_nsec, ctime_sec, ctime_nsec, dev, ino, uid, gid) = stat_fields(meta);
    IndexEntry {
        ctime_sec,
        ctime_nsec,
        mtime_sec,
        mtime_nsec,
        dev,
        ino,
        mode,
        uid,
        gid,
        size: size.min(u32::MAX as usize) as u32,
        oid,
        flags: path.len().min(0x0fff) as u16,
        flags_extended: None,
        path: path.to_string(),
    }
}

#[cfg(unix)]
fn stat_fields(meta: Option<&fs::Metadata>) -> (u32, u32, u32, u32, u32, u32, u32, u32) {
    use std::os::unix::fs::MetadataExt;
    match meta {
        Some(m) => (
            m.mtime() as u32,
            m.mtime_nsec() as u32,
            m.ctime() as u32,
            m.ctime_nsec() as u32,
            m.dev() as u32,
            m.ino() as u32,
            m.uid(),
            m.gid(),
        ),
        None => (0, 0, 0, 0, 0, 0, 0, 0),
    }
}

#[cfg(not(unix))]
fn stat_fields(_meta: Option<&fs::Metadata>) -> (u32, u32, u32, u32, u32, u32, u32, u32) {
    (0, 0, 0, 0, 0, 0, 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_urls_resolve_against_base() {
        assert_eq!(resolve_relative_url("/a/b/super", "../lib"), "/a/b/lib");
        assert_eq!(
            resolve_relative_url("/a/b/super/", "./lib"),
            "/a/b/super/lib"
        );
        assert_eq!(resolve_relative_url("/a/b/super", "../../x"), "/a/x");
    }

    #[test]
    fn config_set_and_unset_round_trip() {
        let t = cfg_set("", "submodule.lib", "path", "lib");
        let t = cfg_set(&t, "submodule.lib", "url", "../lib");
        assert_eq!(t, "[submodule \"lib\"]\n\tpath = lib\n\turl = ../lib\n");
        let t = cfg_set(&t, "submodule.lib", "url", "../other");
        assert!(t.contains("url = ../other") && !t.contains("../lib\n"));
        let t = cfg_unset(&t, "submodule.lib", "url").expect("was set");
        assert_eq!(t, "[submodule \"lib\"]\n\tpath = lib\n");
        assert!(cfg_unset(&t, "submodule.lib", "branch").is_none());
        assert_eq!(cfg_remove_section(&t, "submodule.lib"), "");
    }

    #[test]
    fn relative_paths_between_dirs() {
        assert_eq!(
            relative_path(Path::new("/r/.git/modules/lib"), Path::new("/r/lib")),
            "../../../lib"
        );
        assert_eq!(
            relative_path(Path::new("/r/lib"), Path::new("/r/.git/modules/lib")),
            "../.git/modules/lib"
        );
    }
}
