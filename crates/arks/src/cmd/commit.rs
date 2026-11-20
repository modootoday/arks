//! `git commit`: every option of `git commit -h`.
//!
//! Message sources (`-m`, `-F`, `-C`, `-c`, `--fixup`, `--squash`, template,
//! amend, an in-progress merge), sign-off and `--trailer`, the editor
//! (`GIT_EDITOR`), `--cleanup` modes, author overrides, the commit index
//! (`-a`, `-i`, `-o`, `-p`, `--interactive`, pathspec files) and the
//! status-shaped `--dry-run` family (`--short`, `--porcelain`, `--long`,
//! `-z`, `--branch`, `-u`, `-v`).
//! Primary: <https://git-scm.com/docs/git-commit>.

use crate::cmd::core::{
    add_signoff, cmd_add, pathspec_rel, print_commit_summary, run_claimed_hooks,
};
use crate::cmd::opt_parse::{Item, Mode, Parsed, parse, read_pathspec_file};
use crate::cmd::shared::{commit_signature, committer_signature, resolve_rev};
use crate::cmd::sign::{sign_commit, want_sign_commit};
use crate::cmd::trailers::{self, Cleanup, IfExists, IfMissing, NewTrailer, Where};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use crate::write_tree::write_tree_from_index;
use arks_index::sort_entries;
use arks_object::{Commit, ObjectKind, RawObject, Signature};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::fs;
use std::io::Read;

/// Status-format options of commit (they imply `--dry-run`).
const STATUS_FORMATS: &[&str] = &["short", "porcelain", "long", "null"];

fn fatal(msg: impl Into<String>) -> CliError {
    CliError::fatal(msg)
}

/// Sign flags in the spelling `want_sign_commit` reads.
pub(crate) fn sign_args(p: &Parsed) -> Vec<String> {
    p.items
        .iter()
        .filter_map(|it| match it {
            Item::Opt {
                key: "gpg-sign",
                val,
                neg,
            } => Some(if *neg {
                "--no-gpg-sign".to_string()
            } else {
                match val {
                    Some(k) => format!("--gpg-sign={k}"),
                    None => "--gpg-sign".to_string(),
                }
            }),
            _ => None,
        })
        .collect()
}

fn read_commit(repo: &Repository, rev: &str) -> CliResult<(Oid, Commit)> {
    let oid =
        resolve_rev(repo, rev).map_err(|_| fatal(format!("could not lookup commit '{rev}'")))?;
    let obj = repo.get_object(&oid)?;
    if obj.kind != ObjectKind::Commit {
        return Err(fatal(format!("could not lookup commit '{rev}'")));
    }
    Ok((oid, Commit::decode(&obj.payload)?))
}

/// `--author=<author>`: `Name <email>`, or a pattern matched against the
/// authors of commits reachable from HEAD (first match wins).
fn author_from_arg(repo: &Repository, spec: &str) -> CliResult<(String, String)> {
    if let Some((n, rest)) = spec.split_once('<') {
        if let Some(e) = rest.strip_suffix('>') {
            return Ok((n.trim().to_string(), e.trim().to_string()));
        }
    }
    if let Ok(head) = repo.head_oid() {
        let mut queue = vec![head];
        let mut seen = std::collections::HashSet::new();
        while let Some(o) = queue.pop() {
            if !seen.insert(o) {
                continue;
            }
            let c = crate::cmd::log_format::read_commit(repo, &o)?;
            let ident = format!("{} <{}>", c.author.name, c.author.email);
            if ident.contains(spec) {
                return Ok((c.author.name.clone(), c.author.email.clone()));
            }
            queue.extend(c.parents.iter().rev().copied());
        }
    }
    Err(fatal(format!(
        "--author '{spec}' is not 'Name <email>' and matches no existing author"
    )))
}

/// The commit message seed and whether the editor runs on it.
struct Seed {
    text: String,
    editor: bool,
    /// The template text (to detect an untouched template).
    template: Option<String>,
}

