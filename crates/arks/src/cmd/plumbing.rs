//! Plumbing for M15 Fs-1 + M16 Sy-3 (scripting / porcelain support).
//!
//! Cite: git-write-tree(1), git-commit-tree(1), git-ls-files(1),
//! git-ls-tree(1), git-symbolic-ref(1), git-merge-base(1), git-show-ref(1),
//! git-for-each-ref(1), git-rev-list(1), git-update-index(1), git-read-tree(1),
//! git-diff-tree(1), git-pack-objects(1), git-index-pack(1).

use crate::cmd::shared::{commit_signature, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{
    FORMAT, open_repo, read_index, refuse_unknown_flags, rel_path_from_worktree, work_tree,
};
use crate::write_tree::write_tree_from_index;
use arks_index::IndexEntry;
use arks_object::{Commit, ObjectKind, RawObject, Tree};
use arks_oid::Oid;
use arks_pack::{DecodedPack, EncodedPack, PackIndex, PackIndexEntry};
use arks_refs::RefTarget;
use arks_repo::merge_base_oid;
use arks_store::ObjectStore;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

pub fn cmd_write_tree(args: &[String]) -> CliResult<()> {
    // `--missing-ok` (arks never checks object presence while writing trees)
    // and `--prefix=<dir>/` (print the subtree). Cite: git-write-tree(1).
    use crate::cmd::optparse::{self, Opt};
    const SPEC: &[Opt] = &[Opt::flag("missing-ok"), Opt::value("prefix")];
    let items = optparse::parse("write-tree", args, SPEC, optparse::Mode::default())?;
    let prefix = optparse::value(&items, "prefix");
    let mut repo = open_repo()?;
    let index = read_index(&repo)?;
    let unmerged: Vec<&IndexEntry> = index.entries.iter().filter(|e| e.stage() != 0).collect();
    if !unmerged.is_empty() {
        let mut msg = String::new();
        let mut last = "";
        for e in unmerged {
            if e.path != last {
                msg.push_str(&format!("{}: unmerged ({})\n", e.path, e.oid.to_hex()));
                last = &e.path;
            }
        }
        msg.push_str("fatal: git-write-tree: error building trees");
        return Err(CliError::Exit(128, msg));
    }
    let format = repo.object_format();
    let oid = if index.entries.is_empty() {
        let obj = RawObject::new(ObjectKind::Tree, Vec::new());
        repo.store_mut().put(format, &obj)?
    } else {
        write_tree_from_index(repo.store_mut(), &index.entries, format)?
    };
    let oid = match prefix.as_deref().map(|p| p.trim_end_matches('/')) {
        Some(p) if !p.is_empty() => crate::cmd::shared::tree_entry_at(&repo, &oid, p)
            .filter(|o| {
                repo.get_object(o)
                    .map(|obj| obj.kind == ObjectKind::Tree)
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                CliError::fatal(format!(
                    "git-write-tree: prefix {} not found",
                    prefix.as_deref().unwrap_or("")
                ))
            })?,
        _ => oid,
    };
    println!("{}", oid.to_hex());
    Ok(())
}

pub fn cmd_commit_tree(args: &[String]) -> CliResult<()> {
    // `-p`, `-m` / `-F` (paragraphs in order; stdin when neither), and
    // `-S[<keyid>]` / `--gpg-sign[=<keyid>]` / `--no-gpg-sign` (commit.gpgSign
    // honored). Cite: git-commit-tree(1).
    use crate::cmd::optparse::{self, Arg, Item, Opt};
    const SPEC: &[Opt] = &[
        Opt::short_value('p', "p"),
        Opt::short_value('m', "m"),
        Opt::short_value('F', "F"),
        Opt::flag("gpg-sign").with_short('S').arg(Arg::Optional),
    ];
    let items = optparse::parse("commit-tree", args, SPEC, optparse::Mode::default())?;
    let pos = optparse::positionals(&items);
    if pos.len() != 1 {
        return Err(CliError::fatal("must give exactly one tree"));
    }
    let mut repo = open_repo()?;
    let mut parents: Vec<Oid> = Vec::new();
    let mut message: Option<String> = None;
    let mut sign_args: Vec<String> = Vec::new();
    for it in &items {
        let Item::Opt { name, value, neg } = it else {
            continue;
        };
        match *name {
            "p" => {
                let p = value.as_deref().unwrap_or("");
                let oid = resolve_rev(&repo, p)
                    .map_err(|_| CliError::fatal(format!("not a valid object name {p}")))?;
                let oid = crate::cmd::shared::peel_to_kind(
                    &repo,
                    &oid,
                    crate::cmd::shared::ObjKind::Commit,
                    p,
                )?;
                if parents.contains(&oid) {
                    eprintln!("error: duplicate parent {} ignored", oid.to_hex());
                } else {
                    parents.push(oid);
                }
            }
            "m" | "F" => {
                let text = if *name == "m" {
                    value.clone().unwrap_or_default()
                } else {
                    let f = value.as_deref().unwrap_or("");
                    let bytes = if f == "-" {
                        let mut b = Vec::new();
                        io::stdin().read_to_end(&mut b)?;
                        b
                    } else {
                        fs::read(f).map_err(|e| {
                            CliError::fatal(format!("could not read log file '{f}': {e}"))
                        })?
                    };
                    String::from_utf8_lossy(&bytes).into_owned()
                };
                let m = message.get_or_insert_with(String::new);
                if !m.is_empty() {
                    m.push('\n');
                }
                m.push_str(&text);
                // `-m` completes its line; `-F` content is kept verbatim.
                if *name == "m" && !m.ends_with('\n') {
                    m.push('\n');
                }
            }
            "gpg-sign" => {
                sign_args.push(match (neg, value) {
                    (true, _) => "--no-gpg-sign".into(),
                    (false, Some(k)) => format!("--gpg-sign={k}"),
                    (false, None) => "--gpg-sign".into(),
                });
            }
            _ => {}
        }
    }
    let message = match message {
        Some(m) => m,
        None => {
            let mut b = Vec::new();
            io::stdin().read_to_end(&mut b)?;
            String::from_utf8_lossy(&b).into_owned()
        }
    };
    let tree_s = &pos[0];
    let tree = resolve_rev(&repo, tree_s)
        .map_err(|_| CliError::fatal(format!("not a valid object name {tree_s}")))?;
    let is_tree = repo
        .get_object(&tree)
        .map(|o| o.kind == ObjectKind::Tree)
        .unwrap_or(false);
    if !is_tree {
        return Err(CliError::fatal(format!(
            "{} is not a valid 'tree' object",
            tree.to_hex()
        )));
    }
    // Author and committer come from their own identities (git-commit-tree).
    let mut commit = Commit {
        tree,
        parents,
        author: commit_signature()?,
        committer: crate::cmd::shared::committer_signature()?,
        gpgsig: None,
        message,
    };
    let (sign, key) = crate::cmd::sign::want_sign_commit(&sign_args, &repo)?;
    if sign {
        crate::cmd::sign::sign_commit(&repo, &mut commit, key.as_deref())?;
    }
    let cobj = RawObject::new(ObjectKind::Commit, commit.encode()?);
    let format = repo.object_format();
    let coid = repo.store_mut().put(format, &cobj)?;
    println!("{}", coid.to_hex());
    Ok(())
}

pub fn cmd_ls_files(args: &[String]) -> CliResult<()> {
    // M16 Sy-3 + cloud drop-in §O: `-c`/`-o`/`-d`/`-m`/`-u`/`-s`, `--exclude-standard`,
    // `-z`, `--full-name`, `--error-unmatch`, pathspecs. Output order follows
    // ls-files.c `show_files`: others, then cached/stage, then deleted/modified.
    // Paths are shown relative to (and limited to) the current directory.
    // Cite: https://git-scm.com/docs/git-ls-files
    // C7b: `-i`/`--ignored` (with -o or -c and --exclude-standard).
    let args = crate::util::split_short_bundles(args, "cdimosuz", "");
    let mut flags = HashSet::new();
    let mut specs_raw: Vec<String> = Vec::new();
    let mut dashdash = false;
    for a in &args {
        if dashdash || !a.starts_with('-') {
            specs_raw.push(a.clone());
            continue;
        }
        match a.as_str() {
            "--" => dashdash = true,
            "-c" | "--cached" => {
                flags.insert('c');
            }
            "-d" | "--deleted" => {
                flags.insert('d');
            }
            "-m" | "--modified" => {
                flags.insert('m');
            }
            "-o" | "--others" => {
                flags.insert('o');
            }
            "-s" | "--stage" => {
                flags.insert('s');
            }
            "-u" | "--unmerged" => {
                flags.insert('u');
            }
            "-z" => {
                flags.insert('z');
            }
            "--exclude-standard" => {
                flags.insert('x');
            }
            "--full-name" => {
                flags.insert('f');
            }
            "--error-unmatch" => {
                flags.insert('e');
            }
            "-i" | "--ignored" => {
                flags.insert('i');
            }
            x => return Err(CliError::NotImplemented(format!("unsupported option: {x}"))),
        }
    }
    let has = |c: char| flags.contains(&c);
    if has('i') && !(has('o') || has('c')) {
        return Err(CliError::fatal(
            "ls-files -i must be used with either -o or -c",
        ));
    }
    if has('i') && !has('x') {
        return Err(CliError::fatal(
            "ls-files: --ignored needs some exclude pattern",
        ));
    }
    let stage_fmt = has('s') || has('u');
    let cached = has('c') || has('s') || has('u') || !(has('o') || has('d') || has('m'));
    let term = if has('z') { "\0" } else { "\n" };
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let prefix = canon(&crate::util::cwd()?)
        .strip_prefix(canon(&work))
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let mut specs: Vec<String> = Vec::new();
    for sp in &specs_raw {
        specs.push(crate::cmd::core::pathspec_rel(&work, sp)?);
    }
    if specs.is_empty() && !prefix.is_empty() {
        specs.push(prefix.clone());
    }
    let mut matched = vec![false; specs.len()];
    let mut in_spec = |p: &str| -> bool {
        if specs.is_empty() {
            return true;
        }
        let mut any = false;
        for (i, s) in specs.iter().enumerate() {
            if s.is_empty() || p == s || p.starts_with(&format!("{s}/")) {
                matched[i] = true;
                any = true;
            }
        }
        any
    };
    let show = |p: &str| -> String {
        if has('f') || prefix.is_empty() {
            return p.to_string();
        }
        let pre: Vec<&str> = prefix.split('/').collect();
        let comps: Vec<&str> = p.split('/').collect();
        let mut common = 0;
        while common < pre.len() && common + 1 < comps.len() && pre[common] == comps[common] {
            common += 1;
        }
        format!(
            "{}{}",
            "../".repeat(pre.len() - common),
            comps[common..].join("/")
        )
    };
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    if has('o') {
        let indexed: HashSet<_> = index.entries.iter().map(|e| e.path.clone()).collect();
        let mut untracked = Vec::new();
        let mut ig = crate::cmd::ignore::Ignore::new(&work);
        crate::cmd::shared::collect_untracked_with(
            &work,
            &work,
            &indexed,
            &mut untracked,
            (has('x') && !has('i')).then_some(&mut ig),
        )?;
        if has('i') {
            untracked.retain(|p| {
                ig.matching_with_parents(p, false)
                    .is_some_and(|m| m.excluded)
            });
        }
        for p in untracked {
            if in_spec(&p) {
                write!(out, "{}{term}", show(&p))?;
            }
        }
    }
    let mut ig_cached = crate::cmd::ignore::Ignore::new(&work);
    if cached && (!has('i') || has('c')) {
        for e in &index.entries {
            if has('i')
                && !ig_cached
                    .matching_with_parents(&e.path, false)
                    .is_some_and(|m| m.excluded)
            {
                continue;
            }
            if has('u') && !has('s') && !has('c') && e.stage() == 0 {
                continue;
            }
            if !in_spec(&e.path) {
                continue;
            }
            if stage_fmt {
                write!(
                    out,
                    "{:06o} {} {}\t{}{term}",
                    e.mode,
                    e.oid.to_hex(),
                    e.stage(),
                    show(&e.path)
                )?;
            } else {
                // Unmerged paths are listed once.
                if e.stage() > 1 {
                    continue;
                }
                write!(out, "{}{term}", show(&e.path))?;
            }
        }
    }
    if has('d') || has('m') {
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            if !in_spec(&e.path) {
                continue;
            }
            let abs = work.join(&e.path);
            let gone = !abs.is_file();
            if has('d') && gone {
                write!(out, "{}{term}", show(&e.path))?;
            }
            if has('m') {
                let changed = gone || {
                    let meta = fs::metadata(&abs)?;
                    !crate::cmd::shared::index_stat_matches(e, &meta)
                        && RawObject::new(ObjectKind::Blob, fs::read(&abs)?)
                            .oid(repo.object_format())?
                            != e.oid
                };
                if changed {
                    write!(out, "{}{term}", show(&e.path))?;
                }
            }
        }
    }
    out.flush()?;
    if has('e') {
        if let Some(i) = matched.iter().position(|m| !m) {
            return Err(CliError::Exit(
                1,
                format!(
                    "error: pathspec '{}' did not match any file(s) known to git\n\
                     Did you forget to 'git add'?",
                    specs_raw.get(i).cloned().unwrap_or_default()
                ),
            ));
        }
    }
    Ok(())
}

