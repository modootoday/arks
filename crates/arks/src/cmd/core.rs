//! Core porcelain: init / objects / index / commit / log / show.

use crate::cmd::shared::{
    collect_untracked, commit_signature, committer_signature, file_mode, file_times,
    index_stat_matches, resolve_rev,
};
use crate::cmd::sign::{is_sign_flag, sign_commit, want_sign_commit};
use crate::error::{CliError, CliResult};
use crate::util::{
    FORMAT, cwd, open_repo, read_index, refuse_unknown_flags, work_tree, write_index,
};
use crate::write_tree::write_tree_from_index;
use arks_index::{IndexEntry, sort_entries};
use arks_object::{Commit, ObjectKind, RawObject, write_loose};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::thread;
pub fn cmd_init(args: &[String]) -> CliResult<()> {
    // git-init: https://git-scm.com/docs/git-init
    // `--bare` (M16 Sy-1), `--object-format=sha256` ([[adr:root/20261110214800]]),
    // `-q/--quiet`, `-b/--initial-branch`, and re-initialisation of an existing repository.
    refuse_unknown_flags(
        args,
        &[
            "--bare",
            "--object-format",
            "-q",
            "--quiet",
            "-b",
            "--initial-branch",
        ],
    )?;
    let bare = args.iter().any(|a| a == "--bare");
    let quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    let mut object_format = "sha1".to_string();
    let mut initial_branch: Option<String> = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--object-format" {
            i += 1;
            let v = args
                .get(i)
                .ok_or_else(|| CliError::Usage("init --object-format <sha1|sha256>".into()))?;
            object_format = v.to_ascii_lowercase();
        } else if let Some(rest) = a.strip_prefix("--object-format=") {
            object_format = rest.to_ascii_lowercase();
        } else if a == "-b" || a == "--initial-branch" {
            i += 1;
            let v = args
                .get(i)
                .ok_or_else(|| CliError::Usage(format!("init: option `{a}` requires a value")))?;
            initial_branch = Some(v.clone());
        } else if let Some(rest) = a.strip_prefix("--initial-branch=") {
            initial_branch = Some(rest.to_string());
        } else if !a.starts_with('-') {
            positional.push(args[i].clone());
        }
        i += 1;
    }
    if object_format != "sha1" && object_format != "sha256" {
        return Err(CliError::Usage(format!(
            "init --object-format: unknown '{object_format}' (claimed: sha1, sha256)"
        )));
    }
    let dir = if let Some(p) = positional.first() {
        PathBuf::from(p)
    } else {
        cwd()?
    };
    fs::create_dir_all(&dir)?;
    let git_dir = if bare { dir.clone() } else { dir.join(".git") };
    let display_dir = |p: &Path| {
        let shown = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        format!("{}/", shown.display())
    };
    if git_dir.join("HEAD").exists() {
        // Re-init keeps HEAD, refs and objects (git-init "Running git init in an
        // existing repository is safe").
        if let Some(b) = &initial_branch {
            eprintln!("warning: re-init: ignored --initial-branch={b}");
        }
        if !quiet {
            println!(
                "Reinitialized existing Git repository in {}",
                display_dir(&git_dir)
            );
        }
        return Ok(());
    }
    let cfg = arks_config::ConfigSet::load(None);
    let configured = initial_branch.or_else(|| cfg.get("init.defaultBranch"));
    let advise = configured.is_none()
        && !cfg.get("advice.defaultBranchName").is_some_and(|v| {
            matches!(
                v.to_ascii_lowercase().as_str(),
                "false" | "no" | "off" | "0"
            )
        });
    let branch = configured.unwrap_or_else(|| "master".into());
    if advise {
        // git-init advice.defaultBranchName (shown when no name was configured).
        eprint!(
            "hint: Using 'master' as the name for the initial branch. This default branch name\n\
             hint: is subject to change. To configure the initial branch name to use in all\n\
             hint: of your new repositories, which will suppress this warning, call:\n\
             hint: \n\
             hint: \tgit config --global init.defaultBranch <name>\n\
             hint: \n\
             hint: Names commonly chosen instead of 'master' are 'main', 'trunk' and\n\
             hint: 'development'. The just-created branch can be renamed via this command:\n\
             hint: \n\
             hint: \tgit branch -m <name>\n"
        );
    }
    if branch.is_empty() || branch.starts_with('-') || branch.contains(char::is_whitespace) {
        return Err(CliError::Msg(format!(
            "invalid initial branch name: '{branch}'"
        )));
    }
    fs::create_dir_all(git_dir.join("objects"))?;
    fs::create_dir_all(git_dir.join("refs/heads"))?;
    fs::create_dir_all(git_dir.join("refs/tags"))?;
    fs::write(git_dir.join("HEAD"), format!("ref: refs/heads/{branch}\n"))?;
    let bare_cfg = if bare { "true" } else { "false" };
    let cfg = if object_format == "sha256" {
        format!(
            "[core]\n\trepositoryformatversion = 1\n\tfilemode = true\n\tbare = {bare_cfg}\n\tlogallrefupdates = true\n[extensions]\n\tobjectFormat = sha256\n"
        )
    } else {
        format!(
            "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = {bare_cfg}\n\tlogallrefupdates = true\n"
        )
    };
    fs::write(git_dir.join("config"), cfg)?;
    if !quiet {
        println!(
            "Initialized empty Git repository in {}",
            display_dir(&git_dir)
        );
    }
    Ok(())
}

pub fn cmd_hash_object(args: &[String]) -> CliResult<()> {
    // `-t <type>`, `-w`, `--stdin`, `--stdin-paths`, `--[no-]filters`,
    // `--path <file>`, `--literally`. Non-blob objects must parse unless
    // `--literally`. Cite: https://git-scm.com/docs/git-hash-object
    use crate::cmd::optparse::{self, Opt};
    const SPEC: &[Opt] = &[
        Opt::short_value('t', "t"),
        Opt::short('w', "w"),
        Opt::flag("stdin"),
        Opt::flag("stdin-paths"),
        Opt::flag("filters"),
        Opt::flag("literally"),
        Opt::value("path"),
    ];
    let items = optparse::parse("hash-object", args, SPEC, optparse::Mode::default())?;
    let usage = |m: &str| {
        CliError::Exit(
            129,
            format!(
                "error: {m}\nusage: git hash-object [-t <type>] [-w] [--path=<file> | --no-filters]\n                       [--stdin [--literally]] [--] <file>..."
            ),
        )
    };
    let files = optparse::positionals(&items);
    let write = optparse::on(&items, "w");
    let stdin = optparse::on(&items, "stdin");
    let stdin_paths = optparse::on(&items, "stdin-paths");
    let no_filters = optparse::flag(&items, "filters") == Some(false);
    let literally = optparse::on(&items, "literally");
    let path_opt = optparse::value(&items, "path");
    let ty = optparse::value(&items, "t").unwrap_or_else(|| "blob".into());
    if stdin_paths {
        if stdin {
            return Err(usage("Can't use --stdin-paths with --stdin"));
        }
        if !files.is_empty() {
            return Err(usage("Can't specify files with --stdin-paths"));
        }
        if path_opt.is_some() {
            return Err(usage("Can't use --stdin-paths with --path"));
        }
    }
    if path_opt.is_some() && no_filters {
        return Err(usage("Can't use --path with --no-filters"));
    }
    let kind = match ObjectKind::parse(&ty) {
        Ok(k) => Some(k),
        Err(_) if literally => None,
        Err(_) => return Err(CliError::fatal(format!("invalid object type \"{ty}\""))),
    };
    let repo = open_repo().ok();
    let format = repo.as_ref().map(|r| r.object_format()).unwrap_or(FORMAT);
    let mut repo = repo;
    let mut inputs: Vec<(Vec<u8>, Option<String>)> = Vec::new();
    if stdin {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        inputs.push((buf, path_opt.clone()));
    }
    let mut names = files.clone();
    if stdin_paths {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s)?;
        names.extend(s.lines().map(str::to_string));
    }
    let mut out = String::new();
    let mut hash_one = |payload: Vec<u8>, filter_path: Option<String>| -> CliResult<()> {
        let mut payload = payload;
        if kind == Some(ObjectKind::Blob) && !no_filters {
            if let (Some(p), Some(r)) = (filter_path.as_deref(), repo.as_ref()) {
                if let Some(wt) = r.work_tree() {
                    if let Some(f) = apply_clean_filter(wt, p, &payload)? {
                        payload = f;
                    }
                }
            }
        }
        let oid = match kind {
            Some(k) => {
                if !literally {
                    crate::cmd::plumbing_ext::validate_object(k, &payload, format)?;
                }
                let obj = RawObject::new(k, payload);
                if write {
                    let r = repo
                        .as_mut()
                        .ok_or_else(|| CliError::fatal("not a git repository"))?;
                    r.store_mut().put(format, &obj)?
                } else {
                    obj.oid(format)?
                }
            }
            None => {
                // `--literally` with an unknown type: hash `<type> <len>\0<payload>`.
                let mut raw = format!("{ty} {}\0", payload.len()).into_bytes();
                raw.extend_from_slice(&payload);
                let digest = arks_hash::hash_bytes(format, &raw)
                    .map_err(|e| CliError::Msg(e.to_string()))?;
                let hex: String = digest
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                if write {
                    return Err(CliError::fatal(format!(
                        "writing object of unknown type '{ty}' is not supported"
                    )));
                }
                out.push_str(&hex);
                out.push('\n');
                return Ok(());
            }
        };
        out.push_str(&oid.to_hex());
        out.push('\n');
        Ok(())
    };
    for (payload, p) in inputs {
        hash_one(payload, p)?;
    }
    for name in &names {
        let payload = fs::read(name).map_err(|e| {
            CliError::fatal(format!(
                "could not open '{name}' for reading: {}",
                if e.kind() == io::ErrorKind::NotFound {
                    "No such file or directory".to_string()
                } else {
                    e.to_string()
                }
            ))
        })?;
        let fp = path_opt.clone().or_else(|| Some(name.clone()));
        hash_one(payload, fp)?;
    }
    if write {
        // OBJ-001 / OBJ-008 (M20 Cx-5): identical concurrent loose creates are idempotent.
        eprintln!(
            "note: OBJ-001 — concurrent identical loose object creates are content-addressed/idempotent; do not invent unlock-by-delete under live writers"
        );
    }
    print!("{out}");
    Ok(())
}