fn editor_command() -> Option<String> {
    ["ARKS_EDITOR", "GIT_EDITOR", "EDITOR"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
}

/// Run the editor on `seed` through `.git/COMMIT_EDITMSG`; returns the file.
fn run_editor(repo: &Repository, seed: &str, cleanup: Cleanup) -> CliResult<String> {
    let path = repo.git_dir().join("COMMIT_EDITMSG");
    let mut text = seed.to_string();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("\n# Please enter the commit message for your changes.");
    if matches!(
        cleanup,
        Cleanup::Strip | Cleanup::Default | Cleanup::Scissors
    ) {
        text.push_str(" Lines starting\n# with '#' will be ignored, and an empty message aborts the commit.\n");
    } else {
        text.push_str(" Lines starting\n# with '#' will be kept; you may remove them yourself if you want to.\n# An empty message aborts the commit.\n");
    }
    fs::write(&path, &text)?;
    let Some(editor) = editor_command() else {
        return Ok(text);
    };
    if editor == ":" {
        return Ok(text);
    }
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("{editor} \"$@\""))
        .arg(&editor)
        .arg(&path)
        .status()
        .map_err(|e| CliError::Msg(format!("could not launch editor '{editor}': {e}")))?;
    if !status.success() {
        return Err(CliError::Exit(
            1,
            "error: There was a problem with the editor.\nPlease supply the message using either -m or -F option.".into(),
        ));
    }
    Ok(fs::read_to_string(&path)?)
}

/// Parsed commit options in one place.
struct Opts {
    messages: Vec<String>,
    file: Option<String>,
    reuse: Option<(String, bool)>,
    fixup: Option<String>,
    squash: Option<String>,
    template: Option<String>,
    edit: Option<bool>,
    cleanup: Cleanup,
    signoff: bool,
    trailers: Vec<String>,
    author: Option<String>,
    date: Option<String>,
    reset_author: bool,
    amend: bool,
    all: bool,
    include: bool,
    patch: bool,
    interactive: bool,
    allow_empty: bool,
    allow_empty_message: bool,
    no_verify: bool,
    quiet: bool,
    dry_run: bool,
    pathspecs: Vec<String>,
}

fn opts_from(p: &Parsed) -> CliResult<Opts> {
    // `-c` / `-C`: the last one wins; `-c` edits.
    let mut reuse = None;
    for it in &p.items {
        if let Item::Opt { key, val, neg } = it {
            match (*key, *neg) {
                ("reuse-message", false) => reuse = val.clone().map(|v| (v, false)),
                ("reedit-message", false) => reuse = val.clone().map(|v| (v, true)),
                ("reuse-message" | "reedit-message", true) => reuse = None,
                _ => {}
            }
        }
    }
    let file = p.value("file");
    let messages = p.values("message");
    let fixup = p.value("fixup");
    let squash = p.value("squash");
    let sources = [file.is_some(), reuse.is_some(), fixup.is_some()];
    if sources.iter().filter(|b| **b).count() > 1 {
        return Err(fatal("Only one of -c/-C/-F/--fixup can be used."));
    }
    if !messages.is_empty() && (file.is_some() || reuse.is_some()) {
        return Err(fatal("Option -m cannot be combined with -c/-C/-F."));
    }
    if squash.is_some() && fixup.is_some() {
        return Err(fatal(
            "options '--squash' and '--fixup' cannot be used together",
        ));
    }
    let amend = p.on("amend");
    let reset_author = p.on("reset-author");
    if reset_author && reuse.is_none() && !amend {
        return Err(fatal(
            "--reset-author can be used only with -C, -c or --amend.",
        ));
    }
    let cleanup = match p.value("cleanup") {
        Some(v) => trailers::parse_cleanup(&v)?,
        None => Cleanup::Default,
    };
    let mut pathspecs = p.args();
    pathspecs.extend(p.after_dashdash());
    if let Some(f) = p.value("pathspec-from-file") {
        if !pathspecs.is_empty() {
            return Err(fatal(
                "'--pathspec-from-file' and pathspec arguments cannot be used together",
            ));
        }
        pathspecs = read_pathspec_file(&f, p.on("pathspec-file-nul"))?;
    } else if p.on("pathspec-file-nul") {
        return Err(fatal(
            "the option '--pathspec-file-nul' requires '--pathspec-from-file'",
        ));
    }
    let (all, include, only, patch, interactive) = (
        p.on("all"),
        p.on("include"),
        p.on("only"),
        p.on("patch"),
        p.on("interactive"),
    );
    if [all, include, only, interactive, patch]
        .iter()
        .filter(|b| **b)
        .count()
        > 1
    {
        return Err(fatal(
            "Only one of --include/--only/--all/--interactive/--patch can be used.",
        ));
    }
    if all && !pathspecs.is_empty() {
        return Err(fatal(format!(
            "paths '{} ...' with -a does not make sense",
            pathspecs[0]
        )));
    }
    let allow_empty = p.on("allow-empty");
    if pathspecs.is_empty() && (include || (only && !allow_empty && !amend)) {
        return Err(fatal("No paths with --include/--only does not make sense."));
    }
    let dry_run = p.on("dry-run") || STATUS_FORMATS.iter().any(|k| p.on(k));
    Ok(Opts {
        messages,
        file,
        reuse,
        fixup,
        squash,
        template: p.value("template"),
        edit: p.flag("edit"),
        cleanup,
        signoff: p.on("signoff"),
        trailers: p.values("trailer"),
        author: p.value("author"),
        date: p.value("date"),
        reset_author,
        amend,
        all,
        include,
        patch,
        interactive,
        allow_empty,
        allow_empty_message: p.on("allow-empty-message"),
        no_verify: p.on("no-verify"),
        quiet: p.on("quiet"),
        dry_run,
        pathspecs,
    })
}