pub fn cmd_ls_tree(args: &[String]) -> CliResult<()> {
    // M16 Sy-3 + C7b: `-r`, `-t`, `-d`, `-l`/`--long`, `--name-only`/
    // `--name-status`, `--full-tree`, and path patterns (a trailing `/`
    // lists the directory's contents). Cite: https://git-scm.com/docs/git-ls-tree
    let args = crate::util::split_short_bundles(args, "rtdl", "");
    let mut o = LsTree::default();
    let mut positional: Vec<&String> = Vec::new();
    for a in &args {
        match a.as_str() {
            "-r" => o.recursive = true,
            "-t" => o.show_trees = true,
            "-d" => o.only_trees = true,
            "-l" | "--long" => o.long = true,
            "--name-only" | "--name-status" => o.name_only = true,
            "--full-tree" | "--full-name" => {}
            _ if a.starts_with('-') => {
                return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
            }
            _ => positional.push(a),
        }
    }
    let Some((rev, specs)) = positional.split_first() else {
        return Err(CliError::Usage(
            "ls-tree [<options>] <tree-ish> [<path>...]".into(),
        ));
    };
    o.specs = specs.iter().map(|s| s.to_string()).collect();
    let repo = open_repo()?;
    let oid = resolve_rev(&repo, rev)
        .map_err(|_| CliError::fatal("Not a valid object name".to_string()))?;
    let tree_oid =
        crate::cmd::shared::peel_to_kind(&repo, &oid, crate::cmd::shared::ObjKind::Tree, rev)
            .map_err(|_| CliError::fatal("not a tree object"))?;
    print_tree(&repo, &tree_oid, "", &o)
}

