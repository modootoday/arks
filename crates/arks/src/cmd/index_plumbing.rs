//! Index plumbing: `update-index`, `read-tree`, `checkout-index`.
//!
//! Options are processed in argv order like stock git (flags apply to the
//! paths that follow them). Index extensions that are pure caches (split
//! index, untracked cache, fsmonitor, resolve-undo) are accepted and leave
//! the entry list unchanged, so `ls-files -s` / `status` read the same.
//! Cite: <https://git-scm.com/docs/git-update-index>,
//! <https://git-scm.com/docs/git-read-tree>,
//! <https://git-scm.com/docs/git-checkout-index>.

use crate::cmd::opt_parse::{self, Item, OptDef, Takes::*, o};
use crate::cmd::shared::{index_stat_matches, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use arks_index::{CE_EXTENDED, CE_VALID, Index, IndexEntry, sort_entries};
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const CE_SKIP_WORKTREE: u16 = 0x4000;
const CE_INTENT_TO_ADD: u16 = 0x2000;

fn stage_flags(path: &str, stage: u16) -> u16 {
    (path.len().min(0x0fff) as u16) | (stage << 12)
}

fn zero_entry(path: &str, oid: Oid, mode: u32, stage: u16) -> IndexEntry {
    IndexEntry {
        ctime_sec: 0,
        ctime_nsec: 0,
        mtime_sec: 0,
        mtime_nsec: 0,
        dev: 0,
        ino: 0,
        mode,
        uid: 0,
        gid: 0,
        size: 0,
        oid,
        flags: stage_flags(path, stage),
        flags_extended: None,
        path: path.to_string(),
    }
}

/// Path given on the command line → index path (relative to the work tree).
fn index_path(work: &Path, arg: &str) -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| work.to_path_buf());
    let abs = if Path::new(arg).is_absolute() {
        PathBuf::from(arg)
    } else {
        cwd.join(arg)
    };
    let work_c = work.canonicalize().unwrap_or_else(|_| work.to_path_buf());
    let mut parts: Vec<String> = Vec::new();
    // Normalize lexically (the path may not exist).
    let base = abs
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .map(|p| p.join(abs.file_name().unwrap_or_default()))
        .unwrap_or(abs);
    let rel = base.strip_prefix(&work_c).unwrap_or(&base);
    for c in rel.components() {
        match c {
            std::path::Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
            std::path::Component::ParentDir => {
                parts.pop();
            }
            _ => {}
        }
    }
    parts.join("/")
}

fn hash_file(
    repo: &mut Repository,
    abs: &Path,
    meta: &fs::Metadata,
    write: bool,
) -> CliResult<Oid> {
    let bytes = if meta.file_type().is_symlink() {
        fs::read_link(abs)?
            .to_string_lossy()
            .into_owned()
            .into_bytes()
    } else {
        fs::read(abs)?
    };
    let obj = RawObject::new(ObjectKind::Blob, bytes);
    let format = repo.object_format();
    if write {
        Ok(repo.store_mut().put(format, &obj)?)
    } else {
        Ok(obj.oid(format)?)
    }
}

fn entry_from_stat(path: &str, oid: Oid, meta: &fs::Metadata) -> IndexEntry {
    let mut e = crate::cmd::working_tree::stat_entry(path, oid, meta);
    if meta.file_type().is_symlink() {
        e.mode = 0o120000;
    }
    e
}

fn set_ext_flag(e: &mut IndexEntry, bit: u16, on: bool) {
    let mut ext = e.flags_extended.unwrap_or(0);
    if on {
        ext |= bit;
    } else {
        ext &= !bit;
    }
    if ext == 0 {
        e.flags &= !CE_EXTENDED;
        e.flags_extended = None;
    } else {
        e.flags |= CE_EXTENDED;
        e.flags_extended = Some(ext);
    }
}

fn has_ext(e: &IndexEntry, bit: u16) -> bool {
    e.flags_extended.unwrap_or(0) & bit != 0
}

fn fatal_path(msg: &str, path: &str) -> CliError {
    CliError::Exit(
        128,
        format!("error: {msg}\nfatal: Unable to process path {path}"),
    )
}

const UPDATE_INDEX_SPEC: &[OptDef] = &[
    o("-q", 'q', No, false),
    o("ignore-submodules", ' ', No, true),
    o("add", ' ', No, true),
    o("replace", ' ', No, true),
    o("remove", ' ', No, true),
    o("unmerged", ' ', No, true),
    o("refresh", ' ', No, false),
    o("really-refresh", ' ', No, false),
    o("cacheinfo", ' ', Req, false),
    o("chmod", ' ', Req, false),
    o("assume-unchanged", ' ', No, true),
    o("skip-worktree", ' ', No, true),
    o("ignore-skip-worktree-entries", ' ', No, true),
    o("info-only", ' ', No, true),
    o("force-remove", ' ', No, true),
    o("-z", 'z', No, false),
    o("stdin", ' ', No, false),
    o("index-info", ' ', No, false),
    o("unresolve", ' ', No, false),
    o("again", 'g', No, false),
    o("ignore-missing", ' ', No, true),
    o("verbose", ' ', No, true),
    o("clear-resolve-undo", ' ', No, false),
    o("index-version", ' ', Req, true),
    o("show-index-version", ' ', No, true),
    o("split-index", ' ', No, true),
    o("untracked-cache", ' ', No, true),
    o("test-untracked-cache", ' ', No, true),
    o("force-untracked-cache", ' ', No, true),
    o("force-write-index", ' ', No, true),
    o("fsmonitor", ' ', No, true),
    o("fsmonitor-valid", ' ', No, true),
];

#[derive(Default)]
struct UiState {
    add: bool,
    remove: bool,
    replace: bool,
    force_remove: bool,
    info_only: bool,
    unmerged: bool,
    ignore_missing: bool,
    ignore_submodules: bool,
    verbose: bool,
    quiet: bool,
    chmod: Option<bool>,
    mark_valid: Option<bool>,
    mark_skip: Option<bool>,
    mark_fsmonitor: Option<bool>,
    unresolve: bool,
    changed: bool,
    needs_update: bool,
}

struct Ui<'a> {
    repo: &'a mut Repository,
    work: PathBuf,
    index: Index,
    st: UiState,
    out: String,
}

