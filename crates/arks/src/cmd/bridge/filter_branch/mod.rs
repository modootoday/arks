//! `git filter-branch`: rewrite the history of the named refs through the
//! documented filters, keeping the originals under `refs/original/`.
//!
//! Primary: <https://git-scm.com/docs/git-filter-branch> (filters and their
//! order, `map` / `skip_commit` / `git_commit_non_empty_tree`, `--prune-empty`,
//! `--original`, `-d`, `-f`, `--state-branch`, "Remap to ancestor", exit
//! status 2 when nothing is rewritten). Observed peer behaviour (git
//! 2.39/2.43): the squelchable warning on stdout followed by a ten second
//! pause, the `Rewrite <id> (i/n)` progress, `Ref '<ref>' was rewritten` /
//! `was deleted`, filters running in `<tempdir>/t` with `GIT_DIR`,
//! `GIT_WORK_TREE=.` and `GIT_INDEX_FILE` exported, and the final checkout
//! of the rewritten `HEAD`.
//!
//! Filters run in one `sh` (see `shell`); the commit filter runs in its own
//! `sh -c` as documented. Helper functions the filters may call invoke
//! `arks` itself, never `git`.

mod shell;
mod tree;

use super::proc::sq;
use crate::cmd::revwalk::{self, Order, RevOpts};
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index};
use arks_object::{Commit, ObjectKind, RawObject, Signature};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use shell::FilterShell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const USAGE: &str = "usage: git filter-branch [--setup <command>] [--subdirectory-filter <directory>] [--env-filter <command>]
\t[--tree-filter <command>] [--index-filter <command>]
\t[--parent-filter <command>] [--msg-filter <command>]
\t[--commit-filter <command>] [--tag-name-filter <command>]
\t[--original <namespace>]
\t[-d <directory>] [-f | --force] [--state-branch <branch>]
\t[--] [<rev-list options>...]";

const WARNING: &str = "WARNING: git-filter-branch has a glut of gotchas generating mangled history
\t rewrites.  Hit Ctrl-C before proceeding to abort, then use an
\t alternative filtering tool such as 'git filter-repo'
\t (https://github.com/newren/git-filter-repo/) instead.  See the
\t filter-branch manual page for more details; to squelch this warning,
\t set FILTER_BRANCH_SQUELCH_WARNING=1.";

#[derive(Default)]
struct Opts {
    setup: Option<String>,
    subdir: Option<String>,
    env: Option<String>,
    tree: Option<String>,
    index: Option<String>,
    parent: Option<String>,
    msg: Option<String>,
    commit: Option<String>,
    tag_name: Option<String>,
    original: String,
    dir: Option<String>,
    force: bool,
    prune_empty: bool,
    remap: bool,
    state_branch: Option<String>,
    revs: Vec<String>,
}

fn parse_opts(args: &[String]) -> CliResult<Opts> {
    let mut o = Opts {
        original: "refs/original".into(),
        ..Opts::default()
    };
    let usage = || CliError::Exit(1, USAGE.to_string());
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--" => {
                i += 1;
                break;
            }
            "-f" | "--force" => o.force = true,
            "--prune-empty" => o.prune_empty = true,
            "--remap-to-ancestor" => o.remap = true,
            _ if a.starts_with('-') => {
                let (name, inline) = match a.split_once('=') {
                    Some((n, v)) if n.starts_with("--") => (n, Some(v.to_string())),
                    _ => (a.as_str(), None),
                };
                let slot: &mut Option<String> = match name {
                    "--setup" => &mut o.setup,
                    "--subdirectory-filter" => &mut o.subdir,
                    "--env-filter" => &mut o.env,
                    "--tree-filter" => &mut o.tree,
                    "--index-filter" => &mut o.index,
                    "--parent-filter" => &mut o.parent,
                    "--msg-filter" => &mut o.msg,
                    "--commit-filter" => &mut o.commit,
                    "--tag-name-filter" => &mut o.tag_name,
                    "-d" => &mut o.dir,
                    "--state-branch" => &mut o.state_branch,
                    "--original" => {
                        let v = match inline {
                            Some(v) => v,
                            None => {
                                i += 1;
                                args.get(i).cloned().ok_or_else(usage)?
                            }
                        };
                        o.original = v;
                        i += 1;
                        continue;
                    }
                    _ => return Err(usage()),
                };
                let v = match inline {
                    Some(v) => v,
                    None => {
                        i += 1;
                        args.get(i).cloned().ok_or_else(usage)?
                    }
                };
                *slot = Some(v);
            }
            _ => break,
        }
        i += 1;
    }
    o.revs = args[i..].to_vec();
    if o.subdir.is_some() {
        o.remap = true;
    }
    o.original = format!("{}/", o.original.trim_end_matches('/'));
    Ok(o)
}