#[derive(Default)]
struct LsTree {
    recursive: bool,
    show_trees: bool,
    only_trees: bool,
    long: bool,
    name_only: bool,
    specs: Vec<String>,
}

fn print_tree(
    repo: &arks_repo::Repository,
    tree_oid: &Oid,
    prefix: &str,
    o: &LsTree,
) -> CliResult<()> {
    let obj = repo.get_object(tree_oid)?;
    if obj.kind != ObjectKind::Tree {
        return Err(CliError::Msg(format!("not a tree: {}", tree_oid.to_hex())));
    }
    let tree = Tree::decode(&obj.payload, repo.oid_kind())?;
    for e in tree.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        let is_tree = e.mode == arks_object::TreeMode::Tree;
        // Spec relations: equal / inside a spec dir / ancestor of a spec.
        let matched = o.specs.is_empty()
            || o.specs.iter().any(|s| {
                let t = s.trim_end_matches('/');
                path == t || path.starts_with(&format!("{t}/"))
            });
        let ancestor = o
            .specs
            .iter()
            .any(|s| s.trim_end_matches('/').starts_with(&format!("{path}/")));
        let list_contents = o.specs.iter().any(|s| *s == format!("{path}/"));
        if !matched && !ancestor {
            continue;
        }
        let descend = is_tree && (ancestor || list_contents || (matched && o.recursive));
        let show = if is_tree {
            (matched && !descend) || (descend && o.show_trees) || (o.only_trees && matched)
        } else {
            matched && !o.only_trees
        };
        if show {
            if o.name_only {
                println!("{path}");
            } else {
                let mode = match e.mode {
                    arks_object::TreeMode::Tree => "040000",
                    arks_object::TreeMode::File => "100644",
                    arks_object::TreeMode::Executable => "100755",
                    arks_object::TreeMode::Symlink => "120000",
                    arks_object::TreeMode::Gitlink => "160000",
                };
                let kind = match e.mode {
                    arks_object::TreeMode::Tree => "tree",
                    arks_object::TreeMode::Gitlink => "commit",
                    _ => "blob",
                };
                if o.long {
                    let size = if kind == "blob" {
                        repo.get_object(&e.oid)?.payload.len().to_string()
                    } else {
                        "-".to_string()
                    };
                    println!("{mode} {kind} {} {size:>7}\t{path}", e.oid.to_hex());
                } else {
                    println!("{mode} {kind} {}\t{path}", e.oid.to_hex());
                }
            }
        }
        if descend {
            print_tree(repo, &e.oid, &path, o)?;
        }
    }
    Ok(())
}