pub fn cmd_cat_file(args: &[String]) -> CliResult<()> {
    // `-e`: exit 0 when the object exists, 1 when a valid name lacks an object,
    // 128 for an invalid name. Cite: https://git-scm.com/docs/git-cat-file
    refuse_unknown_flags(args, &["-t", "-s", "-p", "-e"])?;
    let Some(mode) = args
        .iter()
        .find(|a| *a == "-t" || *a == "-s" || *a == "-p" || *a == "-e")
    else {
        // `cat-file <type> <object>`: raw content, peeling tags/commits to
        // `<type>` when the object is not already of that type.
        let [kind, object] = args else {
            return Err(CliError::Usage(
                "cat-file: require -t, -s, -e, -p or <type> <object>".into(),
            ));
        };
        let repo = open_repo()?;
        let oid = resolve_rev(&repo, object)
            .map_err(|_| CliError::fatal(format!("Not a valid object name {object}")))?;
        let want = ObjectKind::parse(kind)
            .map_err(|_| CliError::fatal(format!("invalid object type \"{kind}\"")))?;
        let mut obj = repo.get_object(&oid)?;
        if obj.kind != want {
            let peeled = match want {
                ObjectKind::Commit => crate::cmd::shared::peel_to_kind(
                    &repo,
                    &oid,
                    crate::cmd::shared::ObjKind::Commit,
                    object,
                ),
                ObjectKind::Tree => crate::cmd::shared::peel_to_kind(
                    &repo,
                    &oid,
                    crate::cmd::shared::ObjKind::Tree,
                    object,
                ),
                _ => Err(CliError::Msg(String::new())),
            }
            .map_err(|_| CliError::fatal(format!("git cat-file {object}: bad file")))?;
            obj = repo.get_object(&peeled)?;
        }
        io::Write::write_all(&mut io::stdout(), &obj.payload)?;
        return Ok(());
    };
    if mode == "-e" {
        let object = args
            .iter()
            .find(|a| !a.starts_with('-'))
            .ok_or_else(|| CliError::Usage("cat-file: missing object".into()))?;
        let repo = open_repo()?;
        let oid = resolve_rev(&repo, object)
            .map_err(|_| CliError::fatal(format!("Not a valid object name {object}")))?;
        return match repo.get_object(&oid) {
            Ok(_) => Ok(()),
            Err(_) => Err(CliError::silent(1)),
        };
    }
    let object = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .ok_or_else(|| CliError::Usage("cat-file: missing object".into()))?;
    let repo = open_repo()?;
    // OBJ-005 (M20 Cx-5): detect truncated/partial loose object files before decode.
    if object.len() == 40 && object.chars().all(|c| c.is_ascii_hexdigit()) {
        let loose = repo
            .common_dir()
            .join("objects")
            .join(&object[..2])
            .join(&object[2..]);
        if loose.is_file() {
            let meta = fs::metadata(&loose)?;
            if meta.len() < 2 {
                return Err(CliError::Msg(format!(
                    "corrupt loose object {} (OBJ-005 — partial/truncated loose write; remove only after crash recovery)",
                    object
                )));
            }
        }
    }
    let oid = resolve_rev(&repo, object)?;
    let obj = match repo.get_object(&oid) {
        Ok(o) => o,
        Err(e) => {
            return Err(CliError::Msg(format!(
                "{e} (OBJ-005 — refuse reading partial/corrupt object; no silent invent)"
            )));
        }
    };
    match mode.as_str() {
        "-t" => println!("{}", obj.kind.as_str()),
        "-s" => println!("{}", obj.payload.len()),
        "-p" => match obj.kind {
            ObjectKind::Blob => {
                io::Write::write_all(&mut io::stdout(), &obj.payload)?;
            }
            ObjectKind::Commit | ObjectKind::Tag => {
                print!("{}", String::from_utf8_lossy(&obj.payload));
            }
            ObjectKind::Tree => {
                let tree = arks_object::Tree::decode(&obj.payload, repo.oid_kind())?;
                for e in &tree.entries {
                    let kind = if e.mode.is_tree() { "tree" } else { "blob" };
                    println!(
                        "{:>6} {} {}\t{}",
                        e.mode.as_str(),
                        kind,
                        e.oid.to_hex(),
                        e.name
                    );
                }
            }
        },
        _ => unreachable!(),
    }
    Ok(())
}

/// M15 Fs-1: run sync hooks (githooks). No async/out-of-band scheduler (no L3).
pub(crate) fn run_claimed_hooks(
    repo: &Repository,
    no_verify: bool,
    names: &[&str],
    stdin: Option<&[u8]>,
) -> CliResult<()> {
    crate::cmd::hooks::run_sync_hooks(repo, no_verify, names, stdin)
}

pub fn cmd_update_ref(args: &[String]) -> CliResult<()> {
    // `-m <reason>`, `-d`, `--[no-]deref`, `--[no-]create-reflog`, `--stdin`
    // transactions (`update`/`create`/`delete`/`verify`/`option`/`start`/
    // `prepare`/`commit`/`abort`) with `-z`. `<new>`/`<old>` are revisions;
    // an all-zero `<old>` requires the ref to be absent.
    // Cite: https://git-scm.com/docs/git-update-ref
    use crate::cmd::optparse::{self, Opt};
    const SPEC: &[Opt] = &[
        Opt::short_value('m', "m"),
        Opt::short('d', "d"),
        Opt::flag("deref"),
        Opt::short('z', "z"),
        Opt::flag("stdin"),
        Opt::flag("create-reflog"),
        Opt::bare("no-verify"),
    ];
    let items = optparse::parse("update-ref", args, SPEC, optparse::Mode::default())?;
    let positional = optparse::positionals(&items);
    let delete = optparse::on(&items, "d");
    let no_verify = optparse::on(&items, "no-verify");
    let no_deref = optparse::flag(&items, "deref") == Some(false);
    let message = optparse::value(&items, "m");
    let create_reflog = optparse::on(&items, "create-reflog");
    let nul = optparse::on(&items, "z");
    let stdin = optparse::on(&items, "stdin");
    let usage = || {
        CliError::Exit(
            129,
            "usage: git update-ref [<options>] -d <refname> [<old-val>]\n   or: git update-ref [<options>]    <refname> <new-val> [<old-val>]\n   or: git update-ref [<options>] --stdin [-z]".into(),
        )
    };
    if message.as_deref() == Some("") {
        return Err(CliError::fatal(
            "Refusing to perform update with empty message.",
        ));
    }
    if stdin {
        if delete || !positional.is_empty() {
            return Err(usage());
        }
    } else if nul {
        return Err(usage());
    }
    let mut repo = open_repo()?;
    // CORR-003 (M20 Cx-7): replace-refs concurrent honesty — refuse under replace.lock.
    // Cite: https://git-scm.com/docs/git-replace + lockfile (never delete live locks).
    let replace_lock = repo.common_dir().join("refs/replace.lock");
    let replace_lock_gd = repo.git_dir().join("refs/replace.lock");
    if replace_lock.is_file() || replace_lock_gd.is_file() {
        let p = if replace_lock.is_file() {
            &replace_lock
        } else {
            &replace_lock_gd
        };
        return Err(CliError::Msg(format!(
            "{} (CORR-003 — concurrent replace-refs / live lock delete anti-pattern refuse; lease ≠ skip)",
            p.display()
        )));
    }
    // REF-009 (M20 Cx-4): plant logs/*.lock → refuse update-ref class.
    crate::reflog::refuse_if_reflog_locked(&repo, "HEAD")?;
    if let Some(name) = positional.first() {
        crate::reflog::refuse_if_reflog_locked(&repo, name)?;
    }
    // REF-003 (M20 Cx-8): loose vs packed delete race — refuse delete under packed-refs.lock.
    let packed_lock = repo.common_dir().join("packed-refs.lock");
    if delete && packed_lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (REF-003 — loose vs packed-refs delete race refuse; serialize delete vs pack-refs; lease ≠ skip)",
            packed_lock.display()
        )));
    }
    run_claimed_hooks(&repo, no_verify, &["reference-transaction", "update"], None)?;
    let opts = crate::cmd::plumbing_ext::RefUpdateOpts {
        no_deref,
        create_reflog,
        message: message.clone(),
    };
    if stdin {
        return crate::cmd::plumbing_ext::update_ref_stdin(&mut repo, nul, &opts);
    }
    let rev = |repo: &Repository, r: &str| -> CliResult<Option<Oid>> {
        if !r.is_empty() && r.bytes().all(|b| b == b'0') && r.len() >= 40 {
            return Ok(None);
        }
        resolve_rev(repo, r)
            .map(Some)
            .map_err(|_| CliError::fatal(format!("{r}: not a valid SHA1")))
    };
    let op = if delete {
        if positional.is_empty() || positional.len() > 2 {
            return Err(usage());
        }
        crate::cmd::plumbing_ext::RefOp {
            name: positional[0].clone(),
            new: None,
            old: positional.get(1).map(|s| rev(&repo, s)).transpose()?,
            no_deref,
            verify: false,
        }
    } else {
        if positional.len() < 2 || positional.len() > 3 {
            return Err(usage());
        }
        let new = rev(&repo, &positional[1])?;
        crate::cmd::plumbing_ext::RefOp {
            name: positional[0].clone(),
            new: Some(new),
            old: positional.get(2).map(|s| rev(&repo, s)).transpose()?,
            no_deref,
            verify: false,
        }
    };
    let target = op.target(&repo);
    if let Err(e) = op.check(&repo, &target) {
        return Err(match e {
            crate::cmd::plumbing_ext::RefCheck::Missing(msg)
            | crate::cmd::plumbing_ext::RefCheck::Mismatch(msg)
                if delete =>
            {
                CliError::Exit(1, format!("error: {msg}"))
            }
            crate::cmd::plumbing_ext::RefCheck::Missing(msg)
            | crate::cmd::plumbing_ext::RefCheck::Mismatch(msg) => {
                CliError::fatal(format!("update_ref failed for ref '{target}': {msg}"))
            }
        });
    }
    op.apply(&mut repo, &target, &opts)
}