fn die(msg: impl Into<String>) -> CliError {
    CliError::Exit(1, msg.into())
}

pub fn cmd_filter_branch(args: &[String]) -> CliResult<()> {
    if std::env::var_os("FILTER_BRANCH_SQUELCH_WARNING").is_none_or(|v| v.is_empty()) {
        println!("{WARNING}");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        std::thread::sleep(std::time::Duration::from_secs(10));
        println!("Proceeding with filter-branch...");
    }
    let o = parse_opts(args)?;
    if o.prune_empty && o.commit.is_some() {
        return Err(die(
            "Cannot set --prune-empty and --commit-filter at the same time",
        ));
    }
    let mut repo = open_repo()?;
    let top = repo.work_tree().map(Path::to_path_buf);
    if let Some(t) = &top {
        require_clean(&repo, t)?;
    }
    let base = top.clone().unwrap_or_else(|| repo.git_dir().to_path_buf());
    let tempdir = match &o.dir {
        Some(d) => abs(&base, d),
        None => base.join(".git-rewrite"),
    };
    if tempdir.exists() {
        if o.force {
            fs::remove_dir_all(&tempdir)?;
        } else {
            return Err(die(format!(
                "{} already exists, please remove it",
                tempdir.display()
            )));
        }
    }
    if !o.force && !repo.refs().list_refs(&o.original)?.is_empty() {
        return Err(die(format!(
            "Cannot create a new backup.\nA previous backup already exists in {}\nForce overwriting the backup with -f",
            o.original
        )));
    }
    fs::create_dir_all(tempdir.join("t"))?;
    fs::create_dir_all(tempdir.join("map"))?;
    let r = Rewrite::new(&mut repo, &o, &tempdir, top.as_deref()).and_then(|mut rw| rw.run());
    let _ = fs::remove_dir_all(&tempdir);
    r
}