impl Ui<'_> {
    fn report(&mut self, line: String) {
        if self.st.verbose {
            self.out.push_str(&line);
            self.out.push('\n');
        }
    }

    fn pos_of(&self, path: &str, stage: u16) -> Option<usize> {
        self.index
            .entries
            .iter()
            .position(|e| e.path == path && e.stage() == stage)
    }

    fn remove_path(&mut self, path: &str) {
        let before = self.index.entries.len();
        self.index.entries.retain(|e| e.path != path);
        if self.index.entries.len() != before {
            self.st.changed = true;
        }
    }

    fn put(&mut self, e: IndexEntry) {
        let path = e.path.clone();
        self.index.entries.retain(|x| x.path != path);
        // A file replaces any directory entries below it and vice versa.
        let dir = format!("{path}/");
        self.index.entries.retain(|x| !x.path.starts_with(&dir));
        self.index
            .entries
            .retain(|x| !path.starts_with(&format!("{}/", x.path)));
        self.index.entries.push(e);
        sort_entries(&mut self.index.entries);
        self.st.changed = true;
    }

    fn mark(&mut self, path: &str) -> CliResult<bool> {
        let (bit_valid, bit_skip) = (self.st.mark_valid, self.st.mark_skip);
        if bit_valid.is_none() && bit_skip.is_none() && self.st.mark_fsmonitor.is_none() {
            return Ok(false);
        }
        let Some(i) = self.pos_of(path, 0) else {
            return Err(CliError::fatal(format!("Unable to mark file {path}")));
        };
        let e = &mut self.index.entries[i];
        if let Some(on) = bit_valid {
            if on {
                e.flags |= CE_VALID;
            } else {
                e.flags &= !CE_VALID;
            }
        }
        if let Some(on) = bit_skip {
            set_ext_flag(e, CE_SKIP_WORKTREE, on);
        }
        self.st.changed = true;
        Ok(true)
    }

    fn update_one(&mut self, path: &str) -> CliResult<()> {
        if path.is_empty() || path.split('/').any(|c| c == ".git") {
            return Err(CliError::fatal(format!("Ignoring path {path}")));
        }
        if self.mark(path)? {
            return Ok(());
        }
        if self.st.unresolve {
            return self.unresolve(path);
        }
        if self.st.force_remove {
            self.remove_path(path);
            self.report(format!("remove '{path}'"));
            return Ok(());
        }
        let abs = self.work.join(path);
        match fs::symlink_metadata(&abs) {
            Err(_) => {
                if !self.st.remove {
                    return Err(fatal_path(
                        &format!("{path}: does not exist and --remove not passed"),
                        path,
                    ));
                }
                self.remove_path(path);
            }
            Ok(meta) if meta.is_dir() => {
                if abs.join(".git").exists() {
                    // Gitlink: record the submodule HEAD.
                    let sub = arks_repo::Repository::discover(&abs)?;
                    let oid = sub.head_oid()?;
                    let mut e = zero_entry(path, oid, 0o160000, 0);
                    e.flags = stage_flags(path, 0);
                    self.put(e);
                } else {
                    let dir = format!("{path}/");
                    if self.st.remove
                        && self.index.entries.iter().any(|e| e.path.starts_with(&dir))
                        && !self.st.replace
                    {
                        // A directory where index files were: nothing to update.
                    }
                    return Err(fatal_path(
                        &format!("{path}: is a directory - add individual files instead"),
                        path,
                    ));
                }
            }
            Ok(meta) => {
                let existing = self.index.entries.iter().any(|e| e.path == path);
                if !existing && !self.st.add {
                    return Err(fatal_path(
                        &format!("{path}: cannot add to the index - missing --add option?"),
                        path,
                    ));
                }
                let conflicts_dir = !self.st.replace
                    && (self
                        .index
                        .entries
                        .iter()
                        .any(|e| e.path.starts_with(&format!("{path}/")))
                        || self
                            .index
                            .entries
                            .iter()
                            .any(|e| path.starts_with(&format!("{}/", e.path))));
                if conflicts_dir {
                    return Err(fatal_path(
                        &format!("'{path}' appears as both a file and as a directory"),
                        path,
                    ));
                }
                let oid = hash_file(self.repo, &abs, &meta, !self.st.info_only)?;
                let mut e = entry_from_stat(path, oid, &meta);
                if let Some(old) = self
                    .index
                    .entries
                    .iter()
                    .find(|x| x.path == path && x.stage() == 0)
                {
                    if !filemode_trusted(self.repo) && e.mode != 0o120000 {
                        e.mode = old.mode;
                    }
                }
                self.put(e);
            }
        }
        self.report(format!("add '{path}'"));
        if let Some(x) = self.st.chmod {
            self.chmod(path, x)?;
        }
        Ok(())
    }

    fn chmod(&mut self, path: &str, exec: bool) -> CliResult<()> {
        let Some(i) = self.pos_of(path, 0) else {
            return Err(CliError::fatal(format!(
                "git update-index: cannot chmod {}x '{path}'",
                if exec { '+' } else { '-' }
            )));
        };
        let e = &mut self.index.entries[i];
        if e.mode & 0o170000 != 0o100000 {
            return Err(CliError::fatal(format!(
                "git update-index: cannot chmod {}x '{path}'",
                if exec { '+' } else { '-' }
            )));
        }
        e.mode = if exec { 0o100755 } else { 0o100644 };
        self.st.changed = true;
        self.report(format!("chmod {}x '{path}'", if exec { '+' } else { '-' }));
        Ok(())
    }

    fn unresolve(&mut self, path: &str) -> CliResult<()> {
        let merge_head = self.repo.git_dir().join("MERGE_HEAD");
        let Ok(text) = fs::read_to_string(&merge_head) else {
            return Ok(());
        };
        let Some(theirs) = text
            .lines()
            .next()
            .and_then(|l| Oid::from_hex(l.trim()).ok())
        else {
            return Ok(());
        };
        let head = self.repo.head_oid()?;
        let ours_t = crate::cmd::switch_tree::commit_files(self.repo, &head)?;
        let theirs_t = crate::cmd::switch_tree::commit_files(self.repo, &theirs)?;
        let (o, t) = (ours_t.get(path), theirs_t.get(path));
        if o.is_none() && t.is_none() {
            return Ok(());
        }
        self.index.entries.retain(|e| e.path != path);
        if let Some(f) = o {
            self.index.entries.push(zero_entry(path, f.oid, f.mode, 2));
        }
        if let Some(f) = t {
            self.index.entries.push(zero_entry(path, f.oid, f.mode, 3));
        }
        sort_entries(&mut self.index.entries);
        self.st.changed = true;
        Ok(())
    }

    fn cacheinfo(&mut self, mode: &str, oid: &str, path: &str) -> CliResult<()> {
        let bad = || CliError::fatal(format!("git update-index: --cacheinfo cannot add {path}"));
        let mode_u = u32::from_str_radix(mode, 8).map_err(|_| bad())?;
        let oid = Oid::from_hex(oid).map_err(|_| bad())?;
        let existing = self.index.entries.iter().any(|e| e.path == path);
        let df = !self.st.replace
            && self.index.entries.iter().any(|e| {
                e.path.starts_with(&format!("{path}/")) || path.starts_with(&format!("{}/", e.path))
            });
        if df || (!existing && !self.st.add) {
            let mut m = String::new();
            if df {
                m.push_str(&format!(
                    "error: '{path}' appears as both a file and as a directory\n"
                ));
            }
            m.push_str(&format!(
                "error: {path}: cannot add to the index - missing --add option?\nfatal: git update-index: --cacheinfo cannot add {path}"
            ));
            return Err(CliError::Exit(128, m));
        }
        self.put(zero_entry(path, oid, mode_u, 0));
        self.report(format!("add '{path}'"));
        Ok(())
    }

    fn refresh(&mut self, really: bool) -> CliResult<()> {
        let format = self.repo.object_format();
        let mut msgs = Vec::new();
        let mut seen_unmerged: BTreeSet<String> = BTreeSet::new();
        for e in self.index.entries.iter_mut() {
            if e.stage() != 0 {
                if !self.st.unmerged && seen_unmerged.insert(e.path.clone()) {
                    msgs.push(format!("{}: needs merge", e.path));
                }
                continue;
            }
            if has_ext(e, CE_SKIP_WORKTREE) || has_ext(e, CE_INTENT_TO_ADD) {
                continue;
            }
            if !really && e.flags & CE_VALID != 0 {
                continue;
            }
            if e.mode == 0o160000 {
                continue;
            }
            let abs = self.work.join(&e.path);
            let Ok(meta) = fs::symlink_metadata(&abs) else {
                if !self.st.ignore_missing {
                    msgs.push(format!("{}: needs update", e.path));
                }
                continue;
            };
            if meta.is_dir() {
                msgs.push(format!("{}: needs update", e.path));
                continue;
            }
            let mode = if meta.file_type().is_symlink() {
                0o120000
            } else {
                crate::cmd::shared::file_mode(&meta)
            };
            if index_stat_matches(e, &meta) && mode == e.mode {
                continue;
            }
            let bytes = if meta.file_type().is_symlink() {
                fs::read_link(&abs)?
                    .to_string_lossy()
                    .into_owned()
                    .into_bytes()
            } else {
                fs::read(&abs)?
            };
            let oid = RawObject::new(ObjectKind::Blob, bytes).oid(format)?;
            if oid != e.oid || mode != e.mode {
                msgs.push(format!("{}: needs update", e.path));
                continue;
            }
            let fresh = entry_from_stat(&e.path, oid, &meta);
            let keep_flags = e.flags;
            let keep_ext = e.flags_extended;
            *e = IndexEntry {
                flags: keep_flags,
                flags_extended: keep_ext,
                ..fresh
            };
            if really {
                e.flags &= !CE_VALID;
            }
            self.st.changed = true;
        }
        if !msgs.is_empty() {
            self.st.needs_update = true;
            if !self.st.quiet {
                for m in msgs {
                    self.out.push_str(&m);
                    self.out.push('\n');
                }
            }
        }
        Ok(())
    }

    fn again(&mut self, specs: &[String]) -> CliResult<()> {
        let head = match self.repo.head_oid() {
            Ok(h) => crate::cmd::switch_tree::commit_files(self.repo, &h)?,
            Err(_) => HashMap::new(),
        };
        let paths: Vec<String> = self
            .index
            .entries
            .iter()
            .filter(|e| e.stage() == 0)
            .filter(|e| {
                head.get(&e.path)
                    .is_none_or(|f| f.oid != e.oid || f.mode != e.mode)
            })
            .filter(|e| {
                specs.is_empty()
                    || specs.iter().any(|s| {
                        let s = s.trim_end_matches('/');
                        e.path == s || e.path.starts_with(&format!("{s}/"))
                    })
            })
            .map(|e| e.path.clone())
            .collect();
        for p in paths {
            self.update_one(&p)?;
        }
        Ok(())
    }

    fn index_info(&mut self, nul: bool) -> CliResult<()> {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;
        let sep = if nul { b'\0' } else { b'\n' };
        for rec in input.split(|b| *b == sep) {
            let line = String::from_utf8_lossy(rec);
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            let (meta, path) = line
                .split_once('\t')
                .ok_or_else(|| CliError::fatal(format!("malformed index info {line}")))?;
            let f: Vec<&str> = meta.split(' ').collect();
            let (mode, oid, stage) = match f.as_slice() {
                [m, o] => (*m, *o, 0u16),
                [m, t, o] if !t.chars().all(|c| c.is_ascii_digit()) => (*m, *o, 0),
                [m, o, s] => (*m, *o, s.parse().unwrap_or(0)),
                _ => return Err(CliError::fatal(format!("malformed index info {line}"))),
            };
            let mode_u = u32::from_str_radix(mode, 8)
                .map_err(|_| CliError::fatal(format!("malformed index info {line}")))?;
            if mode_u == 0 {
                self.index
                    .entries
                    .retain(|e| !(e.path == path && (stage == 0 || e.stage() == stage)));
                self.st.changed = true;
                self.report(format!("remove '{path}'"));
                continue;
            }
            let oid = Oid::from_hex(oid)
                .map_err(|_| CliError::fatal(format!("malformed index info {line}")))?;
            if stage == 0 {
                self.index.entries.retain(|e| e.path != path);
            } else {
                self.index
                    .entries
                    .retain(|e| !(e.path == path && (e.stage() == 0 || e.stage() == stage)));
            }
            self.index
                .entries
                .push(zero_entry(path, oid, mode_u, stage));
            sort_entries(&mut self.index.entries);
            self.st.changed = true;
            self.report(format!("add '{path}'"));
        }
        Ok(())
    }
}