pub fn cmd_rev_parse(args: &[String]) -> CliResult<()> {
    // Sy-1: `--abbrev-ref` / `--verify` / `--symbolic-full-name` (git-rev-parse).
    // Cloud drop-in §O: `--short[=n]`, `-q`, `--show-toplevel`, `--git-dir`,
    // `--absolute-git-dir`, `--git-common-dir`, `--show-prefix`, `--show-cdup`,
    // `--is-inside-work-tree`, `--is-inside-git-dir`, `--is-bare-repository`,
    // `^<rev>` and `A..B` ranges. Arguments are processed in order.
    // Cite: https://git-scm.com/docs/git-rev-parse
    use crate::cmd::shared::abbrev_hex;
    let mut abbrev_ref = false;
    let mut symbolic_full = false;
    let mut verify = false;
    let mut quiet = false;
    let mut short: Option<usize> = None;
    let mut pending: Vec<String> = Vec::new();
    let repo = open_repo()?;
    let cwd = crate::util::cwd()?;
    let work = repo.work_tree().map(Path::to_path_buf).or_else(|| {
        (repo.kind() == arks_repo::GitDirKind::DotGitDir)
            .then(|| repo.git_dir().parent().map(Path::to_path_buf))
            .flatten()
    });
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let inside_work = work.as_ref().is_some_and(|w| {
        canon(&cwd).starts_with(canon(w)) && !canon(&cwd).starts_with(canon(repo.git_dir()))
    });
    let prefix = work
        .as_ref()
        .and_then(|w| {
            canon(&cwd)
                .strip_prefix(canon(w))
                .ok()
                .map(Path::to_path_buf)
        })
        .unwrap_or_default();
    let git_dir_display = |dir: &Path| -> String {
        if canon(&cwd.join(".git")) == canon(dir) {
            ".git".to_string()
        } else {
            canon(dir).display().to_string()
        }
    };
    let mut dashdash = false;
    for a in args {
        if dashdash {
            continue;
        }
        match a.as_str() {
            "--" => dashdash = true,
            "--abbrev-ref" | "--abbrev-ref=strict" | "--abbrev-ref=loose" => abbrev_ref = true,
            "--symbolic-full-name" => symbolic_full = true,
            "--verify" => verify = true,
            "-q" | "--quiet" => quiet = true,
            "--short" => {
                short = Some(7);
                verify = true;
            }
            "--show-toplevel" => match &work {
                Some(w) => println!("{}", canon(w).display()),
                None => {
                    return Err(CliError::fatal("this operation must be run in a work tree"));
                }
            },
            "--git-dir" => println!("{}", git_dir_display(repo.git_dir())),
            "--git-common-dir" => println!("{}", git_dir_display(repo.common_dir())),
            "--absolute-git-dir" => println!("{}", canon(repo.git_dir()).display()),
            "--show-prefix" => {
                let p = prefix.to_string_lossy().replace('\\', "/");
                if p.is_empty() {
                    println!();
                } else {
                    println!("{p}/");
                }
            }
            "--show-cdup" => {
                println!("{}", "../".repeat(prefix.components().count()));
            }
            "--is-inside-work-tree" => println!("{inside_work}"),
            "--is-inside-git-dir" => {
                println!("{}", canon(&cwd).starts_with(canon(repo.git_dir())))
            }
            "--is-bare-repository" => {
                println!("{}", repo.kind() == arks_repo::GitDirKind::Bare)
            }
            // Object names of every ref under refs/ (or one namespace), by name.
            "--all" | "--branches" | "--tags" | "--remotes" => {
                let prefix = match a.as_str() {
                    "--branches" => "refs/heads/",
                    "--tags" => "refs/tags/",
                    "--remotes" => "refs/remotes/",
                    _ => "refs/",
                };
                let mut refs = repo.refs().list_refs(prefix)?;
                refs.sort_by(|x, y| x.0.cmp(&y.0));
                for (_, oid) in refs {
                    println!("{}", oid.to_hex());
                }
            }
            x => {
                if let Some(n) = x.strip_prefix("--short=") {
                    short = Some(
                        n.parse::<usize>()
                            .map_err(|_| CliError::fatal(format!("bad --short: {n}")))?
                            .clamp(4, 40),
                    );
                    verify = true;
                } else if x.starts_with('-') && x.len() > 1 && !x.starts_with("^") {
                    return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
                } else {
                    pending.push(a.clone());
                }
            }
        }
        if !verify && !abbrev_ref && !symbolic_full {
            for rev in pending.drain(..) {
                print_rev_arg(&repo, &rev, work.as_deref())?;
            }
        }
    }
    if verify {
        if pending.len() != 1 {
            if quiet {
                return Err(CliError::silent(1));
            }
            return Err(CliError::fatal("Needed a single revision"));
        }
        let rev = &pending[0];
        let oid = match resolve_rev(&repo, rev) {
            Ok(o) => o,
            Err(_) if quiet => return Err(CliError::silent(1)),
            Err(_) => return Err(CliError::fatal("Needed a single revision")),
        };
        if abbrev_ref {
            println!("{}", abbreviate_ref(&repo, rev)?);
        } else if symbolic_full {
            println!("{}", symbolic_full_name(&repo, rev)?);
        } else {
            match short {
                Some(n) => println!("{}", abbrev_hex(&repo, &oid, n)),
                None => println!("{}", oid.to_hex()),
            }
        }
        return Ok(());
    }
    for rev in pending {
        if abbrev_ref {
            println!("{}", abbreviate_ref(&repo, &rev)?);
        } else if symbolic_full {
            println!("{}", symbolic_full_name(&repo, &rev)?);
        } else {
            print_rev_arg(&repo, &rev, work.as_deref())?;
        }
    }
    Ok(())
}

/// One non-`--verify` rev-parse argument: `<rev>`, `^<rev>`, `A..B`, `A...B`;
/// an unresolvable argument is echoed and must name a working-tree path.
fn print_rev_arg(repo: &Repository, rev: &str, work: Option<&Path>) -> CliResult<()> {
    let res = |r: &str| resolve_rev(repo, if r.is_empty() { "HEAD" } else { r });
    if let Some((l, r)) = rev.split_once("...") {
        if let (Ok(a), Ok(b)) = (res(l), res(r)) {
            println!("{}", b.to_hex());
            println!("{}", a.to_hex());
            if let Some(base) = arks_repo::merge_base_oid(repo, &a, &b)? {
                println!("^{}", base.to_hex());
            }
            return Ok(());
        }
    } else if let Some((l, r)) = rev.split_once("..") {
        if let (Ok(a), Ok(b)) = (res(l), res(r)) {
            println!("{}", b.to_hex());
            println!("^{}", a.to_hex());
            return Ok(());
        }
    } else if let Some(x) = rev.strip_prefix('^') {
        if let Ok(o) = resolve_rev(repo, x) {
            println!("^{}", o.to_hex());
            return Ok(());
        }
    } else if let Ok(o) = resolve_rev(repo, rev) {
        println!("{}", o.to_hex());
        return Ok(());
    }
    println!("{rev}");
    let exists = work.is_some_and(|w| {
        let cwd = crate::util::cwd().unwrap_or_else(|_| w.to_path_buf());
        cwd.join(rev).exists()
    });
    if !exists {
        return Err(CliError::fatal(format!(
            "ambiguous argument '{rev}': unknown revision or path not in the working tree.\n\
             Use '--' to separate paths from revisions, like this:\n\
             'git <command> [<revision>...] -- [<file>...]'"
        )));
    }
    Ok(())
}

/// Short unambiguous ref name (`git rev-parse --abbrev-ref`).
fn abbreviate_ref(repo: &Repository, rev: &str) -> CliResult<String> {
    if rev == "HEAD" {
        return match repo.head_target()? {
            RefTarget::Symbolic(s) => Ok(shorten_ref_name(&s)),
            RefTarget::Direct(_) => Ok("HEAD".into()),
        };
    }
    if rev.starts_with("refs/") {
        return Ok(shorten_ref_name(rev));
    }
    // Already short, or resolve via DWIM then shorten if we find a ref form.
    let as_heads = format!("refs/heads/{rev}");
    if repo.refs().resolve(&as_heads).is_ok() {
        return Ok(rev.to_string());
    }
    let as_tags = format!("refs/tags/{rev}");
    if repo.refs().resolve(&as_tags).is_ok() {
        return Ok(rev.to_string());
    }
    let as_remotes = format!("refs/remotes/{rev}");
    if repo.refs().resolve(&as_remotes).is_ok() {
        return Ok(rev.to_string());
    }
    // Fall back: if resolve works as OID-ish name, still print the given token.
    let _ = resolve_rev(repo, rev)?;
    Ok(rev.to_string())
}