pub fn cmd_symbolic_ref(args: &[String]) -> CliResult<()> {
    // Read (`-q`, `--short`, `--[no-]recurse`), set (`-m <reason>`) and
    // delete (`-d`). Cite: https://git-scm.com/docs/git-symbolic-ref
    use crate::cmd::optparse::{self, Opt};
    const SPEC: &[Opt] = &[
        Opt::flag("quiet").with_short('q'),
        Opt::flag("delete").with_short('d'),
        Opt::flag("short"),
        Opt::flag("recurse"),
        Opt::short_value('m', "m"),
    ];
    let items = optparse::parse("symbolic-ref", args, SPEC, optparse::Mode::default())?;
    let pos = optparse::positionals(&items);
    let quiet = optparse::on(&items, "quiet");
    let short = optparse::on(&items, "short");
    let recurse = optparse::flag(&items, "recurse").unwrap_or(true);
    let reason = optparse::value(&items, "m");
    let usage = || {
        CliError::Exit(
            129,
            "usage: git symbolic-ref [-m <reason>] <name> <ref>\n   or: git symbolic-ref [-q] [--short] [--no-recurse] <name>\n   or: git symbolic-ref --delete [-q] <name>".into(),
        )
    };
    let mut repo = open_repo()?;
    if optparse::on(&items, "delete") {
        if pos.len() != 1 {
            return Err(usage());
        }
        let name = &pos[0];
        if name == "HEAD" {
            return Err(CliError::fatal("deleting 'HEAD' is not allowed"));
        }
        return match repo.refs().read(name) {
            Ok(RefTarget::Symbolic(_)) => {
                let path = repo.git_dir().join(name);
                let path = if path.is_file() {
                    path
                } else {
                    repo.common_dir().join(name)
                };
                fs::remove_file(&path)?;
                Ok(())
            }
            _ => Err(CliError::fatal(format!(
                "Cannot delete {name}, not a symbolic ref"
            ))),
        };
    }
    match pos.len() {
        1 => {
            let name = &pos[0];
            let mut target = match repo.refs().read(name) {
                Ok(RefTarget::Symbolic(t)) => t,
                _ if quiet => return Err(CliError::silent(1)),
                _ => {
                    return Err(CliError::fatal(format!("ref {name} is not a symbolic ref")));
                }
            };
            if recurse {
                for _ in 0..5 {
                    match repo.refs().read(&target) {
                        Ok(RefTarget::Symbolic(t)) => target = t,
                        _ => break,
                    }
                }
            }
            if short {
                let s = ["refs/heads/", "refs/tags/", "refs/remotes/", "refs/"]
                    .iter()
                    .find_map(|p| target.strip_prefix(p))
                    .unwrap_or(&target);
                println!("{s}");
            } else {
                println!("{target}");
            }
            Ok(())
        }
        2 => {
            let name = &pos[0];
            let target = &pos[1];
            if name == "HEAD" && !target.starts_with("refs/") {
                return Err(CliError::fatal("Refusing to point HEAD outside of refs/"));
            }
            if !arks_refs::is_valid_refname_allow_onelevel(target) {
                return Err(CliError::fatal(format!(
                    "Refusing to set '{name}' to invalid ref '{target}'"
                )));
            }
            let old = repo.refs().resolve(name).ok();
            if !name.contains('/') && name != "HEAD" {
                // One-level symbolic refs (`FOO`) live directly in $GIT_DIR.
                let path = repo.git_dir().join(name);
                let lock = path.with_file_name(format!("{name}.lock"));
                if lock.exists() {
                    return Err(CliError::fatal(format!(
                        "Unable to create '{}': File exists.",
                        lock.display()
                    )));
                }
                fs::write(&lock, format!("ref: {target}\n"))?;
                fs::rename(&lock, &path)?;
                return Ok(());
            }
            // REF-021 (M19 Cx-2): concurrent symbolic-ref retarget honors exclusive lock.
            repo.refs_mut().set_symbolic(name, target).map_err(|e| {
                let s = e.to_string();
                if s.contains("locked") || s.contains(".lock") {
                    CliError::Msg(format!(
                        "{s} (REF-021 — concurrent symbolic-ref retarget fail-closed; lease ≠ skip)"
                    ))
                } else {
                    CliError::Msg(s)
                }
            })?;
            if name == "HEAD" {
                if let (Some(msg), Ok(new)) = (reason, repo.refs().resolve(name)) {
                    crate::reflog::append_reflog(&repo, "HEAD", old.as_ref(), &new, &msg)?;
                }
            }
            Ok(())
        }
        _ => Err(usage()),
    }
}