fn abs(base: &Path, p: &str) -> PathBuf {
    let p = Path::new(p);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

/// git's `require_clean_work_tree` for "rewrite branches".
fn require_clean(repo: &Repository, top: &Path) -> CliResult<()> {
    let index = read_index(repo)?;
    let head: HashMap<String, (Oid, u32)> = match repo.head_oid() {
        Ok(h) => crate::cmd::switch_tree::commit_files(repo, &h)?
            .into_iter()
            .map(|(p, f)| (p, (f.oid, f.mode)))
            .collect(),
        Err(_) => HashMap::new(),
    };
    let mut staged = index.entries.iter().any(|e| e.stage() != 0);
    let stage0: Vec<_> = index.entries.iter().filter(|e| e.stage() == 0).collect();
    if stage0.len() != head.len()
        || stage0
            .iter()
            .any(|e| head.get(&e.path) != Some(&(e.oid, e.mode)))
    {
        staged = true;
    }
    let mut unstaged = false;
    for e in &stage0 {
        if e.mode == 0o160000 {
            continue;
        }
        let abs = top.join(&e.path);
        let Ok(meta) = fs::symlink_metadata(&abs) else {
            unstaged = true;
            break;
        };
        if crate::cmd::diffcore::sources::stat_clean(e, &meta) {
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
        let oid = RawObject::new(ObjectKind::Blob, bytes).oid(repo.object_format())?;
        if oid != e.oid || crate::cmd::shared::file_mode(&meta) != e.mode && e.mode != 0o120000 {
            unstaged = true;
            break;
        }
    }
    match (unstaged, staged) {
        (false, false) => Ok(()),
        (true, false) => Err(die("Cannot rewrite branches: You have unstaged changes.")),
        (true, true) => Err(die(
            "Cannot rewrite branches: You have unstaged changes.\nAdditionally, your index contains uncommitted changes.",
        )),
        (false, true) => Err(die(
            "Cannot rewrite branches: Your index contains uncommitted changes.",
        )),
    }
}

/// Identity values the filters see and may change.
struct Ident {
    author: [String; 3],
    committer: [String; 3],
}

struct Rewrite<'a> {
    repo: &'a mut Repository,
    o: &'a Opts,
    tempdir: PathBuf,
    top: Option<PathBuf>,
    exe: String,
}

impl<'a> Rewrite<'a> {
    fn new(
        repo: &'a mut Repository,
        o: &'a Opts,
        tempdir: &Path,
        top: Option<&Path>,
    ) -> CliResult<Self> {
        let exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "arks".into());
        Ok(Self {
            repo,
            o,
            tempdir: tempdir.to_path_buf(),
            top: top.map(Path::to_path_buf),
            exe,
        })
    }

    fn map_dir(&self) -> PathBuf {
        self.tempdir.join("map")
    }

    /// `map <id>`: the rewritten ids, or the id itself when not rewritten.
    fn map(&self, oid: &Oid) -> Vec<Oid> {
        match fs::read_to_string(self.map_dir().join(oid.to_hex())) {
            Ok(text) => text
                .split_whitespace()
                .filter_map(|h| Oid::from_hex(h).ok())
                .collect(),
            Err(_) => vec![*oid],
        }
    }

    fn was_rewritten(&self, oid: &Oid) -> bool {
        self.map_dir().join(oid.to_hex()).exists()
    }

    /// The refs named by the positive revision arguments.
    fn refs_to_rewrite(&self, rev: &RevOpts) -> CliResult<Vec<String>> {
        let mut out: Vec<String> = Vec::new();
        let mut push = |r: String| {
            if !out.contains(&r) {
                out.push(r);
            }
        };
        let revs: Vec<String> = if rev.revs.is_empty() {
            vec!["HEAD".into()]
        } else {
            rev.revs.clone()
        };
        for r in revs {
            if r.starts_with('^') {
                continue;
            }
            if let Some(code) = r.strip_prefix('\u{0}') {
                for name in revwalk::select_refs(self.repo, code)? {
                    if name != "HEAD" && !name.starts_with(&self.o.original) {
                        push(name);
                    }
                }
                continue;
            }
            let pos = if let Some((_, b)) = r.split_once("...") {
                vec![
                    b.to_string(),
                    r.split_once("...")
                        .map(|x| x.0.to_string())
                        .unwrap_or_default(),
                ]
            } else if let Some((_, b)) = r.split_once("..") {
                vec![b.to_string()]
            } else {
                vec![r.clone()]
            };
            for p in pos {
                let p = if p.is_empty() { "HEAD".to_string() } else { p };
                if resolve_rev(self.repo, &p).is_err() {
                    return Err(CliError::fatal(format!(
                        "ambiguous argument '{p}': unknown revision or path not in the working tree.\n\
                         Use '--' to separate paths from revisions, like this:\n\
                         'git <command> [<revision>...] -- [<file>...]'"
                    )));
                }
                if let Some(full) = self.full_ref_name(&p) {
                    push(full);
                }
            }
        }
        Ok(out)
    }

    /// `rev-parse --symbolic-full-name` for a plain ref argument.
    fn full_ref_name(&self, name: &str) -> Option<String> {
        if name == "HEAD" {
            return match self.repo.head_target().ok()? {
                arks_refs::RefTarget::Symbolic(t) => Some(t),
                _ => None,
            };
        }
        let cands = [
            name.to_string(),
            format!("refs/{name}"),
            format!("refs/tags/{name}"),
            format!("refs/heads/{name}"),
            format!("refs/remotes/{name}"),
            format!("refs/remotes/{name}/HEAD"),
        ];
        cands
            .into_iter()
            .find(|c| c.starts_with("refs/") && self.repo.refs().resolve(c).is_ok())
    }

    fn run(&mut self) -> CliResult<()> {
        let o = self.o;
        // Revision arguments and paths for the walk.
        let (rev_args, mut paths): (Vec<String>, Vec<String>) =
            match o.revs.iter().position(|a| a == "--") {
                Some(i) => (o.revs[..i].to_vec(), o.revs[i + 1..].to_vec()),
                None => (o.revs.clone(), Vec::new()),
            };
        if let Some(d) = &o.subdir {
            paths = vec![d.trim_end_matches('/').to_string()];
        }
        let remap = o.remap || !paths.is_empty();
        let mut rev = RevOpts::default();
        let mut a = crate::cmd::diffcore::options::Args::new(&rev_args);
        while a.i < rev_args.len() {
            let cur = a.cur().to_string();
            if rev.take(&mut a)? {
                a.i += 1;
                continue;
            }
            if cur.starts_with('-') && cur.len() > 1 {
                return Err(CliError::fatal(format!("unrecognized argument: {cur}")));
            }
            rev.push_rev(&cur);
            a.i += 1;
        }
        let refs = self.refs_to_rewrite(&rev)?;
        if refs.is_empty() {
            return Err(die("You must specify a ref to rewrite."));
        }
        rev.order = Some(Order::Topo);
        rev.reverse = true;
        rev.rewrite = true;
        rev.simplify_merges = !paths.is_empty();
        rev.specs = paths.clone();
        let tips = revwalk::resolve_tips(self.repo, &rev, Some("HEAD"))?;
        let list = revwalk::walk(self.repo, &rev, &tips)?;
        self.load_state()?;
        if list.is_empty() {
            return Err(CliError::Exit(2, "Found nothing to rewrite".into()));
        }
        println!();
        let mut sh = self.start_shell()?;
        let start = Instant::now();
        let n = list.len();
        for (i, w) in list.iter().enumerate() {
            let secs = start.elapsed().as_secs();
            let remaining = if i == 0 {
                0
            } else {
                secs * (n - i) as u64 / i as u64
            };
            print!(
                "\rRewrite {} ({}/{}) ({} seconds passed, remaining {} predicted)    ",
                w.oid.to_hex(),
                i + 1,
                n,
                secs,
                remaining
            );
            let parents = if paths.is_empty() {
                w.commit.parents.clone()
            } else {
                w.parents.clone()
            };
            let r = self.rewrite_one(&mut sh, &w.oid, &w.commit, &parents);
            if let Err(e) = r {
                sh.finish();
                return Err(e);
            }
        }
        println!();
        let r = self.update_refs(&mut sh, &refs, remap);
        sh.finish();
        r?;
        self.save_state()?;
        self.checkout_head()
    }

    fn start_shell(&self) -> CliResult<FilterShell> {
        let o = self.o;
        let dir = self.tempdir.to_string_lossy().into_owned();
        let t = self.tempdir.join("t");
        let git_dir = fs::canonicalize(self.repo.git_dir())
            .unwrap_or_else(|_| self.repo.git_dir().to_path_buf());
        let env = vec![
            (
                "GIT_DIR".to_string(),
                git_dir.to_string_lossy().into_owned(),
            ),
            ("GIT_WORK_TREE".to_string(), ".".to_string()),
            (
                "GIT_INDEX_FILE".to_string(),
                format!("{}/../index", t.display()),
            ),
        ];
        let fns = functions();
        let mut pre = format!(
            "__dir={}; __arks={}; export __dir __arks\n{fns}\n",
            sq(&dir),
            sq(&self.exe)
        );
        let var = |name: &str, v: &Option<String>| -> String {
            format!("{name}={}\n", sq(v.as_deref().unwrap_or("")))
        };
        pre.push_str(&var("__f_env", &o.env));
        pre.push_str(&var("__f_tree", &o.tree));
        pre.push_str(&var("__f_index", &o.index));
        pre.push_str(&var("__f_parent", &o.parent));
        pre.push_str(&var("__f_msg", &o.msg));
        pre.push_str(&var("__f_tag", &o.tag_name));
        pre.push_str(&format!(
            "__f_commit={}\n",
            sq(&format!("{fns}\n{}", o.commit.as_deref().unwrap_or("")))
        ));
        let mut sh = FilterShell::start(&t, &env, &pre)?;
        if let Some(setup) = &o.setup {
            let st = sh.run(&format!("{{ eval {}; }} </dev/null", sq(setup)))?;
            if st != 0 {
                sh.finish();
                return Err(die(format!("setup failed: {setup}")));
            }
        }
        Ok(sh)
    }

    fn read_ident(&self, c: &Commit) -> Ident {
        let date = |s: &Signature| format!("@{} {}", s.timestamp, s.timezone);
        Ident {
            author: [
                c.author.name.clone(),
                c.author.email.clone(),
                date(&c.author),
            ],
            committer: [
                c.committer.name.clone(),
                c.committer.email.clone(),
                date(&c.committer),
            ],
        }
    }

    fn rewrite_one(
        &mut self,
        sh: &mut FilterShell,
        oid: &Oid,
        commit: &Commit,
        parents: &[Oid],
    ) -> CliResult<()> {
        let o = self.o;
        let dir = self.tempdir.clone();
        let id = self.read_ident(commit);
        // 1. Environment (and --env-filter).
        let mut s = format!("GIT_COMMIT={}; export GIT_COMMIT\n", oid.to_hex());
        let names = [
            "GIT_AUTHOR_NAME",
            "GIT_AUTHOR_EMAIL",
            "GIT_AUTHOR_DATE",
            "GIT_COMMITTER_NAME",
            "GIT_COMMITTER_EMAIL",
            "GIT_COMMITTER_DATE",
        ];
        let vals: Vec<&String> = id.author.iter().chain(id.committer.iter()).collect();
        for (n, v) in names.iter().zip(&vals) {
            s.push_str(&format!("{n}={}; export {n}\n", sq(v)));
        }
        if o.env.is_some() {
            s.push_str("{ eval \"$__f_env\"; } </dev/null; __s=$?\n");
        } else {
            s.push_str("__s=0\n");
        }
        s.push_str(&format!(
            "printf '%s\\0' \"$GIT_AUTHOR_NAME\" \"$GIT_AUTHOR_EMAIL\" \"$GIT_AUTHOR_DATE\" \"$GIT_COMMITTER_NAME\" \"$GIT_COMMITTER_EMAIL\" \"$GIT_COMMITTER_DATE\" > {}\n(exit $__s)",
            sq(&dir.join("env.out").to_string_lossy())
        ));
        if sh.run(&s)? != 0 {
            return Err(die(format!(
                "env filter failed: {}",
                o.env.as_deref().unwrap_or("")
            )));
        }
        let env_vals: Vec<String> = fs::read(dir.join("env.out"))
            .map(|b| {
                b.split(|c| *c == 0)
                    .map(|x| String::from_utf8_lossy(x).into_owned())
                    .collect()
            })
            .unwrap_or_default();
        // 2. Tree, tree filter and index filter.
        let mut tree_oid = commit.tree;
        if let Some(d) = &o.subdir {
            tree_oid = match tree::subtree(self.repo, &commit.tree, d)? {
                Some(t) => t,
                None => tree::empty_tree(self.repo)?,
            };
        }
        let index_path = dir.join("index");
        let mut entries = tree::entries_of(self.repo, &tree_oid)?;
        let t = dir.join("t");
        if let Some(f) = &o.tree {
            tree::checkout(self.repo, &t, &entries)?;
            if sh.run("( eval \"$__f_tree\" ) </dev/null")? != 0 {
                return Err(die(format!("tree filter failed: {f}")));
            }
            entries = tree::scan(self.repo, &t, &entries)?;
        }
        if let Some(f) = &o.index {
            tree::write_index_file(self.repo, &index_path, entries)?;
            if sh.run("{ eval \"$__f_index\"; } </dev/null")? != 0 {
                return Err(die(format!("index filter failed: {f}")));
            }
            entries = tree::read_index_file(self.repo, &index_path)?;
        }
        let new_tree = if o.tree.is_some() || o.index.is_some() {
            tree::tree_of_entries(self.repo, &entries)?
        } else {
            tree_oid
        };
        // 3. Parents (and --parent-filter).
        let mut mapped: Vec<Oid> = Vec::new();
        for p in parents {
            for m in self.map(p) {
                if !mapped.contains(&m) {
                    mapped.push(m);
                }
            }
        }
        if let Some(f) = &o.parent {
            let pstr: String = mapped
                .iter()
                .map(|p| format!("-p {}", p.to_hex()))
                .collect::<Vec<_>>()
                .join(" ");
            fs::write(dir.join("parents"), format!("{pstr}\n"))?;
            let snippet = format!(
                "eval \"$__f_parent\" < {} > {}",
                sq(&dir.join("parents").to_string_lossy()),
                sq(&dir.join("parents.new").to_string_lossy())
            );
            if sh.run(&snippet)? != 0 {
                return Err(die(format!("parent filter failed: {f}")));
            }
            let text = fs::read_to_string(dir.join("parents.new")).unwrap_or_default();
            mapped.clear();
            let words: Vec<&str> = text.split_whitespace().collect();
            let mut k = 0;
            while k < words.len() {
                if words[k] == "-p" {
                    if let Some(w) = words.get(k + 1) {
                        let oid = resolve_rev(self.repo, w)
                            .map_err(|_| CliError::fatal(format!("not a valid object name {w}")))?;
                        if !mapped.contains(&oid) {
                            mapped.push(oid);
                        }
                    }
                    k += 2;
                } else {
                    k += 1;
                }
            }
        }
        // 4. Message (and --msg-filter).
        let raw = self.repo.get_object(oid)?.payload.to_vec();
        let msg = match find_body(&raw) {
            Some(i) => raw[i..].to_vec(),
            None => Vec::new(),
        };
        let msg_path = dir.join("message");
        fs::write(&msg_path, &msg)?;
        let new_msg_path = dir.join("message.new");
        if let Some(f) = &o.msg {
            let snippet = format!(
                "eval \"$__f_msg\" < {} > {}",
                sq(&msg_path.to_string_lossy()),
                sq(&new_msg_path.to_string_lossy())
            );
            if sh.run(&snippet)? != 0 {
                return Err(die(format!("msg filter failed: {f}")));
            }
        } else {
            fs::write(&new_msg_path, &msg)?;
        }
        let new_msg = fs::read(&new_msg_path)?;
        // 5. Commit (or --commit-filter).
        let ids: Vec<Oid> = if o.commit.is_some() {
            let mut cargs = vec![sq(&new_tree.to_hex())];
            for p in &mapped {
                cargs.push("-p".into());
                cargs.push(sq(&p.to_hex()));
            }
            let snippet = format!(
                "\"${{SHELL_PATH:-/bin/sh}}\" -c \"$__f_commit\" 'git commit-tree' {} < {} > {}",
                cargs.join(" "),
                sq(&new_msg_path.to_string_lossy()),
                sq(&dir.join("commit.out").to_string_lossy())
            );
            if sh.run(&snippet)? != 0 {
                return Err(die("could not write rewritten commit"));
            }
            let text = fs::read_to_string(dir.join("commit.out")).unwrap_or_default();
            let mut ids = Vec::new();
            for w in text.split_whitespace() {
                let oid = resolve_rev(self.repo, w)
                    .map_err(|_| die("could not write rewritten commit"))?;
                ids.push(oid);
            }
            ids
        } else if o.prune_empty && self.is_empty_commit(&new_tree, &mapped)? {
            mapped.clone()
        } else {
            vec![self.write_commit(new_tree, mapped, &env_vals, new_msg)?]
        };
        let body: String = ids.iter().map(|i| format!("{}\n", i.to_hex())).collect();
        fs::write(self.map_dir().join(oid.to_hex()), body)?;
        Ok(())
    }

    /// `--prune-empty`: one parent with the same tree, or a root commit
    /// with an empty tree.
    fn is_empty_commit(&mut self, tree: &Oid, parents: &[Oid]) -> CliResult<bool> {
        match parents {
            [] => {
                let empty = tree::empty_tree(self.repo)?;
                Ok(*tree == empty)
            }
            [p] => {
                let obj = self.repo.get_object(p)?;
                Ok(Commit::decode(&obj.payload)?.tree == *tree)
            }
            _ => Ok(false),
        }
    }

    fn write_commit(
        &mut self,
        tree: Oid,
        parents: Vec<Oid>,
        env: &[String],
        message: Vec<u8>,
    ) -> CliResult<Oid> {
        let get = |i: usize| env.get(i).cloned().unwrap_or_default();
        let sig = |n: String, e: String, d: String| -> CliResult<Signature> {
            let (timestamp, timezone) = crate::cmd::shared::parse_git_date(&d)
                .ok_or_else(|| CliError::fatal(format!("invalid date format: {d}")))?;
            Ok(Signature {
                name: n,
                email: e,
                timestamp,
                timezone,
            })
        };
        let mut c = Commit {
            tree,
            parents,
            author: sig(get(0), get(1), get(2))?,
            committer: sig(get(3), get(4), get(5))?,
            gpgsig: None,
            message: String::from_utf8_lossy(&message).into_owned(),
        };
        let (sign, key) = crate::cmd::sign::want_sign_commit(&[], self.repo)?;
        if sign {
            crate::cmd::sign::sign_commit(self.repo, &mut c, key.as_deref())?;
        }
        let format = self.repo.object_format();
        Ok(self
            .repo
            .store_mut()
            .put(format, &RawObject::new(ObjectKind::Commit, c.encode()?))?)
    }

    /// Nearest ancestor of `oid` that was rewritten ("Remap to ancestor").
    fn remap_ancestor(&self, oid: &Oid) -> CliResult<Option<Oid>> {
        let mut seen = HashSet::new();
        let mut queue = std::collections::VecDeque::from([*oid]);
        while let Some(c) = queue.pop_front() {
            if !seen.insert(c) {
                continue;
            }
            if self.was_rewritten(&c) {
                return Ok(Some(c));
            }
            let obj = self.repo.get_object(&c)?;
            if obj.kind != ObjectKind::Commit {
                continue;
            }
            queue.extend(Commit::decode(&obj.payload)?.parents);
        }
        Ok(None)
    }

    fn update_refs(&mut self, sh: &mut FilterShell, refs: &[String], remap: bool) -> CliResult<()> {
        let orig = self.o.original.clone();
        for r in refs {
            let Ok(raw) = self.repo.refs().resolve(r) else {
                continue;
            };
            let Ok(sha) = crate::cmd::shared::peel_tags(self.repo, &raw) else {
                continue;
            };
            if self.repo.get_object(&sha).map(|o| o.kind) != Ok(ObjectKind::Commit) {
                continue;
            }
            let mut target = sha;
            if !self.was_rewritten(&sha) && remap {
                if let Some(a) = self.remap_ancestor(&sha)? {
                    target = a;
                }
            }
            let new = self.map(&target);
            match new.as_slice() {
                [] => {
                    self.repo.refs_mut().delete(r, None)?;
                    println!("Ref '{r}' was deleted");
                }
                [one] if *one == sha => {
                    eprintln!("WARNING: Ref '{r}' is unchanged");
                    continue;
                }
                [one, rest @ ..] => {
                    if !rest.is_empty() {
                        eprintln!("WARNING: '{r}' was rewritten into multiple commits:");
                        for x in &new {
                            eprintln!("{}", x.to_hex());
                        }
                        eprintln!("WARNING: Ref '{r}' points to the first one now.");
                    }
                    self.repo.refs_mut().update(r, one, None)?;
                    let _ = crate::reflog::append_reflog(
                        self.repo,
                        r,
                        Some(&raw),
                        one,
                        "filter-branch: rewrite",
                    );
                    println!("Ref '{r}' was rewritten");
                }
            }
            let backup = format!("{orig}{r}");
            self.repo.refs_mut().update(&backup, &raw, None)?;
        }
        if self.o.tag_name.is_some() {
            self.rewrite_tags(sh)?;
        }
        Ok(())
    }

    /// `--tag-name-filter`: tags pointing (through tag objects) at rewritten
    /// commits get a new name from the filter; annotated tags are recreated
    /// without their signature.
    fn rewrite_tags(&mut self, sh: &mut FilterShell) -> CliResult<()> {
        let f = self.o.tag_name.clone().unwrap_or_default();
        let dir = self.tempdir.clone();
        let mut tags = self.repo.refs().list_refs("refs/tags/")?;
        tags.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, raw) in tags {
            let Ok(sha) = crate::cmd::shared::peel_tags(self.repo, &raw) else {
                continue;
            };
            if !self.was_rewritten(&sha) {
                continue;
            }
            let Some(new_sha) = self.map(&sha).into_iter().next() else {
                continue;
            };
            let short = name.trim_start_matches("refs/tags/").to_string();
            fs::write(dir.join("tag.in"), format!("{short}\n"))?;
            let snippet = format!(
                "eval \"$__f_tag\" < {} > {}",
                sq(&dir.join("tag.in").to_string_lossy()),
                sq(&dir.join("tag.out").to_string_lossy())
            );
            if sh.run(&snippet)? != 0 {
                return Err(die(format!("tag name filter failed: {f}")));
            }
            let new_name = fs::read_to_string(dir.join("tag.out"))
                .unwrap_or_default()
                .trim_end_matches('\n')
                .to_string();
            if new_name.is_empty() {
                continue;
            }
            println!(
                "{short} -> {new_name} ({} -> {})",
                sha.to_hex(),
                new_sha.to_hex()
            );
            let obj = self.repo.get_object(&raw)?;
            let target = if obj.kind == ObjectKind::Tag {
                let payload = retarget_tag(&obj.payload, &new_sha);
                let format = self.repo.object_format();
                self.repo
                    .store_mut()
                    .put(format, &RawObject::new(ObjectKind::Tag, payload))?
            } else {
                new_sha
            };
            self.repo
                .refs_mut()
                .update(&format!("refs/tags/{new_name}"), &target, None)
                .map_err(|_| die(format!("Could not write tag {new_name}")))?;
        }
        Ok(())
    }

    /// `--state-branch`: seed the map from the branch's `filter.map`.
    fn load_state(&mut self) -> CliResult<()> {
        let Some(b) = &self.o.state_branch else {
            return Ok(());
        };
        let full = format!("refs/heads/{b}");
        match self.repo.refs().resolve(&full) {
            Ok(c) => {
                eprintln!(
                    "Populating map from {b} ({}) into {}",
                    c.to_hex(),
                    self.map_dir().display()
                );
                let tree = Commit::decode(&self.repo.get_object(&c)?.payload)?.tree;
                if let Some(blob) =
                    crate::cmd::shared::tree_entry_at(self.repo, &tree, "filter.map")
                {
                    let text =
                        String::from_utf8_lossy(&self.repo.get_object(&blob)?.payload).into_owned();
                    for line in text.lines() {
                        if let Some((old, new)) = line.split_once(':') {
                            fs::write(self.map_dir().join(old), format!("{new}\n"))?;
                        }
                    }
                }
            }
            Err(_) => eprintln!("Branch {b} does not exist. Will create"),
        }
        Ok(())
    }

    /// `--state-branch`: save the map as a new commit on the branch.
    fn save_state(&mut self) -> CliResult<()> {
        let Some(b) = self.o.state_branch.clone() else {
            return Ok(());
        };
        eprintln!("Saving rewrite state to {b}");
        let mut lines: Vec<String> = Vec::new();
        for e in fs::read_dir(self.map_dir())? {
            let e = e?;
            let old = e.file_name().to_string_lossy().into_owned();
            let new = fs::read_to_string(e.path())?;
            if let Some(first) = new.split_whitespace().next() {
                lines.push(format!("{old}:{first}"));
            }
        }
        lines.sort();
        let body: String = lines.iter().map(|l| format!("{l}\n")).collect();
        let format = self.repo.object_format();
        let blob = self
            .repo
            .store_mut()
            .put(format, &RawObject::new(ObjectKind::Blob, body.into_bytes()))?;
        let entry = arks_object::TreeEntry {
            mode: arks_object::TreeMode::File,
            name: "filter.map".into(),
            oid: blob,
        };
        let tree = arks_object::Tree::new(vec![entry])?;
        let tree_oid = self
            .repo
            .store_mut()
            .put(format, &RawObject::new(ObjectKind::Tree, tree.encode()?))?;
        let full = format!("refs/heads/{b}");
        let parent = self.repo.refs().resolve(&full).ok();
        let c = Commit {
            tree: tree_oid,
            parents: parent.into_iter().collect(),
            author: crate::cmd::shared::commit_signature()?,
            committer: crate::cmd::shared::committer_signature()?,
            gpgsig: None,
            message: "Sync\n".into(),
        };
        let oid = self
            .repo
            .store_mut()
            .put(format, &RawObject::new(ObjectKind::Commit, c.encode()?))?;
        self.repo.refs_mut().update(&full, &oid, None)?;
        Ok(())
    }

    /// Bring the index and work tree to the rewritten `HEAD`.
    fn checkout_head(&mut self) -> CliResult<()> {
        let Some(top) = self.top.clone() else {
            return Ok(());
        };
        let Ok(head) = self.repo.head_oid() else {
            return Ok(());
        };
        let index = read_index(self.repo)?;
        let cur = tree::tree_of_entries(self.repo, &index.entries)?;
        let new_tree = Commit::decode(&self.repo.get_object(&head)?.payload)?.tree;
        if cur == new_tree {
            return Ok(());
        }
        // The switch reads `from` as a commit or a tree: the current index tree.
        crate::cmd::switch_tree::two_way_switch(self.repo, &top, Some(&cur), &head)?;
        Ok(())
    }
}