fn filemode_trusted(repo: &Repository) -> bool {
    let cfg = arks_config::ConfigSet::load(Some(repo.git_dir()));
    cfg.get_bool("core.filemode").unwrap_or(true)
}

/// Concurrency refusals kept from the M19/M20 issue catalog (lease ≠ skip).
fn refuse_locked(repo: &Repository, items: &[Item]) -> CliResult<()> {
    let lock = arks_index::index_lock_path(&repo.index_path());
    let has = |n: &str| {
        items
            .iter()
            .any(|i| matches!(i, Item::Opt { key, .. } if *key == n))
    };
    if lock.is_file() {
        let why = if has("refresh") || has("really-refresh") {
            "MISC-012 — update-index --refresh honors exclusive lock; no poll-skip"
        } else if has("chmod") {
            "IDX-013 — update-index --chmod under exclusive index.lock refuse; lease ≠ skip"
        } else if has("skip-worktree") {
            "IDX-010 — skip-worktree bit flip under exclusive index.lock refuse; lease ≠ skip"
        } else {
            "IDX-011 — index extension/version rewrite under exclusive index.lock refuse; lease ≠ skip"
        };
        return Err(CliError::Msg(format!(
            "{} (index locked; {why})",
            lock.display()
        )));
    }
    let shared = repo.git_dir().join("sharedindex.lock");
    if shared.is_file() {
        return Err(CliError::Msg(format!(
            "{} (IDX-004 — split-index / sharedindex.lock contention refuse; lease ≠ skip)",
            shared.display()
        )));
    }
    let uc_lock = repo.git_dir().join("untracked-cache.lock");
    if uc_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (IDX-005 — untracked-cache / sparse-index expand race refuse; lease ≠ skip)",
            uc_lock.display()
        )));
    }
    Ok(())
}