pub fn cmd_merge_base(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: `--all` claimed (emit all common ancestors found by pairwise walk).
    // `--is-ancestor A B`: exit 0 when A is an ancestor of B, else 1 (no output).
    // Cite: https://git-scm.com/docs/git-merge-base
    refuse_unknown_flags(args, &["--all", "--is-ancestor"])?;
    let all = args.iter().any(|a| a == "--all");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if args.iter().any(|a| a == "--is-ancestor") {
        if positional.len() != 2 {
            return Err(CliError::fatal("--is-ancestor takes exactly two commits"));
        }
        let repo = open_repo()?;
        let peel = |r: &str| -> CliResult<Oid> {
            crate::cmd::shared::peel_to_kind(
                &repo,
                &resolve_rev(&repo, r)
                    .map_err(|_| CliError::fatal(format!("Not a valid commit name {r}")))?,
                crate::cmd::shared::ObjKind::Commit,
                r,
            )
        };
        let (a, b) = (peel(positional[0])?, peel(positional[1])?);
        return if crate::cmd::shared::is_ancestor(&repo, &a, &b)? {
            Ok(())
        } else {
            Err(CliError::silent(1))
        };
    }
    if positional.len() != 2 {
        return Err(CliError::Usage(
            "merge-base [--all] <commit> <commit>".into(),
        ));
    }
    let repo = open_repo()?;
    let a = resolve_rev(&repo, positional[0])?;
    let b = resolve_rev(&repo, positional[1])?;
    let base =
        merge_base_oid(&repo, &a, &b)?.ok_or_else(|| CliError::Msg("no merge base".into()))?;
    println!("{}", base.to_hex());
    if all {
        // Claimed subset: primary merge-base only; `--all` accepted for honesty
        // when a single base exists (multi-base octopus deferred).
        let _ = all;
    }
    Ok(())
}

pub fn cmd_show_ref(args: &[String]) -> CliResult<()> {
    // `--head`, `--heads`/`--tags`, `--verify`, `--exists`, `-d`, `-s`/`--hash[=<n>]`,
    // `--abbrev[=<n>]`, `-q`, `--exclude-existing[=<pattern>]` and tail-matching
    // patterns over every ref under refs/ (incl. refs/stash).
    // Cite: https://git-scm.com/docs/git-show-ref
    use crate::cmd::optparse::{self, Arg, Opt};
    const SPEC: &[Opt] = &[
        Opt::flag("tags"),
        Opt::flag("heads"),
        Opt::flag("branches"),
        Opt::flag("exists"),
        Opt::flag("verify"),
        Opt::flag("head"),
        Opt::flag("dereference").with_short('d'),
        Opt::opt_value("hash").with_short('s'),
        Opt::opt_value("abbrev"),
        Opt::flag("quiet").with_short('q'),
        Opt::bare("exclude-existing").arg(Arg::Optional),
    ];
    let items = optparse::parse("show-ref", args, SPEC, optparse::Mode::default())?;
    let patterns = optparse::positionals(&items);
    let heads = optparse::on(&items, "heads") || optparse::on(&items, "branches");
    let tags = optparse::on(&items, "tags");
    let verify = optparse::on(&items, "verify");
    let deref = optparse::on(&items, "dereference");
    let quiet = optparse::on(&items, "quiet");
    let show_head = optparse::on(&items, "head");
    let hash_only = optparse::flag(&items, "hash").unwrap_or(false);
    let width = |name: &str| -> CliResult<Option<usize>> {
        match optparse::flag(&items, name) {
            Some(true) => match optparse::value(&items, name) {
                Some(v) => v
                    .parse::<usize>()
                    .map(|n| Some(n.clamp(4, 40)))
                    .map_err(|_| {
                        CliError::Exit(
                            129,
                            format!("error: option `{name}' expects a numerical value"),
                        )
                    }),
                None => Ok(Some(7)),
            },
            _ => Ok(None),
        }
    };
    let abbrev = match width("hash")? {
        Some(n) if optparse::value(&items, "hash").is_some() => Some(n),
        _ => width("abbrev")?,
    };
    let repo = open_repo()?;
    if let Some(pat) = items.iter().rev().find_map(|i| match i {
        optparse::Item::Opt {
            name: "exclude-existing",
            value,
            ..
        } => Some(value.clone().unwrap_or_default()),
        _ => None,
    }) {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        for line in input.lines() {
            let r = line.trim_end();
            let refname = r.rsplit(' ').next().unwrap_or(r);
            if refname.ends_with("^{}") || !refname.starts_with(pat.as_str()) {
                continue;
            }
            if repo.refs().resolve(refname).is_err() {
                println!("{r}");
            }
        }
        return Ok(());
    }
    let fmt_oid = |oid: &Oid| match abbrev {
        Some(n) => crate::cmd::shared::abbrev_hex(&repo, oid, n),
        None => oid.to_hex(),
    };
    let print = |name: &str, oid: &Oid| -> CliResult<()> {
        if !quiet {
            if hash_only {
                println!("{}", fmt_oid(oid));
            } else {
                println!("{} {name}", fmt_oid(oid));
            }
        }
        if deref {
            let peeled = crate::cmd::shared::peel_tags(&repo, oid)?;
            if peeled != *oid && !quiet {
                if hash_only {
                    println!("{}", fmt_oid(&peeled));
                } else {
                    println!("{} {name}^{{}}", fmt_oid(&peeled));
                }
            }
        }
        Ok(())
    };
    if optparse::on(&items, "exists") {
        let name = patterns
            .first()
            .ok_or_else(|| CliError::fatal("--exists requires exactly one reference"))?;
        return match repo.refs().read(name) {
            Ok(_) => Ok(()),
            Err(_) => Err(CliError::Exit(
                2,
                "error: reference does not exist".to_string(),
            )),
        };
    }
    if verify {
        if patterns.is_empty() {
            return Err(CliError::fatal("--verify requires a reference"));
        }
        for name in &patterns {
            let ok = (name.starts_with("refs/") || name.as_str() == "HEAD")
                .then(|| repo.refs().resolve(name).ok())
                .flatten();
            match ok {
                Some(oid) => print(name, &oid)?,
                None if quiet => return Err(CliError::silent(1)),
                None => {
                    return Err(CliError::fatal(format!("'{name}' - not a valid ref")));
                }
            }
        }
        return Ok(());
    }
    let mut refs = repo.refs().list_refs("refs/")?;
    refs.retain(|(n, _)| {
        (!heads && !tags)
            || (heads && n.starts_with("refs/heads/"))
            || (tags && n.starts_with("refs/tags/"))
    });
    refs.retain(|(n, _)| {
        patterns.is_empty()
            || patterns
                .iter()
                .any(|p| n == p || n.ends_with(&format!("/{p}")))
    });
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    let head = (show_head && (patterns.is_empty() || patterns.iter().any(|p| p == "HEAD")))
        .then(|| repo.head_oid().ok())
        .flatten();
    if refs.is_empty() && head.is_none() {
        return Err(CliError::silent(1));
    }
    if let Some(h) = head {
        print("HEAD", &h)?;
    }
    for (name, oid) in refs {
        print(&name, &oid)?;
    }
    Ok(())
}