/// Offset of the commit message (after the header's blank line).
fn find_body(raw: &[u8]) -> Option<usize> {
    raw.windows(2).position(|w| w == b"\n\n").map(|i| i + 2)
}

/// Point a tag object at `target` and drop any signature block.
fn retarget_tag(payload: &[u8], target: &Oid) -> Vec<u8> {
    let text = String::from_utf8_lossy(payload);
    let mut out = String::new();
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if i == 0 && line.starts_with("object ") {
            out.push_str(&format!("object {}\n", target.to_hex()));
            continue;
        }
        out.push_str(line);
    }
    if let Some(pos) = out.find("-----BEGIN PGP SIGNATURE-----") {
        out.truncate(pos);
    } else if let Some(pos) = out.find("-----BEGIN SSH SIGNATURE-----") {
        out.truncate(pos);
    }
    out.into_bytes()
}

/// Shell functions the filters may call (git-filter-branch documents
/// `map`, `skip_commit` and `git_commit_non_empty_tree`).
fn functions() -> String {
    [
        "map() { if test -r \"$__dir/map/$1\"; then cat \"$__dir/map/$1\"; else echo \"$1\"; fi; }",
        "skip_commit() { shift; while [ -n \"$1\" ]; do shift; map \"$1\"; shift; done; }",
        "git_commit_non_empty_tree() { if test $# = 3 && test \"$1\" = \"$(\"$__arks\" rev-parse \"$3^{tree}\")\"; then map \"$3\"; else \"$__arks\" commit-tree \"$@\"; fi; }",
    ]
    .join("\n")
}