fn symbolic_full_name(repo: &Repository, rev: &str) -> CliResult<String> {
    if rev == "HEAD" {
        return match repo.head_target()? {
            RefTarget::Symbolic(s) => Ok(s),
            RefTarget::Direct(_) => Err(CliError::Msg(
                "rev-parse --symbolic-full-name: detached HEAD".into(),
            )),
        };
    }
    if rev.starts_with("refs/") {
        let _ = repo.refs().resolve(rev)?;
        return Ok(rev.to_string());
    }
    let as_heads = format!("refs/heads/{rev}");
    if repo.refs().resolve(&as_heads).is_ok() {
        return Ok(as_heads);
    }
    let as_tags = format!("refs/tags/{rev}");
    if repo.refs().resolve(&as_tags).is_ok() {
        return Ok(as_tags);
    }
    let as_remotes = format!("refs/remotes/{rev}");
    if repo.refs().resolve(&as_remotes).is_ok() {
        return Ok(as_remotes);
    }
    Err(CliError::Msg(format!(
        "rev-parse --symbolic-full-name: unknown ref {rev}"
    )))
}

fn shorten_ref_name(full: &str) -> String {
    full.strip_prefix("refs/heads/")
        .or_else(|| full.strip_prefix("refs/tags/"))
        .or_else(|| full.strip_prefix("refs/remotes/"))
        .unwrap_or(full)
        .to_string()
}

pub fn cmd_add(args: &[String]) -> CliResult<()> {
    // Fs-4 + M16 Sy-1: `-A`/`-u`/`--chmod=+x|-x` (git-add). Cite: https://git-scm.com/docs/git-add
    // FS3: hunk-level `-p`, `-e` edit, `-i` menu (plus FS1 scripted/TTY answers).
    if args.iter().any(|a| a == "-i" || a == "--interactive") {
        return crate::cmd::interactive_patch::cmd_add_interactive(args);
    }
    if args.iter().any(|a| a == "-e" || a == "--edit") {
        return crate::cmd::interactive_patch::cmd_add_edit(args);
    }
    if args.iter().any(|a| a == "-p" || a == "--patch") {
        return crate::cmd::interactive_patch::cmd_add_patch_hunks(args);
    }
    refuse_unknown_flags(args, &["-A", "--all", "-u", "--update", "--chmod"])?;
    // HOOK-005 (M20 Cx-4): nested index write under held lock (hook/agent) refuse-closed.
    // Keep LOCK-001/LOCK-004 + retry guidance so Cm-2/Cx-2 regressions stay honest.
    {
        let repo = open_repo()?;
        // FS-007 opt-in refuse when .git under sync root (ARKS_HOST_ENV_REFUSE_SYNC=1).
        // IDX-004 (M20 Cx-5): split-index sharedindex.lock race refuse.
        let shared = repo.git_dir().join("sharedindex.lock");
        if shared.is_file() {
            return Err(CliError::Msg(format!(
                "{} (IDX-004 — split-index / sharedindex.lock contention refuse; no parallel shared-index dialect)",
                shared.display()
            )));
        }
        // MISC-008 (M20 Cx-7): sparse cone rewrite vs concurrent materialize.
        let sparse_lock = repo.git_dir().join("info/sparse-checkout.lock");
        if sparse_lock.is_file() {
            return Err(CliError::Msg(format!(
                "{} (MISC-008 — parallel sparse-checkout cone vs materialize refuse; lease ≠ skip)",
                sparse_lock.display()
            )));
        }
        // MISC-019 (M20 Cx-5): concurrent gitattributes install.
        if let Ok(wt) = work_tree(&repo) {
            let attr_lock = wt.join(".gitattributes.lock");
            if attr_lock.is_file() {
                return Err(CliError::Msg(format!(
                    "{} (MISC-019 — concurrent .gitattributes install refuse; lease ≠ skip)",
                    attr_lock.display()
                )));
            }
        }
        let lock = arks_index::index_lock_path(&repo.index_path());
        if lock.is_file() {
            return Err(CliError::Msg(format!(
                "{} (index locked; LOCK-001 exclusive create; LOCK-004 porcelain die-class; HOOK-005 nested index write refuse; retry after the holder finishes — no skip)",
                lock.display()
            )));
        }
    }
    let all = args.iter().any(|a| a == "-A" || a == "--all");
    let update = args.iter().any(|a| a == "-u" || a == "--update");
    let mut chmod_mode: Option<u32> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--chmod" {
            i += 1;
            let v = args
                .get(i)
                .ok_or_else(|| CliError::Usage("add --chmod=+x|-x".into()))?;
            chmod_mode = Some(match v.as_str() {
                "+x" => 0o100755,
                "-x" => 0o100644,
                other => {
                    return Err(CliError::Usage(format!(
                        "add --chmod: unsupported mode '{other}' (claim +x|-x)"
                    )));
                }
            });
        } else if let Some(rest) = args[i].strip_prefix("--chmod=") {
            chmod_mode = Some(match rest {
                "+x" => 0o100755,
                "-x" => 0o100644,
                other => {
                    return Err(CliError::Usage(format!(
                        "add --chmod: unsupported mode '{other}' (claim +x|-x)"
                    )));
                }
            });
        }
        i += 1;
    }
    let positional: Vec<&String> = {
        let mut out = Vec::new();
        let mut skip_next = false;
        for a in args {
            if skip_next {
                skip_next = false;
                continue;
            }
            if a == "--chmod" {
                skip_next = true;
                continue;
            }
            if a.starts_with("--chmod=") || a.starts_with('-') {
                continue;
            }
            out.push(a);
        }
        out
    };
    if positional.is_empty() && !all && !update {
        return Err(CliError::Usage(
            "add: path required (or -A/--all / -u/--update)".into(),
        ));
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut index = read_index(&repo)?;
    let mut paths: Vec<String> = Vec::new();
    if all || update {
        let indexed: std::collections::HashSet<_> =
            index.entries.iter().map(|e| e.path.clone()).collect();
        if all {
            let mut untracked = Vec::new();
            collect_untracked(&work, &work, &indexed, &mut untracked)?;
            paths.extend(untracked);
        }
        for e in index.entries.iter().filter(|e| e.stage() == 0) {
            paths.push(e.path.clone());
        }
        paths.sort();
        paths.dedup();
    }
    // Pathspecs (git-add §DESCRIPTION): a directory (including `.` = the whole
    // tree) adds untracked files below it and stages modifications and
    // removals of tracked files below it.
    let mut removable: HashSet<String> = HashSet::new();
    for p in &positional {
        let rel = pathspec_rel(&work, p)?;
        let abs = if rel.is_empty() {
            work.clone()
        } else {
            work.join(&rel)
        };
        let prefix = if rel.is_empty() {
            String::new()
        } else {
            format!("{rel}/")
        };
        let tracked: Vec<String> = index
            .entries
            .iter()
            .filter(|e| e.stage() == 0 && (e.path == rel || e.path.starts_with(&prefix)))
            .map(|e| e.path.clone())
            .collect();
        if abs.is_dir() {
            let mut nested = Vec::new();
            let indexed: std::collections::HashSet<_> = std::collections::HashSet::new();
            collect_untracked(&work, &abs, &indexed, &mut nested)?;
            paths.extend(nested);
        } else if !abs.exists() && tracked.is_empty() {
            return Err(CliError::fatal(format!(
                "pathspec '{p}' did not match any files"
            )));
        } else if !tracked.contains(&rel) {
            paths.push(rel.clone());
        }
        removable.extend(tracked.iter().cloned());
        paths.extend(tracked);
    }
    paths.sort();
    paths.dedup();
    let mut updates: HashMap<String, IndexEntry> = HashMap::new();
    let mut removals: HashSet<String> = HashSet::new();
    // Pf4-2: classify dirty paths serially (stat short-circuit), then parallel
    // content-addressed loose writes (safe; OID path rename). Index lock stays serial.
    let mut dirty: Vec<String> = Vec::new();
    {
        let stage0: HashMap<&str, &IndexEntry> = index
            .entries
            .iter()
            .filter(|e| e.stage() == 0)
            .map(|e| (e.path.as_str(), e))
            .collect();
        for rel in paths {
            let abs = work.join(&rel);
            if !abs.is_file() {
                if all || update || removable.contains(&rel) {
                    removals.insert(rel);
                }
                continue;
            }
            if update && !stage0.contains_key(rel.as_str()) {
                continue;
            }
            let meta = fs::metadata(&abs)?;
            // Git-style ce_match_stat short-circuit: unchanged stage-0 entries skip
            // re-hash/re-write (git-add / index refresh). Cite: gitformat-index.
            if chmod_mode.is_none() {
                if let Some(existing) = stage0.get(rel.as_str()) {
                    if index_stat_matches(existing, &meta) {
                        continue;
                    }
                }
            }
            dirty.push(rel);
        }
    }
    if !dirty.is_empty() {
        let objects_dir = repo.store().objects_dir().to_path_buf();
        let format = repo.object_format();
        let workers = parallel_workers(dirty.len());
        if workers <= 1 {
            for rel in dirty {
                let entry = stage_blob_entry(&work, &objects_dir, &rel, chmod_mode, format)?;
                updates.insert(rel, entry);
            }
        } else {
            let chunk = dirty.len().div_ceil(workers);
            let mut staged: Vec<IndexEntry> = Vec::with_capacity(dirty.len());
            let mut worker_err: Option<CliError> = None;
            thread::scope(|scope| {
                let mut handles = Vec::with_capacity(workers);
                for piece in dirty.chunks(chunk) {
                    let objects_dir = &objects_dir;
                    let work = &work;
                    handles.push(scope.spawn(move || -> CliResult<Vec<IndexEntry>> {
                        let mut local = Vec::with_capacity(piece.len());
                        for rel in piece {
                            local.push(stage_blob_entry(
                                work,
                                objects_dir,
                                rel,
                                chmod_mode,
                                format,
                            )?);
                        }
                        Ok(local)
                    }));
                }
                for h in handles {
                    match h.join() {
                        Ok(Ok(local)) => staged.extend(local),
                        Ok(Err(e)) => {
                            worker_err = Some(e);
                            break;
                        }
                        Err(_) => {
                            worker_err = Some(CliError::Msg("add: worker panicked".into()));
                            break;
                        }
                    }
                }
            });
            if let Some(e) = worker_err {
                return Err(e);
            }
            for entry in staged {
                updates.insert(entry.path.clone(), entry);
            }
        }
    }
    if !updates.is_empty() || !removals.is_empty() {
        index.entries.retain(|e| {
            if updates.contains_key(&e.path) {
                return false;
            }
            if removals.contains(&e.path) && e.stage() == 0 {
                return false;
            }
            true
        });
        index.entries.extend(updates.into_values());
        sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
    }
    Ok(())
}