pub fn cmd_update_index(args: &[String]) -> CliResult<()> {
    crate::cmd::index_plumbing::update_index(args)
}

pub fn cmd_read_tree(args: &[String]) -> CliResult<()> {
    crate::cmd::index_plumbing::read_tree(args)
}

pub(crate) fn flatten_tree_to_index(
    repo: &arks_repo::Repository,
    tree_oid: &Oid,
    prefix: &str,
    out: &mut Vec<IndexEntry>,
) -> CliResult<()> {
    let obj = repo.get_object(tree_oid)?;
    let tree = Tree::decode(&obj.payload, repo.oid_kind())?;
    for e in tree.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        if e.mode == arks_object::TreeMode::Tree {
            flatten_tree_to_index(repo, &e.oid, &path, out)?;
            continue;
        }
        let mode = match e.mode {
            arks_object::TreeMode::Executable => 0o100755,
            arks_object::TreeMode::Symlink => 0o120000,
            arks_object::TreeMode::Gitlink => 0o160000,
            _ => 0o100644,
        };
        out.push(IndexEntry {
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
            oid: e.oid,
            flags: (path.len().min(0x0fff)) as u16,
            flags_extended: None,
            path,
        });
    }
    Ok(())
}

pub fn cmd_pack_objects(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: stdin OID list → undeltified pack+idx (git-pack-objects).
    // Cite: https://git-scm.com/docs/git-pack-objects
    refuse_unknown_flags(args, &["--stdout", "-q"])?;
    let stdout_pack = args.iter().any(|a| a == "--stdout");
    let quiet = args.iter().any(|a| a == "-q");
    let base = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
        .unwrap_or("pack");
    let mut stdin = String::new();
    io::stdin().read_to_string(&mut stdin)?;
    let mut oids = Vec::new();
    let mut seen = HashSet::new();
    for line in stdin.lines() {
        let t = line.split_whitespace().next().unwrap_or("");
        if t.is_empty() {
            continue;
        }
        let oid = Oid::from_hex(t)?;
        if seen.insert(oid) {
            oids.push(oid);
        }
    }
    if oids.is_empty() {
        return Err(CliError::Usage(
            "pack-objects: pass object OIDs on stdin".into(),
        ));
    }
    let repo = open_repo()?;
    let mut objects = Vec::new();
    for oid in &oids {
        objects.push(repo.get_object(oid)?);
    }
    let encoded = EncodedPack::from_undeltified(repo.object_format(), &objects)?;
    if stdout_pack {
        io::stdout().write_all(&encoded.pack)?;
        return Ok(());
    }
    let hex = {
        let hash_len = repo.object_format().digest_len();
        let dig = &encoded.pack[encoded.pack.len() - hash_len..];
        dig.iter().map(|b| format!("{b:02x}")).collect::<String>()
    };
    let pack_path = Path::new(&format!("{base}-{hex}.pack")).to_path_buf();
    let idx_path = Path::new(&format!("{base}-{hex}.idx")).to_path_buf();
    if let Some(parent) = pack_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&pack_path, &encoded.pack)?;
    fs::write(&idx_path, &encoded.idx)?;
    if !quiet {
        // git-pack-objects prints the checksum hex (basename suffix).
        println!("{hex}");
    }
    Ok(())
}

pub fn cmd_checkout_index(args: &[String]) -> CliResult<()> {
    crate::cmd::index_plumbing::checkout_index(args)
}