/// Build the message seed (before sign-off, trailers, editor and cleanup).
fn message_seed(repo: &Repository, o: &Opts, prev: Option<&Commit>) -> CliResult<Option<Seed>> {
    let joined = (!o.messages.is_empty()).then(|| o.messages.join("\n\n"));
    let editor_if = |default: bool| o.edit.unwrap_or(default);
    if let Some(spec) = &o.fixup {
        let (kind, rev) = match spec.split_once(':') {
            Some(("amend", r)) => ("amend", r),
            Some(("reword", r)) => ("reword", r),
            _ => ("fixup", spec.as_str()),
        };
        let (_, c) = read_commit(repo, rev)?;
        let subject = crate::cmd::log_format::subject_line(&c.message);
        if kind == "fixup" {
            let mut text = format!("fixup! {subject}\n\n");
            if let Some(m) = &joined {
                text.push_str(m);
                text.push('\n');
            }
            return Ok(Some(Seed {
                text,
                editor: editor_if(false),
                template: None,
            }));
        }
        if joined.is_some() {
            return Err(fatal(format!(
                "options '-m' and '--fixup={kind}' cannot be used together"
            )));
        }
        // The amend! message carries the target's whole message as its body.
        let text = format!("amend! {subject}\n\n{}", c.message);
        return Ok(Some(Seed {
            text,
            editor: editor_if(true),
            template: None,
        }));
    }
    if let Some(m) = &joined {
        let mut text = String::new();
        if let Some(rev) = &o.squash {
            let (_, c) = read_commit(repo, rev)?;
            text.push_str(&format!(
                "squash! {}\n\n",
                crate::cmd::log_format::subject_line(&c.message)
            ));
        }
        text.push_str(m);
        text.push('\n');
        return Ok(Some(Seed {
            text,
            editor: editor_if(false),
            template: None,
        }));
    }
    if let Some(f) = &o.file {
        let mut text = String::new();
        if f == "-" {
            std::io::stdin().read_to_string(&mut text)?;
        } else {
            text = fs::read_to_string(f)
                .map_err(|e| fatal(format!("could not read log file '{f}': {e}")))?;
        }
        return Ok(Some(Seed {
            text,
            editor: editor_if(false),
            template: None,
        }));
    }
    if let Some((rev, reedit)) = &o.reuse {
        let (_, c) = read_commit(repo, rev)?;
        return Ok(Some(Seed {
            text: c.message,
            editor: editor_if(*reedit),
            template: None,
        }));
    }
    if let Some(rev) = &o.squash {
        let (_, c) = read_commit(repo, rev)?;
        return Ok(Some(Seed {
            text: format!(
                "squash! {}\n\n",
                crate::cmd::log_format::subject_line(&c.message)
            ),
            editor: editor_if(true),
            template: None,
        }));
    }
    let merge_msg = repo.git_dir().join("MERGE_MSG");
    if let Ok(text) = fs::read_to_string(&merge_msg) {
        return Ok(Some(Seed {
            text,
            editor: editor_if(true),
            template: None,
        }));
    }
    if let Some(prev) = prev {
        return Ok(Some(Seed {
            text: prev.message.clone(),
            editor: editor_if(true),
            template: None,
        }));
    }
    let template_path = o.template.clone().or_else(|| {
        let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
        cfg.get("commit.template")
    });
    let template = match template_path {
        Some(t) => {
            Some(fs::read_to_string(&t).map_err(|e| fatal(format!("could not read '{t}': {e}")))?)
        }
        None => None,
    };
    if o.edit == Some(false) {
        return Ok(None);
    }
    Ok(Some(Seed {
        text: template.clone().unwrap_or_default(),
        editor: true,
        template,
    }))
}