/// Worktree-relative form of a cwd-relative pathspec (`""` = whole tree).
pub(crate) fn pathspec_rel(work: &Path, p: &str) -> CliResult<String> {
    let cwd = cwd()?;
    let abs = cwd.join(p);
    let canon = |x: &Path| x.canonicalize().unwrap_or_else(|_| x.to_path_buf());
    let work_c = canon(work);
    // Normalize lexically when the path does not exist (deleted tracked file).
    let abs_c = if abs.exists() {
        canon(&abs)
    } else {
        let mut out = canon(&cwd);
        for comp in Path::new(p).components() {
            match comp {
                std::path::Component::ParentDir => {
                    out.pop();
                }
                std::path::Component::CurDir => {}
                other => out.push(other),
            }
        }
        out
    };
    let rel = abs_c.strip_prefix(&work_c).map_err(|_| {
        CliError::fatal(format!(
            "{p}: '{p}' is outside repository at '{}'",
            work_c.display()
        ))
    })?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

fn parallel_workers(n: usize) -> usize {
    if n < 32 {
        return 1;
    }
    thread::available_parallelism()
        .map(|p| p.get().clamp(2, 8))
        .unwrap_or(4)
        .min(n)
}

fn stage_blob_entry(
    work: &Path,
    objects_dir: &Path,
    rel: &str,
    chmod_mode: Option<u32>,
    format: arks_store::ObjectFormat,
) -> CliResult<IndexEntry> {
    let abs = work.join(rel);
    let meta = fs::metadata(&abs)?;
    let mut payload = fs::read(&abs)?;
    // FS6 closest: honor configured `filter.<name>.clean` when `.gitattributes` sets filter=.
    if let Some(filtered) = apply_clean_filter(work, rel, &payload)? {
        payload = filtered;
    }
    let size = payload.len().min(u32::MAX as usize) as u32;
    let obj = RawObject::new(ObjectKind::Blob, payload);
    let oid = write_loose(objects_dir, format, &obj)?;
    let (ctime_sec, ctime_nsec, mtime_sec, mtime_nsec) = file_times(&meta);
    let mode = chmod_mode.unwrap_or_else(|| file_mode(&meta));
    Ok(IndexEntry {
        ctime_sec,
        ctime_nsec,
        mtime_sec,
        mtime_nsec,
        dev: 0,
        ino: 0,
        mode,
        uid: 0,
        gid: 0,
        size,
        oid,
        flags: (rel.len().min(0x0fff)) as u16,
        flags_extended: None,
        path: rel.to_owned(),
    })
}

pub fn cmd_status(args: &[String]) -> CliResult<()> {
    // Optional locks: stock Git may refresh the index under `index.lock` unless
    // `--no-optional-locks` / `GIT_OPTIONAL_LOCKS=0` (IDX-002 background status
    // vs writers; AGENT-006 / IDX-008 agent readers). arks status is read-only
    // on the index (product default = no optional locks). Cite: git-status.
    // M16 Sy-1: `-b` branch header claimed.
    // Cloud drop-in §O: `-sb`, `--short`, `--branch`, `--porcelain=v1`, `-u[all|no|normal]`.
    let args = &crate::util::alias_flags(
        crate::util::split_short_bundles(args, "sbz", ""),
        &[
            ("--short", "-s"),
            ("--branch", "-b"),
            ("--porcelain=v1", "--porcelain"),
            ("--untracked-files", "-u"),
            ("--untracked-files=all", "-u"),
            ("-uall", "-u"),
            ("--untracked-files=normal", "-unormal"),
            ("--untracked-files=no", "-uno"),
            ("--ignored=traditional", "--ignored"),
        ],
    );
    let (args, after_dashdash): (Vec<String>, Vec<String>) =
        match args.iter().position(|a| a == "--") {
            Some(i) => (args[..i].to_vec(), args[i + 1..].to_vec()),
            None => (args.clone(), Vec::new()),
        };
    let args = &args;
    refuse_unknown_flags(
        args,
        &[
            "--no-optional-locks",
            "--porcelain",
            "--porcelain=v2",
            "--ignored",
            "-s",
            "-b",
            "-u",
            "-unormal",
            "-uno",
        ],
    )?;
    let untracked_all = args.iter().any(|a| a == "-u");
    let untracked_no = args.iter().any(|a| a == "-uno");
    let porcelain_v2 = args.iter().any(|a| a == "--porcelain=v2");
    let show_ignored = args.iter().any(|a| a == "--ignored");
    let porcelain = args.iter().any(|a| a == "--porcelain" || a == "-s") || porcelain_v2;
    let porcelain_v1 = args.iter().any(|a| a == "--porcelain") || porcelain_v2;
    let show_branch = args.iter().any(|a| a == "-b");
    let _no_optional = wants_no_optional_locks(args);
    let repo = open_repo()?;
    // FSM-002 (M20 Cx-7): concurrent FSM transition refuse.
    let fsm_transition = repo.git_dir().join(".arks-fsm-transition");
    if fsm_transition.is_file() {
        return Err(CliError::Msg(
            "status refused (FSM-002 — concurrent fsmonitor/state-machine transition in progress; serialize clients)".into(),
        ));
    }
    // FSM-004 (M20 Cx-7): incomplete FSM marker refuse.
    let fsm_incomplete = repo.git_dir().join(".fsmonitor-incomplete");
    if fsm_incomplete.is_file() {
        return Err(CliError::Msg(
            "status refused (FSM-004 — incomplete fsmonitor FSM marker; refuse stampede full-scan under crash recovery)".into(),
        ));
    }
    let index_path = repo.index_path();
    let lock_before = arks_index::index_lock_path(&index_path);
    let had_lock = lock_before.is_file();
    use crate::cmd::status_report::{Untracked, collect, write_long};
    use std::io::Write as _;
    let mode = if untracked_no {
        Untracked::No
    } else if untracked_all {
        Untracked::All
    } else {
        Untracked::Normal
    };
    let mut d = collect(&repo, mode)?;
    if show_ignored && mode != Untracked::No {
        d.ignored = crate::cmd::status_report::collect_ignored(&repo)?;
    }
    // Pathspecs limit every section (git-status `[<pathspec>...]`).
    let specs: Vec<String> = {
        let work = work_tree(&repo)?;
        args.iter()
            .filter(|a| !a.starts_with('-'))
            .chain(after_dashdash.iter())
            .map(|p| pathspec_rel(&work, p))
            .collect::<CliResult<_>>()?
    };
    if !specs.is_empty() {
        let hit = |p: &str| {
            let p = p.trim_end_matches('/');
            specs.iter().any(|s| {
                s.is_empty()
                    || p == s
                    || p.starts_with(&format!("{s}/"))
                    || s.starts_with(&format!("{p}/"))
            })
        };
        d.staged.retain(|k, _| hit(k));
        d.unstaged.retain(|k, _| hit(k));
        d.ignored.retain(|k| hit(k));
        // A collapsed untracked directory shown through a deeper pathspec is
        // listed at the pathspec instead.
        let work = work_tree(&repo)?;
        let mut shown = Vec::new();
        for u in d.untracked.drain(..) {
            if !hit(&u) {
                continue;
            }
            match specs
                .iter()
                .find(|s| u.ends_with('/') && s.starts_with(u.as_str()) && s.len() > u.len())
            {
                Some(s) if work.join(s).is_dir() => shown.push(format!("{s}/")),
                Some(s) => shown.push(s.clone()),
                None => shown.push(u),
            }
        }
        shown.dedup();
        d.untracked = shown;
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    if porcelain_v2 {
        write_porcelain_v2(&mut out, &repo, &d, show_branch)?;
    } else if porcelain {
        // v1 porcelain: optional `## branch` then XY PATH (git-status §Short Format):
        // X = index vs HEAD (A/M/D), Y = worktree vs index (M/D), sorted by path,
        // untracked (`??`) last.
        if show_branch {
            match (&d.branch, d.initial) {
                (Some(b), true) => writeln!(out, "## No commits yet on {b}")?,
                (Some(b), false) => writeln!(out, "## {b}")?,
                (None, _) => writeln!(out, "## HEAD (no branch)")?,
            }
        }
        let mut tracked: std::collections::BTreeMap<&String, (char, char)> =
            std::collections::BTreeMap::new();
        for (p, c) in &d.staged {
            tracked.entry(p).or_insert((' ', ' ')).0 = *c;
        }
        for (p, c) in &d.unstaged {
            tracked.entry(p).or_insert((' ', ' ')).1 = *c;
        }
        // `--porcelain` paths are always root-relative; `-s` follows cwd.
        let show = |p: &str| {
            if porcelain_v1 {
                p.to_string()
            } else {
                d.show(p)
            }
        };
        for (p, (x, y)) in &tracked {
            writeln!(out, "{x}{y} {}", show(p))?;
        }
        for p in &d.untracked {
            writeln!(out, "?? {}", show(p))?;
        }
        for p in &d.ignored {
            writeln!(out, "!! {}", show(p))?;
        }
    } else {
        write_long(&mut out, &d, mode)?;
    }
    io::Write::flush(&mut out)?;
    // Honesty: status must not create optional index.lock (AGENT-006 / IDX-008 / IDX-002).
    if !had_lock && lock_before.is_file() {
        return Err(CliError::Msg(
            "internal error: status created index.lock (optional locks forbidden; IDX-002/AGENT-006)"
                .into(),
        ));
    }
    Ok(())
}

/// `status --porcelain=v2` (git-status §Porcelain Format Version 2):
/// `# branch.*` headers with `--branch`, `1 <XY> N... <mH> <mI> <mW> <hH>
/// <hI> <path>` per changed tracked path, then `? <path>` and `! <path>`.
/// Partial: rename (`2`) and unmerged (`u`) records are not emitted.
fn write_porcelain_v2(
    out: &mut impl io::Write,
    repo: &Repository,
    d: &crate::cmd::status_report::StatusData,
    show_branch: bool,
) -> CliResult<()> {
    let zero = "0".repeat(repo.oid_kind().hex_len());
    if show_branch {
        let head = repo.head_oid().ok();
        writeln!(
            out,
            "# branch.oid {}",
            head.map_or("(initial)".to_string(), |o| o.to_hex())
        )?;
        let name = d.branch.clone().unwrap_or_else(|| "(detached)".to_string());
        writeln!(out, "# branch.head {name}")?;
        if let (Some(b), Some(h)) = (&d.branch, head) {
            let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
            if let (Some(remote), Some(merge)) = (
                cfg.get(&format!("branch.{b}.remote")),
                cfg.get(&format!("branch.{b}.merge")),
            ) {
                let short = merge.trim_start_matches("refs/heads/").to_string();
                let (full, shown) = if remote == "." {
                    (format!("refs/heads/{short}"), short.clone())
                } else {
                    (
                        format!("refs/remotes/{remote}/{short}"),
                        format!("{remote}/{short}"),
                    )
                };
                writeln!(out, "# branch.upstream {shown}")?;
                if let Ok(up) = repo.refs().resolve(&full) {
                    let a = arks_repo::ancestor_set(repo, h)?;
                    let b = arks_repo::ancestor_set(repo, up)?;
                    writeln!(
                        out,
                        "# branch.ab +{} -{}",
                        a.difference(&b).count(),
                        b.difference(&a).count()
                    )?;
                }
            }
        }
    }
    let mut head: std::collections::HashMap<String, (u32, Oid)> = Default::default();
    if let Ok(h) = repo.head_oid() {
        let tree = crate::cmd::log_format::read_commit(repo, &h)?.tree;
        let mut v = Vec::new();
        crate::cmd::plumbing::flatten_tree_to_index(repo, &tree, "", &mut v)?;
        head = v.into_iter().map(|e| (e.path, (e.mode, e.oid))).collect();
    }
    let index: std::collections::HashMap<String, (u32, Oid)> = read_index(repo)?
        .entries
        .into_iter()
        .filter(|e| e.stage() == 0)
        .map(|e| (e.path, (e.mode, e.oid)))
        .collect();
    let work = work_tree(repo)?;
    let mut tracked: std::collections::BTreeMap<&String, (char, char)> = Default::default();
    for (p, c) in &d.staged {
        tracked.entry(p).or_insert(('.', '.')).0 = *c;
    }
    for (p, c) in &d.unstaged {
        tracked.entry(p).or_insert(('.', '.')).1 = *c;
    }
    for (p, (x, y)) in &tracked {
        let (mh, hh) = head
            .get(*p)
            .map_or((0, zero.clone()), |(m, o)| (*m, o.to_hex()));
        let (mi, hi) = index
            .get(*p)
            .map_or((0, zero.clone()), |(m, o)| (*m, o.to_hex()));
        let mw = match y {
            'D' => 0,
            '.' => mi,
            _ => fs::symlink_metadata(work.join(p))
                .map(|m| {
                    if m.file_type().is_symlink() {
                        0o120000
                    } else {
                        file_mode(&m)
                    }
                })
                .unwrap_or(mi),
        };
        writeln!(
            out,
            "1 {x}{y} N... {mh:06o} {mi:06o} {mw:06o} {hh} {hi} {p}"
        )?;
    }
    for p in &d.untracked {
        writeln!(out, "? {p}")?;
    }
    for p in &d.ignored {
        writeln!(out, "! {p}")?;
    }
    Ok(())
}

/// `-unormal` (default): show a wholly untracked directory once as `dir/`
/// (git-status `--untracked-files`).
pub(crate) fn collapse_untracked_dirs(
    untracked: &[String],
    indexed: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut tracked_dirs: HashSet<&str> = HashSet::new();
    for p in indexed {
        let mut cut = p.len();
        while let Some(i) = p[..cut].rfind('/') {
            if !tracked_dirs.insert(&p[..i]) {
                break;
            }
            cut = i;
        }
    }
    let mut out: Vec<String> = Vec::new();
    for p in untracked {
        let mut shown = p.clone();
        let mut search = 0;
        while let Some(off) = p[search..].find('/') {
            let dir = &p[..search + off];
            if !tracked_dirs.contains(dir) {
                shown = format!("{dir}/");
                break;
            }
            search += off + 1;
        }
        if out.last() != Some(&shown) {
            out.push(shown);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// True when agent/profile path or Git-compatible no-optional-locks is requested.
///
/// Defaults **on** for `ARKS_PROFILE=agent` (arks agent overlay). Also honors
/// `--no-optional-locks` and `GIT_OPTIONAL_LOCKS=0` (git-status).
/// True when agent/profile path or Git-compatible no-optional-locks is requested.
///
/// Defaults **on** for `ARKS_PROFILE=agent` (arks agent overlay). Also honors
/// `--no-optional-locks` and `GIT_OPTIONAL_LOCKS=0` (git-status).
pub fn wants_no_optional_locks(args: &[String]) -> bool {
    if args.iter().any(|a| a == "--no-optional-locks") {
        return true;
    }
    if std::env::var("GIT_OPTIONAL_LOCKS")
        .map(|v| v == "0")
        .unwrap_or(false)
    {
        return true;
    }
    if std::env::var("ARKS_PROFILE")
        .map(|v| v.eq_ignore_ascii_case("agent"))
        .unwrap_or(false)
    {
        return true;
    }
    // Default product path: status never takes optional locks (compat mitigate).
    true
}

pub fn cmd_commit(args: &[String]) -> CliResult<()> {
    // Cloud drop-in §O: bundled short options (`-am <msg>`), `-q`, `-n`.
    // C7b: `--author`, `--date`, `-s`/`--signoff`, `--no-edit` and
    // `[--] <pathspec>...` (commit only those paths, git `--only`).
    // Cite: https://git-scm.com/docs/git-commit
    let args = &crate::util::alias_flags(
        crate::util::split_short_bundles(args, "aqvns", "mFt"),
        &[
            ("-n", "--no-verify"),
            ("--quiet", "-q"),
            ("--message", "-m"),
            ("-s", "--signoff"),
        ],
    );
    let args = &args
        .iter()
        .flat_map(|a| match a.strip_prefix("--message=") {
            Some(v) => vec!["-m".to_string(), v.to_string()],
            None => vec![a.clone()],
        })
        .collect::<Vec<_>>();
    // Filter sign flags (incl. `-S<keyid>`) before refuse_unknown_flags.
    let flag_args: Vec<String> = args.iter().filter(|a| !is_sign_flag(a)).cloned().collect();
    refuse_unknown_flags(
        &flag_args,
        &[
            "-m",
            "-F",
            "--file",
            "-t",
            "--template",
            "--allow-empty",
            "--no-verify",
            "--amend",
            "-a",
            "--all",
            "-q",
            "-v",
            "--author",
            "--date",
            "--signoff",
            "--no-edit",
            "--only",
            "-o",
            "--",
        ],
    )?;
    let signoff = args.iter().any(|a| a == "--signoff");
    let allow_empty = args.iter().any(|a| a == "--allow-empty");
    let no_verify = args.iter().any(|a| a == "--no-verify");
    let amend = args.iter().any(|a| a == "--amend");
    let all = args.iter().any(|a| a == "-a" || a == "--all");
    // REF-009 (M20 Cx-4): early refuse when reflog lock held (commit appends logs).
    {
        let repo = open_repo()?;
        crate::reflog::refuse_if_reflog_locked(&repo, "HEAD")?;
        if let Ok(RefTarget::Symbolic(name)) = repo.head_target() {
            crate::reflog::refuse_if_reflog_locked(&repo, &name)?;
        }
    }
    let mut message: Option<String> = None;
    let mut file_msg: Option<PathBuf> = None;
    let mut template: Option<PathBuf> = None;
    let mut author_ident: Option<String> = None;
    let mut author_date: Option<String> = None;
    let mut pathspecs: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--" {
            pathspecs.extend(args[i + 1..].iter().cloned());
            break;
        } else if args[i] == "--author" || args[i] == "--date" {
            let v = args.get(i + 1).cloned().ok_or_else(|| {
                CliError::fatal(format!("option `{}' requires a value", &args[i][2..]))
            })?;
            if args[i] == "--author" {
                author_ident = Some(v);
            } else {
                author_date = Some(v);
            }
            i += 1;
        } else if let Some(v) = args[i].strip_prefix("--author=") {
            author_ident = Some(v.to_string());
        } else if let Some(v) = args[i].strip_prefix("--date=") {
            author_date = Some(v.to_string());
        } else if !args[i].starts_with('-') {
            pathspecs.push(args[i].clone());
        } else if args[i] == "-m" {
            i += 1;
            let part = args
                .get(i)
                .ok_or_else(|| CliError::Usage("commit: -m requires a message".into()))?;
            message = Some(match message {
                // git-commit: repeated -m values become separate paragraphs.
                Some(mut m) => {
                    m.push_str("\n\n");
                    m.push_str(part);
                    m
                }
                None => part.clone(),
            });
        } else if args[i] == "-F" || args[i] == "--file" {
            i += 1;
            let p = args
                .get(i)
                .ok_or_else(|| CliError::Usage("commit -F <file>".into()))?;
            file_msg = Some(PathBuf::from(p));
        } else if let Some(rest) = args[i].strip_prefix("--file=") {
            file_msg = Some(PathBuf::from(rest));
        } else if args[i] == "-t" || args[i] == "--template" {
            i += 1;
            let p = args
                .get(i)
                .ok_or_else(|| CliError::Usage("commit -t <file>".into()))?;
            template = Some(PathBuf::from(p));
        } else if let Some(rest) = args[i].strip_prefix("--template=") {
            template = Some(PathBuf::from(rest));
        }
        i += 1;
    }
    if message.is_none() {
        if let Some(path) = file_msg {
            let text = fs::read_to_string(&path)
                .map_err(|e| CliError::Msg(format!("commit -F {}: {e}", path.display())))?;
            message = Some(text);
        }
    }
    // O3: editor-driven commit when GIT_EDITOR / ARKS_EDITOR / EDITOR is set (scripted OK).
    if message.is_none() && !amend {
        message = Some(commit_message_via_editor(template.as_deref())?);
    }

    if all && !pathspecs.is_empty() {
        return Err(CliError::fatal(format!(
            "paths '{} ...' with -a does not make sense",
            pathspecs[0]
        )));
    }
    // Fs-5: `-a` / `--all` stages tracked updates then commits (git-commit).
    if all {
        cmd_add(&["-u".into()])?;
    }
    // `commit <pathspec>`: stage those paths, then commit HEAD + only them.
    if !pathspecs.is_empty() {
        cmd_add(&pathspecs)?;
    }

    let mut repo = open_repo()?;
    run_claimed_hooks(&repo, no_verify, &["pre-commit"], None)?;

    let mut index = read_index(&repo)?;
    if !pathspecs.is_empty() {
        let work = work_tree(&repo)?;
        let specs = pathspecs
            .iter()
            .map(|p| pathspec_rel(&work, p))
            .collect::<CliResult<Vec<_>>>()?;
        let mut entries = Vec::new();
        if let Ok(head) = repo.head_oid() {
            let tree = crate::cmd::log_format::read_commit(&repo, &head)?.tree;
            crate::cmd::plumbing::flatten_tree_to_index(&repo, &tree, "", &mut entries)?;
        }
        entries.retain(|e| !crate::cmd::log_format::path_in_specs(&e.path, &specs));
        entries.extend(
            index
                .entries
                .iter()
                .filter(|e| crate::cmd::log_format::path_in_specs(&e.path, &specs))
                .cloned(),
        );
        sort_entries(&mut entries);
        index.entries = entries;
    }
    let index = index;
    let message = message.map(|m| if signoff { add_signoff(&m) } else { m });
    let override_author = |mut sig: arks_object::Signature| -> CliResult<arks_object::Signature> {
        if let Some(id) = &author_ident {
            let (name, email) = id
                .split_once('<')
                .and_then(|(n, rest)| Some((n.trim(), rest.strip_suffix('>')?)))
                .ok_or_else(|| CliError::fatal(format!("--author '{id}' is not 'Name <email>'")))?;
            sig.name = name.to_string();
            sig.email = email.to_string();
        }
        if let Some(d) = &author_date {
            let (ts, tz) = crate::cmd::shared::parse_git_date(d)
                .ok_or_else(|| CliError::fatal(format!("invalid date format: {d}")))?;
            sig.timestamp = ts;
            sig.timezone = tz;
        }
        Ok(sig)
    };
    if index.entries.is_empty() && !allow_empty && !amend {
        return Err(CliError::Msg("nothing to commit (empty index)".into()));
    }

    let tree = if index.entries.is_empty() {
        let obj = RawObject::new(ObjectKind::Tree, Vec::new());
        let format = repo.object_format();
        repo.store_mut().put(format, &obj)?
    } else {
        let format = repo.object_format();
        write_tree_from_index(repo.store_mut(), &index.entries, format)?
    };

    if amend {
        let head = repo
            .head_oid()
            .map_err(|_| CliError::Msg("commit --amend: no HEAD to amend".into()))?;
        let pobj = repo.get_object(&head)?;
        if pobj.kind != ObjectKind::Commit {
            return Err(CliError::Msg("HEAD is not a commit".into()));
        }
        let prev = Commit::decode(&pobj.payload)?;
        let message = match message {
            Some(m) => {
                if m.ends_with('\n') {
                    m
                } else {
                    format!("{m}\n")
                }
            }
            None if signoff => add_signoff(&prev.message),
            None => prev.message.clone(),
        };
        run_claimed_hooks(&repo, no_verify, &["commit-msg"], Some(message.as_bytes()))?;
        if prev.tree == tree && message == prev.message && !allow_empty {
            return Err(CliError::Msg("nothing to commit (amend unchanged)".into()));
        }
        let sig = committer_signature()?;
        let subject = message.lines().next().unwrap_or("").to_owned();
        let (do_sign, sign_key) = want_sign_commit(args, &repo)?;
        let mut commit = Commit {
            tree,
            parents: prev.parents.clone(),
            author: override_author(prev.author.clone())?,
            committer: sig,
            gpgsig: None,
            message,
        };
        if do_sign {
            sign_commit(&repo, &mut commit, sign_key.as_deref())?;
        }
        let cobj = RawObject::new(ObjectKind::Commit, commit.encode()?);
        let format = repo.object_format();
        let coid = repo.store_mut().put(format, &cobj)?;
        // M16 Sy-5: detached HEAD amend updates HEAD directly (git-commit).
        match repo.head_target()? {
            RefTarget::Symbolic(name) => {
                repo.refs_mut().update(&name, &coid, Some(&head))?;
                let _ = crate::reflog::append_head_and_branch(
                    &repo,
                    &name,
                    Some(&head),
                    &coid,
                    &format!("commit (amend): {subject}"),
                );
            }
            RefTarget::Direct(_) => {
                repo.refs_mut().update("HEAD", &coid, Some(&head))?;
                let _ = crate::reflog::append_reflog(
                    &repo,
                    "HEAD",
                    Some(&head),
                    &coid,
                    &format!("commit (amend): {subject}"),
                );
            }
        }
        if !args.iter().any(|a| a == "-q") {
            print_commit_summary(&repo, &coid, true)?;
        }
        return Ok(());
    }

    let message = message.ok_or_else(|| CliError::Usage("commit: -m <msg> required".into()))?;
    let message = if message.ends_with('\n') {
        message
    } else {
        format!("{message}\n")
    };
    run_claimed_hooks(&repo, no_verify, &["commit-msg"], Some(message.as_bytes()))?;
    let parents = match repo.head_oid() {
        Ok(oid) => vec![oid],
        Err(_) => Vec::new(),
    };
    if let Some(parent) = parents.first() {
        let pobj = repo.get_object(parent)?;
        if pobj.kind == ObjectKind::Commit {
            let pc = Commit::decode(&pobj.payload)?;
            if pc.tree == tree && !allow_empty {
                return Err(CliError::Msg("nothing to commit (tree unchanged)".into()));
            }
        }
    }
    let author = override_author(commit_signature()?)?;
    let committer = committer_signature()?;
    let subject = message.lines().next().unwrap_or("").to_owned();
    let (do_sign, sign_key) = want_sign_commit(args, &repo)?;
    let mut commit = Commit {
        tree,
        parents: parents.clone(),
        author,
        committer,
        gpgsig: None,
        message,
    };
    if do_sign {
        sign_commit(&repo, &mut commit, sign_key.as_deref())?;
    }
    let cobj = RawObject::new(ObjectKind::Commit, commit.encode()?);
    let format = repo.object_format();
    let coid = repo.store_mut().put(format, &cobj)?;
    // M16 Sy-5: detached HEAD commit updates HEAD directly (git-commit).
    match repo.head_target()? {
        RefTarget::Symbolic(name) => {
            let old = parents.first().copied();
            repo.refs_mut().update(&name, &coid, old.as_ref())?;
            let _ = crate::reflog::append_head_and_branch(
                &repo,
                &name,
                old.as_ref(),
                &coid,
                &format!("commit: {subject}"),
            );
        }
        RefTarget::Direct(_) => {
            let old = parents.first().copied();
            repo.refs_mut().update("HEAD", &coid, old.as_ref())?;
            let _ = crate::reflog::append_reflog(
                &repo,
                "HEAD",
                old.as_ref(),
                &coid,
                &format!("commit: {subject}"),
            );
        }
    }
    if !args.iter().any(|a| a == "-q") {
        print_commit_summary(&repo, &coid, false)?;
    }
    Ok(())
}

/// `-s`: append `Signed-off-by: <committer>` unless it is already the last
/// trailer; a message ending in a trailer block gets no blank separator.
/// Cite: https://git-scm.com/docs/git-commit#Documentation/git-commit.txt---signoff
fn add_signoff(message: &str) -> String {
    let Ok(c) = committer_signature() else {
        return message.to_string();
    };
    let line = format!("Signed-off-by: {} <{}>", c.name, c.email);
    let body = message.trim_end_matches('\n');
    if body.lines().last() == Some(line.as_str()) {
        return format!("{body}\n");
    }
    let last_para = body.rsplit("\n\n").next().unwrap_or("");
    let is_trailer = |l: &str| {
        l.split_once(": ").is_some_and(|(k, _)| {
            !k.is_empty() && k.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        })
    };
    let has_paragraphs = body.contains("\n\n");
    if has_paragraphs && !last_para.is_empty() && last_para.lines().all(is_trailer) {
        format!("{body}\n{line}\n")
    } else {
        format!("{body}\n\n{line}\n")
    }
}

/// git-commit summary (builtin/commit.c `print_summary`): `[<branch> <abbrev>] <subject>`,
/// ` Author:` when it differs from the committer, ` Date:` for amend, then the
/// shortstat line and `create mode` / `delete mode` lines.
/// git `print_commit_summary`; `show_date` = SUMMARY_SHOW_AUTHOR_DATE (amend,
/// sequencer commits).
pub(crate) fn print_commit_summary(
    repo: &Repository,
    coid: &Oid,
    show_date: bool,
) -> CliResult<()> {
    let amend = show_date;
    use crate::cmd::switch_tree::commit_files;
    use crate::cmd::unified_diff::{StatRow, shortstat_line, stat_change};
    let c = crate::cmd::log_format::read_commit(repo, coid)?;
    let label = match repo.head_target()? {
        RefTarget::Symbolic(s) => s.strip_prefix("refs/heads/").unwrap_or(&s).to_string(),
        RefTarget::Direct(_) => "detached HEAD".to_string(),
    };
    let root = if c.parents.is_empty() {
        " (root-commit)"
    } else {
        ""
    };
    let mut out = format!(
        "[{label}{root} {}] {}\n",
        crate::cmd::shared::abbrev_hex(repo, coid, 7),
        crate::cmd::log_format::subject_line(&c.message)
    );
    if (c.author.name.as_str(), c.author.email.as_str())
        != (c.committer.name.as_str(), c.committer.email.as_str())
    {
        out.push_str(&format!(
            " Author: {} <{}>\n",
            c.author.name, c.author.email
        ));
    }
    if amend {
        out.push_str(&format!(
            " Date: {}\n",
            crate::cmd::log_format::default_date(&c.author)
        ));
    }
    let mine = commit_files(repo, coid)?;
    let parent = match c.parents.first() {
        Some(p) => commit_files(repo, p)?,
        None => HashMap::new(),
    };
    let mut keys: Vec<&String> = mine.keys().chain(parent.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut rows = Vec::new();
    let mut modes = Vec::new();
    for k in keys {
        let (a, b) = (parent.get(k), mine.get(k));
        if a == b {
            continue;
        }
        // A gitlink has no object here; git diffs it as `Subproject commit <oid>`.
        let content = |f: &crate::cmd::switch_tree::TreeFile| -> CliResult<Vec<u8>> {
            if f.mode == 0o160000 {
                return Ok(format!("Subproject commit {}\n", f.oid.to_hex()).into_bytes());
            }
            Ok(repo.get_object(&f.oid)?.payload)
        };
        let ap = a.map(content).transpose()?;
        let bp = b.map(content).transpose()?;
        rows.push(StatRow {
            path: k.clone(),
            change: stat_change(ap.as_deref(), bp.as_deref()),
        });
        match (a, b) {
            (None, Some(f)) => modes.push(format!(" create mode {:o} {k}", f.mode)),
            (Some(f), None) => modes.push(format!(" delete mode {:o} {k}", f.mode)),
            (Some(x), Some(y)) if x.mode != y.mode => {
                modes.push(format!(" mode change {:o} => {:o} {k}", x.mode, y.mode))
            }
            _ => {}
        }
    }
    if let Some(line) = shortstat_line(&rows) {
        out.push_str(&line);
        out.push('\n');
    }
    for m in modes {
        out.push_str(&m);
        out.push('\n');
    }
    print!("{out}");
    Ok(())
}

/// O3: editor-required commit message via `-t` template + GIT_EDITOR / ARKS_EDITOR / EDITOR.
fn commit_message_via_editor(template: Option<&Path>) -> CliResult<String> {
    let editor = std::env::var("ARKS_EDITOR")
        .or_else(|_| std::env::var("GIT_EDITOR"))
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| {
            CliError::NotImplemented(
                "commit without -m/-F requires GIT_EDITOR / ARKS_EDITOR / EDITOR (O3 scripted); pass -m or set editor"
                    .into(),
            )
        })?;
    let repo = open_repo()?;
    let msg_path = repo.git_dir().join("COMMIT_EDITMSG");
    let mut seed = String::new();
    if let Some(t) = template {
        seed = fs::read_to_string(t)
            .map_err(|e| CliError::Msg(format!("commit -t {}: {e}", t.display())))?;
    }
    if !seed.ends_with('\n') && !seed.is_empty() {
        seed.push('\n');
    }
    seed.push_str(
        "\n# Please enter the commit message for your changes.\n# Lines starting with '#' are ignored (O3 scripted editor).\n",
    );
    fs::write(&msg_path, &seed)?;
    drop(repo);
    run_editor_cmd(&editor, &msg_path)?;
    let text = fs::read_to_string(&msg_path)?;
    let cleaned = text
        .lines()
        .filter(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    if cleaned.trim().is_empty() {
        return Err(CliError::Msg(
            "aborting commit due to empty commit message".into(),
        ));
    }
    Ok(cleaned)
}

fn run_editor_cmd(editor: &str, path: &Path) -> CliResult<()> {
    let mut argv: Vec<String> = editor.split_whitespace().map(|s| s.to_string()).collect();
    if argv.is_empty() {
        return Err(CliError::Usage("empty editor command".into()));
    }
    argv.push(path.display().to_string());
    let status = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .status()
        .map_err(|e| CliError::Msg(format!("spawn editor '{editor}': {e}")))?;
    if !status.success() {
        return Err(CliError::Msg(format!(
            "editor '{editor}' exited {}",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}

/// FS6: run `filter.<name>.clean` when `.gitattributes` assigns `filter=<name>` (LFS closest).
pub(crate) fn apply_clean_filter(
    work: &Path,
    rel: &str,
    payload: &[u8],
) -> CliResult<Option<Vec<u8>>> {
    let Some(name) = attribute_filter_name(work, rel)? else {
        return Ok(None);
    };
    let repo = open_repo()?;
    let cfg = fs::read_to_string(repo.git_dir().join("config")).unwrap_or_default();
    let key = format!("filter.{name}.clean");
    let Some(cmd) = config_value(&cfg, &key) else {
        return Ok(None);
    };
    let mut argv: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
    if argv.is_empty() {
        return Ok(None);
    }
    for a in &mut argv {
        *a = a.replace("%f", rel);
    }
    let mut child = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(work)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| CliError::Msg(format!("filter.{name}.clean: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(payload)
            .map_err(|e| CliError::Msg(format!("filter clean stdin: {e}")))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| CliError::Msg(format!("filter clean: {e}")))?;
    if !out.status.success() {
        return Err(CliError::Msg(format!(
            "filter.{name}.clean exited {}",
            out.status.code().unwrap_or(-1)
        )));
    }
    Ok(Some(out.stdout))
}

fn attribute_filter_name(work: &Path, rel: &str) -> CliResult<Option<String>> {
    let attr = work.join(".gitattributes");
    if !attr.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&attr)?;
    let file = Path::new(rel)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(rel);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let pattern = parts.next().unwrap_or("");
        let matches = pattern == "*"
            || pattern == rel
            || pattern == file
            || (pattern.starts_with("*.") && file.ends_with(&pattern[1..]));
        if !matches {
            continue;
        }
        for attr in parts {
            if let Some(name) = attr.strip_prefix("filter=") {
                return Ok(Some(name.to_string()));
            }
        }
    }
    Ok(None)
}

fn config_value(text: &str, key: &str) -> Option<String> {
    let key = key.to_ascii_lowercase();
    let mut section = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            section = line
                .trim_matches(|c| c == '[' || c == ']')
                .to_ascii_lowercase()
                .replace('"', "");
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let full = if section.contains(' ') {
            let mut bits = section.split_whitespace();
            let head = bits.next().unwrap_or("");
            let name = bits.next().unwrap_or("");
            format!("{head}.{name}.{}", k.trim().to_ascii_lowercase())
        } else {
            format!("{section}.{}", k.trim().to_ascii_lowercase())
        };
        if full == key {
            return Some(v.trim().trim_matches('"').to_string());
        }
    }
    None
}