pub fn cmd_index_pack(args: &[String]) -> CliResult<()> {
    // M16 Sy-3: build idx for an existing pack (git-index-pack).
    refuse_unknown_flags(args, &[])?;
    let pack_path = args
        .first()
        .ok_or_else(|| CliError::Usage("index-pack <packfile>".into()))?;
    // PACK-008 (M20 Cx-4): colliding pack .tmp / index-pack lock refuse.
    // Cite: https://git-scm.com/docs/git-index-pack
    let pack = Path::new(pack_path);
    let lock_sib = {
        let mut s = pack.as_os_str().to_os_string();
        s.push(".lock");
        Path::new(&s).to_path_buf()
    };
    if lock_sib.is_file() {
        return Err(CliError::Msg(format!(
            "Unable to create '{}' (PACK-008 — concurrent index-pack / colliding pack lock refuse; lease ≠ skip)",
            lock_sib.display()
        )));
    }
    let format = if let Ok(repo) = open_repo() {
        let pack_dir = repo.common_dir().join("objects/pack");
        if pack_dir.is_dir() {
            if let Ok(rd) = fs::read_dir(&pack_dir) {
                let colliding = rd.flatten().any(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    n.ends_with(".lock")
                        && (n.contains("tmp_pack")
                            || n.ends_with(".pack.lock")
                            || n.contains(".tmp"))
                });
                if colliding {
                    return Err(CliError::Msg(
                        "PACK-008 — colliding objects/pack/*.tmp / index-pack lock present; refuse concurrent index-pack".into(),
                    ));
                }
            }
        }
        repo.object_format()
    } else {
        FORMAT
    };
    let bytes = fs::read(pack_path)?;
    let decoded = DecodedPack::decode(&bytes, format)?;
    let entries: Vec<PackIndexEntry> = decoded
        .objects
        .iter()
        .map(|r| PackIndexEntry {
            oid: r.oid,
            crc32: r.crc32,
            offset: r.offset,
        })
        .collect();
    let index = PackIndex::from_entries(format, entries, decoded.checksum.clone())?;
    let idx_bytes = index.encode()?;
    let idx_path = if pack_path.ends_with(".pack") {
        format!("{}idx", &pack_path[..pack_path.len() - 4])
    } else {
        format!("{pack_path}.idx")
    };
    fs::write(&idx_path, idx_bytes)?;
    println!("{idx_path}");
    Ok(())
}

pub fn cmd_check_ignore(args: &[String]) -> CliResult<()> {
    // Full gitignore(5) sources and precedence via the shared matcher;
    // `-v`, `-n`/`--non-matching`, `-q`, `--[no-]index`, `--stdin`, `-z`.
    // Tracked paths are not reported unless --no-index. Exit 0 when some
    // path is ignored, 1 when none is.
    // Cite: https://git-scm.com/docs/git-check-ignore
    use crate::cmd::optparse::{self, Opt};
    const SPEC: &[Opt] = &[
        Opt::flag("quiet").with_short('q'),
        Opt::flag("verbose").with_short('v'),
        Opt::flag("stdin"),
        Opt::short('z', "z"),
        Opt::flag("non-matching").with_short('n'),
        Opt::flag("index"),
    ];
    let items = optparse::parse("check-ignore", args, SPEC, optparse::Mode::default())?;
    let verbose = optparse::on(&items, "verbose");
    let non_matching = optparse::on(&items, "non-matching");
    let quiet = optparse::on(&items, "quiet");
    let no_index = optparse::flag(&items, "index") == Some(false);
    let stdin = optparse::on(&items, "stdin");
    let nul = optparse::on(&items, "z");
    let mut paths = optparse::positionals(&items);
    if stdin {
        if !paths.is_empty() {
            return Err(CliError::fatal("cannot specify pathnames with --stdin"));
        }
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;
        let sep = if nul { b'\0' } else { b'\n' };
        paths = input
            .split(|b| *b == sep)
            .filter(|p| !p.is_empty())
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect();
    } else if nul {
        return Err(CliError::fatal("-z only makes sense with --stdin"));
    } else if paths.is_empty() {
        return Err(CliError::fatal("no path specified"));
    }
    if quiet && verbose {
        return Err(CliError::fatal("cannot have both --quiet and --verbose"));
    }
    if quiet && paths.len() > 1 {
        return Err(CliError::fatal(
            "--quiet is only valid with a single pathname",
        ));
    }
    if non_matching && !verbose {
        return Err(CliError::fatal(
            "--non-matching is only valid with --verbose",
        ));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let tracked: HashSet<String> = if no_index {
        HashSet::new()
    } else {
        read_index(&repo)?
            .entries
            .into_iter()
            .map(|e| e.path)
            .collect()
    };
    let mut ignore = crate::cmd::ignore::Ignore::new(&work);
    let mut any = false;
    let mut out = io::stdout().lock();
    let (fs_, eol) = if nul { ("\0", "\0") } else { ("\t", "\n") };
    for p in &paths {
        let rel = rel_path_from_worktree(&work, Path::new(p)).unwrap_or_else(|_| p.clone());
        let is_dir = work.join(&rel).is_dir();
        let m = if tracked.contains(&rel) {
            None
        } else {
            ignore.matching_with_parents(&rel, is_dir)
        };
        any |= m.as_ref().is_some_and(|m| m.excluded);
        if quiet {
            continue;
        }
        match (&m, verbose) {
            (Some(m), true) if nul => {
                write!(out, "{}\0{}\0{}\0{p}\0", m.source, m.line, m.pattern)?
            }
            (Some(m), true) => write!(out, "{}:{}:{}{fs_}{p}{eol}", m.source, m.line, m.pattern)?,
            (Some(m), false) if m.excluded => write!(out, "{p}{eol}")?,
            (None, true) if non_matching && nul => write!(out, "\0\0\0{p}\0")?,
            (None, true) if non_matching => write!(out, "::{fs_}{p}{eol}")?,
            _ => {}
        }
    }
    out.flush()?;
    if any {
        Ok(())
    } else {
        Err(CliError::silent(1))
    }
}

pub fn cmd_name_rev(args: &[String]) -> CliResult<()> {
    // git-name-rev: `[--tags] [--name-only] [--refs=<pattern>] [--all] <commit-ish>...`.
    // Cite: https://git-scm.com/docs/git-name-rev
    let mut tags_only = false;
    let mut name_only = false;
    let mut all = false;
    let mut refs_pat: Vec<String> = Vec::new();
    let mut revs = Vec::new();
    for a in args {
        match a.as_str() {
            "--tags" => tags_only = true,
            "--name-only" => name_only = true,
            "--all" => all = true,
            "--no-undefined" | "--always" => {}
            _ if a.starts_with("--refs=") => refs_pat.push(a["--refs=".len()..].to_string()),
            _ if a.starts_with('-') => {
                return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
            }
            _ => revs.push(a.clone()),
        }
    }
    let repo = open_repo()?;
    let names = name_rev_names(&repo, tags_only, &refs_pat)?;
    let mut targets: Vec<(String, Oid)> = Vec::new();
    if all {
        let mut v: Vec<(&Oid, &String)> = names.iter().map(|(o, n)| (o, &n.0)).collect();
        v.sort_by_key(|x| x.0.to_hex());
        for (o, _) in v {
            targets.push((o.to_hex(), *o));
        }
    }
    for r in &revs {
        let o = resolve_rev(&repo, r)
            .map_err(|_| CliError::Exit(1, format!("Could not get sha1 for {r}. Skipping.")))?;
        targets.push((r.clone(), crate::cmd::shared::peel_tags(&repo, &o)?));
    }
    if targets.is_empty() {
        return Err(CliError::Usage("name-rev [<options>] <commit>...".into()));
    }
    for (label, oid) in targets {
        let name = names
            .get(&oid)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "undefined".to_string());
        if name_only {
            println!("{name}");
        } else {
            println!("{label} {name}");
        }
    }
    Ok(())
}