pub(crate) fn update_index(args: &[String]) -> CliResult<()> {
    // `-g`/`--again` consumes the rest of argv as pathspecs (git callback).
    let cut = args
        .iter()
        .position(|a| a == "-g" || (a.len() >= 4 && "--again".starts_with(a.as_str())))
        .map(|i| i + 1)
        .unwrap_or(args.len());
    let again_specs: Vec<String> = args[cut..].to_vec();
    let items = opt_parse::parse(
        "update-index",
        &args[..cut],
        UPDATE_INDEX_SPEC,
        opt_parse::Mode::default(),
    )?;
    let mut repo = open_repo()?;
    refuse_locked(&repo, &items.items)?;
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    let disk_version = index.version;
    let mut ui = Ui {
        repo: &mut repo,
        work: work.clone(),
        index,
        st: UiState::default(),
        out: String::new(),
    };
    let mut nul = false;
    let mut read_stdin = false;
    let mut index_info = false;
    let mut force_write = false;
    let mut show_version = false;
    let mut new_version: Option<u32> = None;
    // A trailing `--` leaves no item but still follows `--stdin` in argv.
    let trailing_dashdash = args[..cut].last().is_some_and(|a| a == "--");
    let items = &items.items;
    let mut i = 0;
    while i < items.len() {
        let it = items[i].clone();
        i += 1;
        let (name, value, neg) = match it {
            Item::Arg(p) | Item::AfterDashDash(p) => {
                if read_stdin {
                    return Err(CliError::Exit(
                        129,
                        "error: option 'stdin' must be the last argument".into(),
                    ));
                }
                let path = index_path(&work, &p);
                ui.update_one(&path)?;
                continue;
            }
            Item::Opt { key, val, neg } => (key, val, neg),
            Item::Unknown(_) | Item::Number(_) => continue,
        };
        let on = !neg;
        match name {
            "-q" => ui.st.quiet = true,
            "ignore-submodules" => ui.st.ignore_submodules = on,
            "add" => ui.st.add = on,
            "replace" => ui.st.replace = on,
            "remove" => ui.st.remove = on,
            "unmerged" => ui.st.unmerged = on,
            "refresh" => ui.refresh(false)?,
            "really-refresh" => ui.refresh(true)?,
            "cacheinfo" => {
                let v = value.unwrap_or_default();
                let parts: Vec<&str> = v.splitn(3, ',').collect();
                if parts.len() == 3 {
                    ui.cacheinfo(parts[0], parts[1], parts[2])?;
                } else {
                    // Legacy `--cacheinfo <mode> <object> <path>`.
                    let take = |i: &mut usize| -> Option<String> {
                        match items.get(*i) {
                            Some(Item::Arg(p)) => {
                                *i += 1;
                                Some(p.clone())
                            }
                            _ => None,
                        }
                    };
                    let (Some(o), Some(p)) = (take(&mut i), take(&mut i)) else {
                        return Err(CliError::Exit(
                            129,
                            "error: option 'cacheinfo' expects <mode>,<sha1>,<path>".into(),
                        ));
                    };
                    ui.cacheinfo(&v, &o, &p)?;
                }
            }
            "chmod" => {
                ui.st.chmod = match value.as_deref() {
                    Some("+x") => Some(true),
                    Some("-x") => Some(false),
                    _ => {
                        return Err(CliError::Exit(
                            129,
                            "error: option 'chmod' expects \"+x\" or \"-x\"".into(),
                        ));
                    }
                }
            }
            "assume-unchanged" => ui.st.mark_valid = Some(on),
            "skip-worktree" => ui.st.mark_skip = Some(on),
            "ignore-skip-worktree-entries" => {}
            "info-only" => ui.st.info_only = on,
            "force-remove" => ui.st.force_remove = on,
            "-z" => nul = true,
            "stdin" => {
                if i < items.len() || trailing_dashdash {
                    return Err(CliError::Exit(
                        129,
                        "error: option 'stdin' must be the last argument".into(),
                    ));
                }
                read_stdin = true;
            }
            "index-info" => index_info = true,
            "unresolve" => ui.st.unresolve = true,
            "again" => ui.again(&again_specs)?,
            "ignore-missing" => ui.st.ignore_missing = on,
            "verbose" => ui.st.verbose = on,
            "clear-resolve-undo" => ui.st.changed = true,
            "index-version" => {
                if neg {
                    new_version = None;
                    continue;
                }
                let v = value.unwrap_or_default();
                let n: u32 = v.parse().map_err(|_| {
                    CliError::Exit(
                        129,
                        "error: option `index-version' expects a numerical value".to_string(),
                    )
                })?;
                if !(2..=4).contains(&n) {
                    return Err(CliError::fatal(format!(
                        "index-version {n} not in range: 2..4"
                    )));
                }
                new_version = Some(n);
            }
            "show-index-version" => show_version = on,
            // The fsmonitor-valid bit lives in the FSMN cache extension, which
            // arks does not keep; marking only validates the path.
            "fsmonitor-valid" => ui.st.mark_fsmonitor = Some(on),
            "split-index" | "untracked-cache" | "force-untracked-cache" => force_write = true,
            "test-untracked-cache" => {
                if on {
                    eprintln!("Testing mtime in '{}' ...... OK", work.display());
                }
            }
            "force-write-index" => force_write = on,
            "fsmonitor" => {
                if on {
                    let cfg = arks_config::ConfigSet::load(Some(ui.repo.git_dir()));
                    if cfg.get("core.fsmonitor").is_none() {
                        eprintln!(
                            "warning: core.fsmonitor is unset; set it if you really want to enable fsmonitor"
                        );
                    }
                }
                force_write = true;
            }
            _ => {}
        }
    }
    if index_info {
        ui.index_info(nul)?;
    }
    if read_stdin {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;
        let sep = if nul { b'\0' } else { b'\n' };
        for rec in input.split(|b| *b == sep) {
            let p = String::from_utf8_lossy(rec);
            let p = p.trim_end_matches('\r');
            if p.is_empty() {
                continue;
            }
            let path = index_path(&work, p);
            ui.update_one(&path)?;
        }
    }
    if show_version {
        ui.out.push_str(&format!("{disk_version}\n"));
    }
    if let Some(v) = new_version {
        if v != ui.index.version {
            ui.index.version = v;
            ui.st.changed = true;
        }
    }
    let Ui {
        repo,
        mut index,
        st,
        out,
        ..
    } = ui;
    if st.changed || force_write {
        sort_entries(&mut index.entries);
        index.extensions.clear();
        crate::util::write_index(repo, &index)?;
    }
    print!("{out}");
    if st.needs_update && !st.quiet {
        return Err(CliError::silent(1));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// read-tree

const READ_TREE_SPEC: &[OptDef] = &[
    o("index-output", ' ', Req, true),
    o("empty", ' ', No, true),
    o("verbose", 'v', No, true),
    o("-m", 'm', No, false),
    o("trivial", ' ', No, true),
    o("aggressive", ' ', No, true),
    o("reset", ' ', No, true),
    o("prefix", ' ', Req, false),
    o("-u", 'u', No, false),
    o("exclude-per-directory", ' ', Req, false),
    o("-i", 'i', No, false),
    o("dry-run", 'n', No, true),
    o("sparse-checkout", ' ', No, true),
    o("debug-unpack", ' ', No, true),
    o("recurse-submodules", ' ', Opt, true),
    o("quiet", 'q', No, true),
];

#[derive(Clone, Copy, PartialEq, Eq)]
struct Blob {
    oid: Oid,
    mode: u32,
}

fn tree_files(repo: &Repository, rev: &str) -> CliResult<BTreeMap<String, Blob>> {
    let oid = resolve_rev(repo, rev)
        .map_err(|_| CliError::fatal(format!("failed to unpack tree object {rev}")))?;
    let files = crate::cmd::switch_tree::commit_files(repo, &oid)
        .map_err(|_| CliError::fatal(format!("failed to unpack tree object {rev}")))?;
    Ok(files
        .into_iter()
        .map(|(p, f)| {
            (
                p,
                Blob {
                    oid: f.oid,
                    mode: f.mode,
                },
            )
        })
        .collect())
}

/// Write a blob to the work tree (files, executables, symlinks).
pub(crate) fn write_worktree_file(
    repo: &Repository,
    abs: &Path,
    oid: &Oid,
    mode: u32,
) -> CliResult<()> {
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    if abs.is_dir() && !abs.is_symlink() {
        fs::remove_dir_all(abs)?;
    } else if abs.exists() || abs.is_symlink() {
        fs::remove_file(abs)?;
    }
    let blob = repo.get_object(oid)?;
    if mode == 0o120000 {
        #[cfg(unix)]
        {
            let target = String::from_utf8_lossy(&blob.payload).to_string();
            std::os::unix::fs::symlink(target, abs)?;
            return Ok(());
        }
    }
    if mode == 0o160000 {
        fs::create_dir_all(abs)?;
        return Ok(());
    }
    fs::write(abs, &blob.payload)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perm = if mode == 0o100755 { 0o755 } else { 0o644 };
        fs::set_permissions(abs, fs::Permissions::from_mode(perm))?;
    }
    Ok(())
}

fn worktree_matches(repo: &Repository, work: &Path, e: &IndexEntry) -> bool {
    let abs = work.join(&e.path);
    let Ok(meta) = fs::symlink_metadata(&abs) else {
        return false;
    };
    if index_stat_matches(e, &meta) {
        return true;
    }
    let bytes = if meta.file_type().is_symlink() {
        match fs::read_link(&abs) {
            Ok(t) => t.to_string_lossy().into_owned().into_bytes(),
            Err(_) => return false,
        }
    } else {
        match fs::read(&abs) {
            Ok(b) => b,
            Err(_) => return false,
        }
    };
    RawObject::new(ObjectKind::Blob, bytes)
        .oid(repo.object_format())
        .map(|o| o == e.oid)
        .unwrap_or(false)
}

/// `verify_uptodate` (unpack-trees): a missing file is fine; otherwise the
/// full stat data (mtime, ctime, inode, owner, size, mode) must match the
/// entry, and a racily clean entry (mtime not older than the index file) is
/// confirmed by content.
fn stat_uptodate(
    repo: &Repository,
    work: &Path,
    e: &IndexEntry,
    index_mtime: Option<(u32, u32)>,
) -> bool {
    let abs = work.join(&e.path);
    let Ok(meta) = fs::symlink_metadata(&abs) else {
        return true;
    };
    let (cs, cn, ms, mn) = crate::cmd::shared::file_times(&meta);
    #[cfg(unix)]
    let (ino, uid, gid) = {
        use std::os::unix::fs::MetadataExt;
        (meta.ino() as u32, meta.uid(), meta.gid())
    };
    #[cfg(not(unix))]
    let (ino, uid, gid) = (e.ino, e.uid, e.gid);
    let mode = if meta.file_type().is_symlink() {
        0o120000
    } else if meta.is_dir() {
        0o040000
    } else {
        crate::cmd::shared::file_mode(&meta)
    };
    // Entries written without inode/owner data (arks tree switches) cannot be
    // stat-compared strictly; confirm them by content instead.
    if e.ino == 0 && e.uid == 0 && e.gid == 0 && e.dev == 0 {
        return worktree_matches(repo, work, e);
    }
    let same = e.mtime_sec == ms
        && e.mtime_nsec == mn
        && e.ctime_sec == cs
        && e.ctime_nsec == cn
        && e.ino == ino
        && e.uid == uid
        && e.gid == gid
        && e.size == meta.len().min(u32::MAX as u64) as u32
        && (e.mode == mode || !filemode_trusted(repo) && e.mode & 0o170000 == mode & 0o170000);
    if !same {
        return false;
    }
    let racy = index_mtime.is_none_or(|(s, n)| (e.mtime_sec, e.mtime_nsec) >= (s, n));
    if racy {
        return worktree_matches(repo, work, e);
    }
    true
}

fn index_file_mtime(repo: &Repository) -> Option<(u32, u32)> {
    let meta = fs::metadata(repo.index_path()).ok()?;
    let (_, _, s, n) = crate::cmd::shared::file_times(&meta);
    Some((s, n))
}

fn remove_empty_parents(work: &Path, file: &Path) {
    let mut cur = file.parent();
    while let Some(dir) = cur {
        if dir == work || fs::remove_dir(dir).is_err() {
            break;
        }
        cur = dir.parent();
    }
}

pub(crate) fn read_tree(args: &[String]) -> CliResult<()> {
    let items = opt_parse::parse(
        "read-tree",
        args,
        READ_TREE_SPEC,
        opt_parse::Mode::default(),
    )?;
    let trees: Vec<String> = items.operands();
    let merge = items.on("-m");
    let reset = items.on("reset");
    let prefix = items.value("prefix");
    let update = items.on("-u");
    let index_only = items.on("-i");
    let dry_run = items.on("dry-run");
    let aggressive = items.on("aggressive");
    let empty = items.on("empty");
    let index_output = items.value("index-output");
    if [merge, reset, prefix.is_some()]
        .iter()
        .filter(|b| **b)
        .count()
        > 1
    {
        return Err(CliError::fatal("Which one? -m, --reset, or --prefix?"));
    }
    let merging = merge || reset || prefix.is_some();
    if (update || index_only) && !merging {
        return Err(CliError::fatal(format!(
            "{} is meaningless without -m, --reset, or --prefix",
            if update { "-u" } else { "-i" }
        )));
    }
    if update && index_only {
        return Err(CliError::fatal("-u and -i at the same time makes no sense"));
    }
    if items.value("exclude-per-directory").is_some() && !update {
        return Err(CliError::fatal(
            "--exclude-per-directory is meaningless unless -u",
        ));
    }
    if empty && !trees.is_empty() {
        return Err(CliError::fatal(
            "passing trees as arguments contradicts --empty",
        ));
    }
    let mut repo = open_repo()?;
    // IDX-014 (M20 Cx-3): parallel read-tree contends on index.lock.
    let lock = arks_index::index_lock_path(&repo.index_path());
    if lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (IDX-014 — read-tree honors exclusive index.lock; no parallel merge dialect)",
            lock.display()
        )));
    }
    let work = work_tree(&repo).ok();
    let old = read_index(&repo)?;
    if trees.is_empty() && !empty && !merging {
        eprintln!(
            "warning: read-tree: emptying the index with no arguments is deprecated; use --empty"
        );
    }
    if merge && !reset && old.entries.iter().any(|e| e.stage() != 0) {
        return Err(CliError::fatal(
            "you need to resolve your current index first",
        ));
    }
    if merging && trees.is_empty() {
        return Err(CliError::Exit(129, "usage: git read-tree [(-m [--trivial] [--aggressive] | --reset | --prefix=<prefix>) [-u | -i]] [--index-output=<file>] [--no-sparse-checkout] (--empty | <tree-ish1> [<tree-ish2> [<tree-ish3>]])".into()));
    }
    let mut tset = Vec::new();
    for t in &trees {
        tset.push(tree_files(&repo, t)?);
    }
    let cur: BTreeMap<String, IndexEntry> = old
        .entries
        .iter()
        .filter(|e| e.stage() == 0)
        .map(|e| (e.path.clone(), e.clone()))
        .collect();
    let same = |e: &IndexEntry, b: &Blob| e.oid == b.oid && e.mode == b.mode;
    let mut result: Vec<IndexEntry> = Vec::new();
    // Paths whose work-tree file must be (re)written or removed with -u.
    let mut writes: Vec<(String, Option<Blob>)> = Vec::new();
    let idx_mtime = index_file_mtime(&repo);
    // unpack-trees `verify_uptodate` / `verify_absent` for a changed path.
    let check_clean = |p: &str, repo: &Repository| -> CliResult<()> {
        if index_only {
            return Ok(());
        }
        let Some(w) = work.as_ref() else {
            return Ok(());
        };
        if let Some(e) = cur.get(p) {
            if !stat_uptodate(repo, w, e, idx_mtime) {
                return Err(CliError::Exit(
                    128,
                    format!("error: Entry '{p}' not uptodate. Cannot merge."),
                ));
            }
        } else if update && w.join(p).is_file() {
            return Err(CliError::Exit(
                128,
                format!("error: Untracked working tree file '{p}' would be overwritten by merge."),
            ));
        }
        Ok(())
    };
    if empty || (trees.is_empty() && !merging) {
        // Empty index.
    } else if let Some(pfx) = &prefix {
        let pfx = pfx.trim_end_matches('/');
        // Bind merge: a new path may not collide with an existing entry.
        let existing: BTreeSet<&str> = old.entries.iter().map(|e| e.path.as_str()).collect();
        for p in tset[0].keys() {
            let path = if pfx.is_empty() {
                p.clone()
            } else {
                format!("{pfx}/{p}")
            };
            if existing.contains(path.as_str()) {
                return Err(CliError::Exit(
                    128,
                    format!("error: Entry '{path}' overlaps with '{path}'.  Cannot bind."),
                ));
            }
        }
        result = old.entries.clone();
        for (p, b) in &tset[0] {
            let path = if pfx.is_empty() {
                p.clone()
            } else {
                format!("{pfx}/{p}")
            };
            result.push(zero_entry(&path, b.oid, b.mode, 0));
            writes.push((path, Some(*b)));
        }
    } else if !merging {
        // Plain read: later trees overlay earlier ones.
        let mut map: BTreeMap<String, Blob> = BTreeMap::new();
        for t in &tset {
            for (p, b) in t {
                map.insert(p.clone(), *b);
            }
        }
        for (p, b) in map {
            result.push(zero_entry(&p, b.oid, b.mode, 0));
        }
    } else if tset.len() == 1 || reset {
        // One-way merge (`--reset` discards unmerged entries and local changes).
        let t = &tset[tset.len() - 1];
        for (p, b) in t {
            match cur.get(p) {
                Some(e) if same(e, b) => {
                    let mut keep = e.clone();
                    if reset && update {
                        if let Some(w) = work.as_ref() {
                            if !worktree_matches(&repo, w, e) {
                                writes.push((p.clone(), Some(*b)));
                                keep = zero_entry(p, b.oid, b.mode, 0);
                            }
                        }
                    }
                    result.push(keep);
                }
                _ => {
                    if !reset {
                        check_clean(p, &repo)?;
                    }
                    result.push(zero_entry(p, b.oid, b.mode, 0));
                    writes.push((p.clone(), Some(*b)));
                }
            }
        }
        for p in cur.keys() {
            if !t.contains_key(p) {
                if !reset {
                    check_clean(p, &repo)?;
                }
                writes.push((p.clone(), None));
            }
        }
    } else if tset.len() == 2 {
        let (h, m) = (&tset[0], &tset[1]);
        let paths: BTreeSet<&String> = h.keys().chain(m.keys()).chain(cur.keys()).collect();
        for p in paths {
            let (hv, mv, iv) = (h.get(p), m.get(p), cur.get(p));
            let i_is = |b: Option<&Blob>| match (iv, b) {
                (None, None) => true,
                (Some(e), Some(b)) => same(e, b),
                _ => false,
            };
            if hv == mv {
                if let Some(e) = iv {
                    result.push(e.clone());
                }
                continue;
            }
            if i_is(mv) {
                if let Some(e) = iv {
                    result.push(e.clone());
                }
                continue;
            }
            if !i_is(hv) {
                return Err(CliError::Exit(
                    128,
                    format!("error: Entry '{p}' would be overwritten by merge. Cannot merge."),
                ));
            }
            check_clean(p, &repo)?;
            match mv {
                Some(b) => {
                    result.push(zero_entry(p, b.oid, b.mode, 0));
                    writes.push((p.clone(), Some(*b)));
                }
                None => writes.push((p.clone(), None)),
            }
        }
    } else {
        let n = tset.len();
        let (o, a, b) = (&tset[n - 3], &tset[n - 2], &tset[n - 1]);
        let paths: BTreeSet<&String> = o
            .keys()
            .chain(a.keys())
            .chain(b.keys())
            .chain(cur.keys())
            .collect();
        for p in paths {
            let (ov, av, bv) = (o.get(p), a.get(p), b.get(p));
            let iv = cur.get(p);
            // The index must match HEAD (stage 2) for every path it holds.
            let index_ok = match (iv, av) {
                (None, _) => true,
                (Some(e), Some(x)) => same(e, x),
                (Some(_), None) => false,
            };
            let merged: Option<Option<Blob>> = match (ov, av, bv) {
                (_, Some(x), Some(y)) if x == y => Some(Some(*x)),
                (Some(oo), Some(x), Some(y)) if oo == x => Some(Some(*y)),
                (Some(oo), Some(x), Some(y)) if oo == y => Some(Some(*x)),
                (None, Some(x), None) => Some(Some(*x)),
                (None, None, Some(y)) => Some(Some(*y)),
                (None, None, None) => Some(None),
                (Some(_), None, None) if aggressive => Some(None),
                (Some(oo), None, Some(y)) if aggressive && oo == y => Some(None),
                (Some(oo), Some(x), None) if aggressive && oo == x => Some(None),
                _ => None,
            };
            if !index_ok {
                // A path the merge leaves exactly as the index has it is fine.
                let keeps = matches!((merged, iv), (Some(Some(m)), Some(e)) if same(e, &m));
                if !keeps {
                    return Err(CliError::Exit(
                        128,
                        format!("error: Entry '{p}' would be overwritten by merge. Cannot merge."),
                    ));
                }
            }
            let changes_index = match (merged, iv) {
                (Some(Some(m)), Some(e)) => !same(e, &m),
                (Some(None), Some(_)) => true,
                (None, Some(_)) => true,
                (Some(Some(_)), None) => true,
                _ => false,
            };
            if changes_index {
                check_clean(p, &repo)?;
            }
            match merged {
                Some(Some(m)) => match iv {
                    Some(e) if same(e, &m) => result.push(e.clone()),
                    _ => {
                        result.push(zero_entry(p, m.oid, m.mode, 0));
                        if av != Some(&m) {
                            writes.push((p.clone(), Some(m)));
                        }
                    }
                },
                Some(None) => {
                    if av.is_some() || iv.is_some() {
                        writes.push((p.clone(), None));
                    }
                }
                None => {
                    for (stage, v) in [(1u16, ov), (2, av), (3, bv)] {
                        if let Some(x) = v {
                            result.push(zero_entry(p, x.oid, x.mode, stage));
                        }
                    }
                }
            }
        }
    }
    sort_entries(&mut result);
    result.dedup_by(|a, b| a.path == b.path && a.stage() == b.stage());
    if dry_run {
        return Ok(());
    }
    if update {
        if let Some(w) = work.as_ref() {
            for (p, v) in &writes {
                let abs = w.join(p);
                match v {
                    Some(b) => {
                        write_worktree_file(&repo, &abs, &b.oid, b.mode)?;
                        if let Ok(meta) = fs::symlink_metadata(&abs) {
                            if let Some(e) =
                                result.iter_mut().find(|e| e.path == *p && e.stage() == 0)
                            {
                                let mut fresh = entry_from_stat(p, b.oid, &meta);
                                fresh.mode = b.mode;
                                *e = fresh;
                            }
                        }
                    }
                    None => {
                        if abs.is_file() || abs.is_symlink() {
                            let _ = fs::remove_file(&abs);
                            remove_empty_parents(w, &abs);
                        }
                    }
                }
            }
        }
    }
    let new_index = Index {
        version: old.version,
        entries: result,
        extensions: Vec::new(),
    };
    match index_output {
        Some(path) => {
            let bytes = new_index.write(repo.oid_kind())?;
            fs::write(path, bytes)?;
        }
        None => crate::util::write_index(&repo, &new_index)?,
    }
    let _ = &mut repo;
    Ok(())
}