fn index_map(entries: &[arks_index::IndexEntry]) -> std::collections::BTreeMap<String, Oid> {
    entries
        .iter()
        .filter(|e| e.stage() == 0)
        .map(|e| (e.path.clone(), e.oid))
        .collect()
}

fn tree_map(
    repo: &Repository,
    commit: Option<&Oid>,
) -> CliResult<std::collections::BTreeMap<String, Oid>> {
    let mut v = Vec::new();
    if let Some(c) = commit {
        let tree = crate::cmd::log_format::read_commit(repo, c)?.tree;
        crate::cmd::plumbing::flatten_tree_to_index(repo, &tree, "", &mut v)?;
    }
    Ok(v.into_iter().map(|e| (e.path, e.oid)).collect())
}

/// Every pathspec must name something in the index or HEAD.
fn check_pathspecs_known(specs: &[String]) -> CliResult<()> {
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut known: Vec<String> = read_index(&repo)?
        .entries
        .iter()
        .map(|e| e.path.clone())
        .collect();
    known.extend(tree_map(&repo, repo.head_oid().ok().as_ref())?.into_keys());
    for s in specs {
        let rel = pathspec_rel(&work, s)?;
        let one = [rel];
        if !known
            .iter()
            .any(|k| crate::cmd::log_format::path_in_specs(k, &one))
        {
            return Err(CliError::Exit(
                1,
                format!("error: pathspec '{s}' did not match any file(s) known to git"),
            ));
        }
    }
    Ok(())
}