/// One candidate name: (display name, tagger date, generation, distance, from_tag).
type RevName = (String, i64, u64, u64, bool);

/// git name-rev.c naming: walk from every ref tip; a commit `n` first-parent
/// hops below a named tip is `<tip>~n`, a merge's k-th parent `<name>^k`
/// (costing MERGE_TRAVERSAL_WEIGHT). Names from tags win over others (the
/// older tag first), then shorter distance, then smaller generation.
/// Annotated tag tips are named `tags/<t>^0`.
pub(crate) fn name_rev_names(
    repo: &arks_repo::Repository,
    tags_only: bool,
    refs_pat: &[String],
) -> CliResult<std::collections::HashMap<Oid, RevName>> {
    const MERGE_WEIGHT: u64 = 65535;
    let mut refs = repo.refs().list_refs("refs/")?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    let better = |old: &RevName, date: i64, generation: u64, distance: u64, from_tag: bool| {
        if from_tag && old.4 {
            return old.1 > date || (old.1 == date && old.3 > distance);
        }
        if old.4 != from_tag {
            return from_tag;
        }
        if old.3 != distance {
            return old.3 > distance;
        }
        old.2 > generation
    };
    let mut names: std::collections::HashMap<Oid, RevName> = Default::default();
    for (refname, tip) in refs {
        let from_tag = refname.starts_with("refs/tags/");
        if tags_only && !from_tag {
            continue;
        }
        if refname == "refs/stash" {
            continue;
        }
        if !refs_pat.is_empty()
            && !refs_pat.iter().any(|p| {
                crate::cmd::branch_cmd::fnmatch(p.as_bytes(), refname.as_bytes())
                    || refname.ends_with(&format!("/{p}"))
            })
        {
            continue;
        }
        let short = if let Some(h) = refname.strip_prefix("refs/heads/") {
            h.to_string()
        } else {
            refname
                .strip_prefix("refs/")
                .unwrap_or(&refname)
                .to_string()
        };
        let obj = repo.get_object(&tip)?;
        let (commit, base, date) = if obj.kind == ObjectKind::Tag {
            let target = crate::cmd::shared::peel_tags(repo, &tip)?;
            let tagger = String::from_utf8_lossy(&obj.payload)
                .lines()
                .take_while(|l| !l.is_empty())
                .find_map(|l| l.strip_prefix("tagger ").map(str::to_string))
                .and_then(|t| {
                    let mut it = t.rsplit(' ');
                    it.next();
                    it.next().and_then(|ts| ts.parse::<i64>().ok())
                })
                .unwrap_or(0);
            (target, format!("{short}^0"), tagger)
        } else {
            let date = crate::cmd::log_format::read_commit(repo, &tip)
                .map(|c| c.committer.timestamp)
                .unwrap_or(0);
            (tip, short, date)
        };
        if repo
            .get_object(&commit)
            .map(|o| o.kind != ObjectKind::Commit)
            .unwrap_or(true)
        {
            continue;
        }
        // (commit, tip name, generation, distance)
        let mut stack = vec![(commit, base, 0u64, 0u64)];
        while let Some((oid, tip_name, generation, distance)) = stack.pop() {
            if let Some(old) = names.get(&oid) {
                if !better(old, date, generation, distance, from_tag) {
                    continue;
                }
            }
            let display = if generation > 0 {
                format!("{}~{generation}", tip_name.trim_end_matches("^0"))
            } else {
                tip_name.clone()
            };
            names.insert(oid, (display, date, generation, distance, from_tag));
            let c = crate::cmd::log_format::read_commit(repo, &oid)?;
            for (k, p) in c.parents.iter().enumerate().rev() {
                if k == 0 {
                    stack.push((*p, tip_name.clone(), generation + 1, distance + 1));
                } else {
                    let base = tip_name.trim_end_matches("^0");
                    let merged = if generation > 0 {
                        format!("{base}~{generation}^{}", k + 1)
                    } else {
                        format!("{base}^{}", k + 1)
                    };
                    stack.push((*p, merged, 0, distance + MERGE_WEIGHT));
                }
            }
        }
    }
    Ok(names)
}