// ---------------------------------------------------------------------------
// checkout-index

const CHECKOUT_INDEX_SPEC: &[OptDef] = &[
    o("all", 'a', No, true),
    o("ignore-skip-worktree-bits", ' ', No, true),
    o("force", 'f', No, true),
    o("quiet", 'q', No, true),
    o("no-create", 'n', No, false),
    o("create", ' ', No, false),
    o("index", 'u', No, true),
    o("-z", 'z', No, false),
    o("stdin", ' ', No, true),
    o("temp", ' ', No, true),
    o("prefix", ' ', Req, true),
    o("stage", ' ', Req, false),
];

fn temp_name(dir: &Path) -> PathBuf {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
        ^ u128::from(std::process::id());
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut n = seed;
    loop {
        let mut s = String::from(".merge_file_");
        for _ in 0..6 {
            s.push(alphabet[(n % 62) as usize] as char);
            n /= 62;
        }
        let p = dir.join(&s);
        if !p.exists() {
            return p;
        }
        n = n
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
    }
}

pub(crate) fn checkout_index(args: &[String]) -> CliResult<()> {
    let items = opt_parse::parse(
        "checkout-index",
        args,
        CHECKOUT_INDEX_SPEC,
        opt_parse::Mode::default(),
    )?;
    let all = items.on("all");
    let force = items.on("force");
    let quiet = items.on("quiet");
    let mut create = true;
    for (key, _, _) in items.opts() {
        match key {
            "no-create" => create = false,
            "create" => create = true,
            _ => {}
        }
    }
    let update_stat = items.on("index");
    let nul = items.on("-z");
    let stdin = items.on("stdin");
    let temp = items.on("temp");
    let ignore_skip = items.on("ignore-skip-worktree-bits");
    let prefix = items.value("prefix").unwrap_or_default();
    let stage = match items.value("stage").as_deref() {
        None => Some(0u16),
        Some("all") => None,
        Some(s @ ("1" | "2" | "3")) => Some(s.parse().unwrap_or(0)),
        Some(_) => {
            return Err(CliError::fatal("stage should be between 1 and 3 or all"));
        }
    };
    let temp = temp || stage.is_none();
    let paths = items.operands();
    if all && !paths.is_empty() {
        return Err(CliError::fatal(
            "git checkout-index: don't mix '--all' and explicit filenames",
        ));
    }
    if stdin && !paths.is_empty() {
        return Err(CliError::fatal(
            "git checkout-index: don't mix '--stdin' and explicit filenames",
        ));
    }
    if all && stdin {
        return Err(CliError::fatal(
            "git checkout-index: don't mix '--all' and '--stdin'",
        ));
    }
    let mut repo = open_repo()?;
    let work = work_tree(&repo)?;
    // M20 Cx-6 MISC-020: an occupied --prefix export target is refused (parallel
    // checkout-index prefix collision class).
    if !prefix.is_empty() && !force {
        let dest = Path::new(&prefix);
        if dest.is_dir() && fs::read_dir(dest)?.next().is_some() {
            return Err(CliError::Msg(format!(
                "checkout-index --prefix '{}' occupied (MISC-020 — parallel checkout-index prefix collision refuse)",
                dest.display()
            )));
        }
    }
    let mut index = read_index(&repo)?;
    let mut names: Vec<String> = paths.iter().map(|p| index_path(&work, p)).collect();
    if stdin {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;
        let sep = if nul { b'\0' } else { b'\n' };
        for rec in input.split(|b| *b == sep) {
            let p = String::from_utf8_lossy(rec);
            let p = p.trim_end_matches('\r');
            if !p.is_empty() {
                names.push(index_path(&work, p));
            }
        }
    }
    if all {
        let mut seen = BTreeSet::new();
        for e in &index.entries {
            if seen.insert(e.path.clone()) {
                names.push(e.path.clone());
            }
        }
    }
    let mut failed = false;
    let mut out = String::new();
    let mut changed = false;
    let idx_mtime = index_file_mtime(&repo);
    let base = if prefix.is_empty() {
        work.clone()
    } else {
        std::env::current_dir()?.join(&prefix)
    };
    for name in &names {
        let picks: Vec<usize> = index
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.path == *name && stage.is_none_or(|s| e.stage() == s || (all && e.stage() == 0))
            })
            .map(|(i, _)| i)
            .collect();
        let picks: Vec<usize> = if stage.is_none() {
            picks
                .into_iter()
                .filter(|&i| index.entries[i].stage() != 0)
                .collect()
        } else {
            picks
        };
        if picks.is_empty() {
            if all || (stage.is_none() && index.entries.iter().any(|e| e.path == *name)) {
                continue;
            }
            failed = true;
            if !quiet {
                let in_index = index.entries.iter().any(|e| e.path == *name);
                if in_index {
                    if let Some(s) = stage {
                        eprintln!("git checkout-index: {name} does not exist at stage {s}");
                    }
                } else {
                    eprintln!("git checkout-index: {name} is not in the cache");
                }
            }
            continue;
        }
        if all && stage == Some(0) && index.entries[picks[0]].stage() != 0 {
            continue;
        }
        if temp {
            let mut cols: Vec<String> = vec![".".into(); 3];
            for &i in &picks {
                let e = &index.entries[i];
                let tmp = temp_name(&std::env::current_dir()?);
                let blob = repo.get_object(&e.oid)?;
                fs::write(&tmp, &blob.payload)?;
                let shown = tmp
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if stage.is_none() {
                    cols[(e.stage() as usize).saturating_sub(1).min(2)] = shown;
                } else {
                    cols[0] = shown;
                }
            }
            let term = if nul { '\0' } else { '\n' };
            if stage.is_none() {
                out.push_str(&format!(
                    "{} {} {}\t{name}{term}",
                    cols[0], cols[1], cols[2]
                ));
            } else {
                out.push_str(&format!("{}\t{name}{term}", cols[0]));
            }
            continue;
        }
        let i = picks[0];
        let e = index.entries[i].clone();
        if has_ext(&e, CE_SKIP_WORKTREE) && !ignore_skip {
            continue;
        }
        // `--prefix` is prepended as a string (`--prefix=pre-` → `pre-<path>`).
        let abs = if prefix.is_empty() {
            base.join(&e.path)
        } else {
            std::env::current_dir()?.join(format!("{prefix}{}", e.path))
        };
        let exists = fs::symlink_metadata(&abs).is_ok();
        if exists {
            if prefix.is_empty() && e.stage() == 0 && stat_uptodate(&repo, &work, &e, idx_mtime) {
                continue;
            }
            if !force {
                if !quiet {
                    eprintln!("{}{} already exists, no checkout", prefix, e.path);
                }
                failed = true;
                continue;
            }
        } else if !create {
            continue;
        }
        write_worktree_file(&repo, &abs, &e.oid, e.mode)?;
        if update_stat && prefix.is_empty() {
            if let Ok(meta) = fs::symlink_metadata(&abs) {
                let mut fresh = entry_from_stat(&e.path, e.oid, &meta);
                fresh.mode = e.mode;
                fresh.flags = e.flags;
                fresh.flags_extended = e.flags_extended;
                index.entries[i] = fresh;
                changed = true;
            }
        }
    }
    if changed {
        crate::util::write_index(&repo, &index)?;
    }
    let _ = &mut repo;
    io::stdout().write_all(out.as_bytes())?;
    if failed {
        return Err(CliError::silent(1));
    }
    Ok(())
}