/// Stage what the commit takes and return the index entries to commit.
fn prepare_index(o: &Opts) -> CliResult<Vec<arks_index::IndexEntry>> {
    if o.all {
        cmd_add(&["-u".into()])?;
    }
    if !o.pathspecs.is_empty() && !o.patch {
        check_pathspecs_known(&o.pathspecs)?;
    }
    if o.patch {
        let mut a = vec!["-p".to_string(), "--".to_string()];
        a.extend(o.pathspecs.iter().cloned());
        cmd_add(&a)?;
    } else if o.interactive {
        cmd_add(&["-i".to_string()])?;
    } else if !o.pathspecs.is_empty() {
        cmd_add(&o.pathspecs)?;
    }
    let repo = open_repo()?;
    let mut index = read_index(&repo)?;
    if !o.pathspecs.is_empty() && !o.include && !o.patch {
        // `--only` (the default with paths): HEAD plus the named paths.
        let work = work_tree(&repo)?;
        let specs = o
            .pathspecs
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
    Ok(index.entries)
}

/// `--dry-run` and the status formats: show what would be committed (using
/// a temporary copy of the index) and exit 1 when nothing would be.
fn dry_run(p: &Parsed, o: &Opts) -> CliResult<()> {
    let repo = open_repo()?;
    let index_path = repo.index_path();
    let saved = fs::read(&index_path).ok();
    let result = (|| -> CliResult<bool> {
        let entries = prepare_index(o)?;
        let repo = open_repo()?;
        let mut index = read_index(&repo)?;
        index.entries = entries;
        crate::util::write_index(&repo, &index)?;
        let head = repo.head_oid().ok();
        let base = if o.amend {
            head.and_then(|h| {
                crate::cmd::log_format::read_commit(&repo, &h)
                    .ok()
                    .and_then(|c| c.parents.first().copied())
            })
        } else {
            head
        };
        let committable = tree_map(&repo, base.as_ref())? != index_map(&index.entries);
        let mut st: Vec<String> = Vec::new();
        for it in &p.items {
            if let Item::Opt { key, val, neg } = it {
                let k = *key;
                if matches!(
                    k,
                    "short" | "porcelain" | "long" | "null" | "branch" | "ahead-behind" | "verbose"
                ) {
                    st.push(if *neg {
                        format!("--no-{k}")
                    } else {
                        format!("--{k}")
                    });
                } else if k == "untracked-files" {
                    st.push(match (neg, val) {
                        (true, _) => "--no-untracked-files".into(),
                        (false, Some(v)) => format!("--untracked-files={v}"),
                        (false, None) => "--untracked-files".into(),
                    });
                }
            }
        }
        crate::cmd::core::cmd_status(&st)?;
        Ok(committable)
    })();
    if let Some(bytes) = saved {
        fs::write(&index_path, bytes)?;
    }
    if result? {
        Ok(())
    } else {
        Err(CliError::silent(1))
    }
}

pub fn cmd_commit(args: &[String]) -> CliResult<()> {
    let p = parse(
        "commit",
        args,
        crate::cmd::porcelain_opts::COMMIT,
        Mode::default(),
    )?;
    let o = opts_from(&p)?;
    // REF-009 (M20 Cx-4): early refuse when reflog lock held (commit appends logs).
    {
        let repo = open_repo()?;
        crate::reflog::refuse_if_reflog_locked(&repo, "HEAD")?;
        if let Ok(RefTarget::Symbolic(name)) = repo.head_target() {
            crate::reflog::refuse_if_reflog_locked(&repo, &name)?;
        }
    }
    if o.dry_run {
        return dry_run(&p, &o);
    }
    let reword = o.fixup.as_deref().is_some_and(|f| f.starts_with("reword:"));
    let entries = if reword {
        // `--fixup=reword:` records only a message: the tree stays HEAD's.
        let repo = open_repo()?;
        let mut v = Vec::new();
        if let Ok(h) = repo.head_oid() {
            let tree = crate::cmd::log_format::read_commit(&repo, &h)?.tree;
            crate::cmd::plumbing::flatten_tree_to_index(&repo, &tree, "", &mut v)?;
        }
        v
    } else {
        prepare_index(&o)?
    };
    let mut repo = open_repo()?;
    run_claimed_hooks(&repo, o.no_verify, &["pre-commit"], None)?;
    let head = repo.head_oid().ok();
    let prev = match (o.amend, head) {
        (true, Some(h)) => Some(crate::cmd::log_format::read_commit(&repo, &h)?),
        (true, None) => return Err(fatal("You have nothing to amend.")),
        _ => None,
    };
    let merge_heads: Vec<Oid> = fs::read_to_string(repo.git_dir().join("MERGE_HEAD"))
        .map(|t| {
            t.lines()
                .filter_map(|l| Oid::from_hex(l.trim()).ok())
                .collect()
        })
        .unwrap_or_default();
    let parents: Vec<Oid> = match &prev {
        Some(c) => c.parents.clone(),
        None => head
            .into_iter()
            .chain(merge_heads.iter().copied())
            .collect(),
    };
    let tree = {
        let format = repo.object_format();
        if entries.is_empty() {
            repo.store_mut()
                .put(format, &RawObject::new(ObjectKind::Tree, Vec::new()))?
        } else {
            write_tree_from_index(repo.store_mut(), &entries, format)?
        }
    };
    let allow_empty = o.allow_empty || reword;
    if prev.is_none() && merge_heads.is_empty() && !allow_empty {
        let parent_tree = tree_map(&repo, head.as_ref())?;
        if parent_tree == index_map(&entries) {
            // git prints the status (what is not staged) and exits 1.
            let mut st = Vec::new();
            if !o.pathspecs.is_empty() {
                st.push("--".to_string());
                st.extend(o.pathspecs.iter().cloned());
            }
            crate::cmd::core::cmd_status(&st)?;
            return Err(CliError::silent(1));
        }
    }
    // Message.
    let seed = message_seed(&repo, &o, prev.as_ref())?;
    let Some(seed) = seed else {
        return Err(CliError::Exit(
            1,
            "Aborting commit due to empty commit message.".into(),
        ));
    };
    let mut text = seed.text.clone();
    if o.signoff {
        text = add_signoff(&text);
    }
    if !o.trailers.is_empty() {
        let new = o
            .trailers
            .iter()
            .map(|t| {
                let (token, value) = trailers::split_arg(t)?;
                Ok(NewTrailer {
                    token,
                    value,
                    place: Where::End,
                    if_exists: IfExists::AddIfDifferentNeighbor,
                    if_missing: IfMissing::Add,
                })
            })
            .collect::<CliResult<Vec<_>>>()?;
        text = trailers::process(&text, &new, &trailers::Opts::default());
    }
    let edited = seed.editor && editor_command().is_some();
    if seed.editor {
        let raw = run_editor(&repo, &text, o.cleanup)?;
        text = raw;
    }
    let message = trailers::cleanup(&text, o.cleanup, edited);
    if message.trim().is_empty() && !o.allow_empty_message {
        return Err(CliError::Exit(
            1,
            "Aborting commit due to empty commit message.".into(),
        ));
    }
    if let Some(t) = &seed.template {
        if !t.trim().is_empty() && message == trailers::cleanup(t, o.cleanup, edited) {
            return Err(CliError::Exit(
                1,
                "Aborting commit; you did not edit the message.".into(),
            ));
        }
    }
    run_claimed_hooks(
        &repo,
        o.no_verify,
        &["commit-msg"],
        Some(message.as_bytes()),
    )?;
    // Author.
    let mut author: Signature = match (&o.reuse, &prev, o.reset_author) {
        (Some((rev, _)), _, false) => read_commit(&repo, rev)?.1.author,
        (None, Some(c), false) => c.author.clone(),
        _ => commit_signature()?,
    };
    if let Some(spec) = &o.author {
        let (n, e) = author_from_arg(&repo, spec)?;
        author.name = n;
        author.email = e;
    }
    if let Some(d) = &o.date {
        let (ts, tz) = crate::cmd::shared::parse_git_date(d)
            .ok_or_else(|| fatal(format!("invalid date format: {d}")))?;
        author.timestamp = ts;
        author.timezone = tz;
    }
    let committer = committer_signature()?;
    let subject = message.lines().next().unwrap_or("").to_owned();
    let (do_sign, sign_key) = want_sign_commit(&sign_args(&p), &repo)?;
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
    let what = if prev.is_some() {
        "commit (amend)"
    } else if !merge_heads.is_empty() {
        "commit (merge)"
    } else if parents.is_empty() {
        "commit (initial)"
    } else {
        "commit"
    };
    let reflog_msg = format!("{what}: {subject}");
    // M16 Sy-5: detached HEAD commit updates HEAD directly (git-commit).
    match repo.head_target()? {
        RefTarget::Symbolic(name) => {
            repo.refs_mut().update(&name, &coid, head.as_ref())?;
            let _ = crate::reflog::append_head_and_branch(
                &repo,
                &name,
                head.as_ref(),
                &coid,
                &reflog_msg,
            );
        }
        RefTarget::Direct(_) => {
            repo.refs_mut().update("HEAD", &coid, head.as_ref())?;
            let _ = crate::reflog::append_reflog(&repo, "HEAD", head.as_ref(), &coid, &reflog_msg);
        }
    }
    for f in [
        "MERGE_HEAD",
        "MERGE_MSG",
        "MERGE_MODE",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
    ] {
        let _ = fs::remove_file(repo.git_dir().join(f));
    }
    if !o.quiet {
        print_commit_summary(&repo, &coid)?;
    }
    Ok(())
}
