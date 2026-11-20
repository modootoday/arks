//! `clone`: option table and the local-path clone (plain path or same-host
//! `file://`): repository layout (`--bare`, `--mirror`,
//! `--separate-git-dir`, `--template`), object sharing (`--shared`,
//! `--reference[-if-able]`, `--dissociate`, `--no-hardlinks`), refs and
//! config (`--origin`, `--branch`, `--single-branch`, `--no-tags`,
//! `--config`, `--recurse-submodules`), checkout (`--no-checkout`,
//! `--sparse`). Network URLs go through `arks_remote`'s claimed clients.
//!
//! Primary: <https://git-scm.com/docs/git-clone>,
//! <https://git-scm.com/docs/gitrepository-layout> (`objects/info/alternates`,
//! gitfile), <https://git-scm.com/docs/git-sparse-checkout>.

use crate::cmd::opt_spec::{Parsed, parse};
use crate::cmd::remote_local::peel;
use crate::error::{CliError, CliResult};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::fs;
use std::path::{Path, PathBuf};

/// Every option `git clone -h` lists (pin 2.39.5 plus host 2.43 additions).
const CLONE_OPTS: &[&str] = &[
    "v|verbose",
    "q|quiet",
    "|progress",
    "|reject-shallow",
    "n|no-checkout",
    "|bare",
    "|mirror",
    "l|local",
    "|no-hardlinks",
    "s|shared",
    "|recurse-submodules=?",
    "|recursive=?",
    "j|jobs=",
    "|template=",
    "|reference=",
    "|reference-if-able=",
    "|dissociate",
    "o|origin=",
    "b|branch=",
    "u|upload-pack=",
    "|depth=",
    "|shallow-since=",
    "|shallow-exclude=",
    "|single-branch",
    "|tags",
    "|shallow-submodules",
    "|separate-git-dir=",
    "c|config=",
    "|server-option=",
    "4|ipv4",
    "6|ipv6",
    "|filter=",
    "|also-filter-submodules",
    "|remote-submodules",
    "|sparse",
    "|bundle-uri=",
    "|ref-format=",
];

/// Ordered config entries written into the new repository.
#[derive(Default)]
struct ConfigOut {
    entries: Vec<(String, String)>,
}

impl ConfigOut {
    fn set(&mut self, key: &str, value: &str) {
        self.entries.push((key.to_string(), value.to_string()));
    }

    /// Render as a git config file, opening a section whenever it changes.
    fn render(&self) -> String {
        let mut out = String::new();
        let mut current = String::new();
        for (k, v) in &self.entries {
            let (head, name) = match k.rsplit_once('.') {
                Some(x) => x,
                None => continue,
            };
            let header = match head.split_once('.') {
                Some((sec, sub)) => format!("[{sec} \"{sub}\"]"),
                None => format!("[{head}]"),
            };
            if header != current {
                out.push_str(&header);
                out.push('\n');
                current = header;
            }
            out.push_str(&format!("\t{name} = {v}\n"));
        }
        out
    }
}

pub fn cmd_clone(args: &[String]) -> CliResult<()> {
    let p = parse(args, CLONE_OPTS)?;
    if p.args.is_empty() {
        return Err(CliError::Exit(
            129,
            "fatal: You must specify a repository to clone.".into(),
        ));
    }
    if p.args.len() > 2 {
        return Err(CliError::Exit(129, "fatal: Too many arguments.".into()));
    }
    let bare = p.on("bare") || p.on("mirror");
    if bare && p.seen("origin") {
        return Err(CliError::fatal(
            "options '--bare' and '--origin foo' cannot be used together",
        ));
    }
    if bare && p.seen("separate-git-dir") {
        return Err(CliError::fatal(
            "options '--bare' and '--separate-git-dir' cannot be used together",
        ));
    }
    if p.on("also-filter-submodules") && !p.seen("filter") {
        return Err(CliError::fatal(
            "the option '--also-filter-submodules' requires '--filter'",
        ));
    }
    if p.on("also-filter-submodules") && !(p.seen("recurse-submodules") || p.seen("recursive")) {
        return Err(CliError::fatal(
            "the option '--also-filter-submodules' requires '--recurse-submodules'",
        ));
    }
    let url = p.args[0].clone();
    let dest = match p.args.get(1) {
        Some(d) => PathBuf::from(d),
        None => {
            let base = crate::cmd::remote::clone_dest_name(&url);
            PathBuf::from(if bare { format!("{base}.git") } else { base })
        }
    };
    if let Some(r) = p.value("reference") {
        crate::cmd::remote::refuse_busy_reference(Path::new(&r))?;
    }
    // NET-009: early refuse when dest already non-empty (before transport).
    if dest.exists() {
        let empty = dest.is_dir()
            && fs::read_dir(&dest)
                .map(|mut d| d.next().is_none())
                .unwrap_or(false);
        if !empty {
            return Err(CliError::fatal(format!(
                "destination path '{}' already exists and is not an empty directory.",
                dest.display()
            )));
        }
    }
    let local = arks_remote::LocalRemoteUrl::parse(&url).ok();
    match local {
        Some(l) => {
            if !l.path.exists() {
                return Err(CliError::fatal(format!(
                    "repository '{url}' does not exist"
                )));
            }
            clone_local(&p, &url, &l.path, &dest, bare)?;
        }
        None => clone_network(&p, &url, &dest)?,
    }
    if let Some(bundle) = p.value("bundle-uri") {
        crate::cmd::remote::seed_bundle_uri(&bundle, &dest)?;
    }
    Ok(())
}

fn humanish_progress(p: &Parsed, dest: &Path, bare: bool) {
    if p.on("quiet") {
        return;
    }
    if bare {
        eprintln!("Cloning into bare repository '{}'...", dest.display());
    } else {
        eprintln!("Cloning into '{}'...", dest.display());
    }
}

/// Network clone through the claimed `arks_remote` clients; options that
/// shape the result beyond depth / filter / config are refused honestly.
fn clone_network(p: &Parsed, url: &str, dest: &Path) -> CliResult<()> {
    for k in [
        "bare",
        "mirror",
        "origin",
        "branch",
        "single-branch",
        "separate-git-dir",
        "shared",
        "sparse",
        "no-checkout",
        "shallow-since",
        "shallow-exclude",
    ] {
        if p.on(k) || (p.value(k).is_some() && k != "no-checkout") {
            return Err(CliError::NotImplemented(format!(
                "clone --{k} is claimed for local-path / file:// sources only"
            )));
        }
    }
    humanish_progress(p, dest, false);
    let depth = p
        .value("depth")
        .map(|d| crate::cmd::remote::parse_depth(&d))
        .transpose()?;
    let filter = p
        .value("filter")
        .map(|f| crate::cmd::remote::parse_filter(&f))
        .transpose()?;
    let opts = arks_remote::FetchOptions {
        depth,
        filter,
        ..Default::default()
    };
    if arks_remote::HttpRemoteUrl::parse(url).is_ok() {
        arks_remote::clone_http_with(url, dest, &opts)?;
    } else if arks_remote::SshRemoteUrl::parse(url).is_ok() {
        arks_remote::clone_ssh_with(url, dest, &opts)?;
    } else if arks_remote::GitDaemonUrl::parse(url).is_ok() {
        arks_remote::clone_git_with(url, dest, &opts)?;
    } else {
        return Err(CliError::fatal(format!(
            "repository '{url}' does not exist"
        )));
    }
    let extra: Vec<String> = p.values("config");
    if !extra.is_empty() {
        let cfg = dest.join(".git/config");
        let mut text = fs::read_to_string(&cfg).unwrap_or_default();
        let mut out = ConfigOut::default();
        for kv in &extra {
            let (k, v) = kv.split_once('=').unwrap_or((kv.as_str(), "true"));
            out.set(k, v);
        }
        text.push_str(&out.render());
        fs::write(cfg, text)?;
    }
    Ok(())
}

/// Hardlink (or copy) every file under `src` into `dst`.
fn copy_tree(src: &Path, dst: &Path, hardlink: bool) -> CliResult<()> {
    fs::create_dir_all(dst)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let from = e.path();
        let to = dst.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_tree(&from, &to, hardlink)?;
        } else if to.exists() {
            continue;
        } else if !hardlink || fs::hard_link(&from, &to).is_err() {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// `objects/` directory of a repository path (work tree or bare).
fn objects_dir_of(path: &Path) -> Option<PathBuf> {
    if !path.is_dir() {
        return None;
    }
    let repo = Repository::discover(path).ok()?;
    repo.common_dir().join("objects").canonicalize().ok()
}

fn clone_local(p: &Parsed, url: &str, src_path: &Path, dest: &Path, bare: bool) -> CliResult<()> {
    let src = Repository::discover(src_path)
        .map_err(|_| CliError::fatal(format!("repository '{url}' does not exist")))?;
    let src_shallow = src.common_dir().join("shallow").is_file();
    if p.on("reject-shallow") && src_shallow {
        return Err(CliError::fatal(
            "source repository is shallow, reject to clone.",
        ));
    }
    let is_file_url = url.starts_with("file://");
    let mut single = p.flag("single-branch");
    for (k, flag) in [
        ("depth", "--depth"),
        ("shallow-since", "--shallow-since"),
        ("shallow-exclude", "--shallow-exclude"),
    ] {
        if p.value(k).is_some() {
            if single.is_none() {
                single = Some(true);
            }
            if !is_file_url {
                eprintln!("warning: {flag} is ignored in local clones; use file:// instead.");
            }
        }
    }
    if let Some(d) = p.value("depth") {
        crate::cmd::remote::parse_depth(&d)?;
    }
    if p.value("filter").is_some() && !is_file_url {
        eprintln!("warning: --filter is ignored in local clones; use file:// instead.");
    }
    let single = single.unwrap_or(false);
    let mirror = p.on("mirror");
    let origin = p.value("origin").unwrap_or_else(|| "origin".to_string());
    if origin.is_empty() || origin.contains(char::is_whitespace) || origin.contains("..") {
        return Err(CliError::fatal(format!(
            "'{origin}' is not a valid remote name"
        )));
    }

    // Choose what to check out before touching the destination.
    let src_refs = src.refs().list_refs("refs/")?;
    let head_branch = match src.head_target() {
        Ok(RefTarget::Symbolic(s)) if src_refs.iter().any(|(n, _)| *n == s) => Some(s),
        _ => None,
    };
    let (branch, detach): (Option<String>, Option<Oid>) = match p.value("branch") {
        Some(b) => {
            let head = format!("refs/heads/{b}");
            let tag = format!("refs/tags/{b}");
            if src_refs.iter().any(|(n, _)| *n == head) {
                (Some(head), None)
            } else if let Some((_, o)) = src_refs.iter().find(|(n, _)| *n == tag) {
                (None, Some(peel(&src, o)))
            } else {
                return Err(CliError::fatal(format!(
                    "Remote branch {b} not found in upstream {origin}"
                )));
            }
        }
        None => (head_branch.clone(), None),
    };
    let detach = detach.or_else(|| {
        if branch.is_none() && p.value("branch").is_none() {
            src.head_oid().ok()
        } else {
            None
        }
    });

    humanish_progress(p, dest, bare);
    let created = !dest.exists();
    fs::create_dir_all(dest)?;
    let res = clone_into(
        p,
        url,
        &src,
        &src_refs,
        dest,
        bare,
        mirror,
        single,
        &origin,
        &branch,
        &head_branch,
        detach,
    );
    if res.is_err() {
        if created {
            let _ = fs::remove_dir_all(dest);
        }
        if let Some(sep) = p.value("separate-git-dir") {
            let _ = fs::remove_dir_all(sep);
        }
    } else if !p.on("quiet") {
        eprintln!("done.");
    }
    res
}

#[allow(clippy::too_many_arguments)]
fn clone_into(
    p: &Parsed,
    url: &str,
    src: &Repository,
    src_refs: &[(String, Oid)],
    dest: &Path,
    bare: bool,
    mirror: bool,
    single: bool,
    origin: &str,
    branch: &Option<String>,
    head_branch: &Option<String>,
    detach: Option<Oid>,
) -> CliResult<()> {
    let dest_abs = dest.canonicalize()?;
    let git_dir = if bare {
        dest_abs.clone()
    } else if let Some(sep) = p.value("separate-git-dir") {
        fs::create_dir_all(&sep)?;
        let sep_abs = Path::new(&sep).canonicalize()?;
        fs::write(
            dest_abs.join(".git"),
            format!("gitdir: {}\n", sep_abs.display()),
        )?;
        sep_abs
    } else {
        dest_abs.join(".git")
    };
    for d in [
        "objects/info",
        "objects/pack",
        "refs/heads",
        "refs/tags",
        "info",
        "hooks",
    ] {
        fs::create_dir_all(git_dir.join(d))?;
    }
    fs::write(
        git_dir.join("description"),
        "Unnamed repository; edit this file 'description' to name the repository.\n",
    )?;
    if let Some(t) = p.value("template").filter(|t| !t.is_empty()) {
        if Path::new(&t).is_dir() {
            copy_tree(Path::new(&t), &git_dir, false)?;
        } else {
            eprintln!("warning: templates not found in {t}");
        }
    }
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n")?;

    // Config, in the order stock git writes it.
    let mut cfg = ConfigOut::default();
    let format_version = if p.value("filter").is_some() {
        "1"
    } else {
        "0"
    };
    cfg.set("core.repositoryformatversion", format_version);
    cfg.set("core.filemode", "true");
    cfg.set("core.bare", if bare { "true" } else { "false" });
    cfg.set("core.logallrefupdates", "true");
    for kv in p.values("config") {
        let (k, v) = kv.split_once('=').unwrap_or((kv.as_str(), "true"));
        cfg.set(k, v);
    }
    let recurse = p.seen("recurse-submodules") || p.seen("recursive");
    if recurse && (p.on("recurse-submodules") || p.on("recursive")) {
        let spec = p
            .value("recurse-submodules")
            .or_else(|| p.value("recursive"))
            .unwrap_or_else(|| ".".into());
        cfg.set("submodule.active", &spec);
    }
    // Local paths are recorded absolute but not normalized (`<cwd>/.` stays).
    let remote_url = if url.contains("://") || Path::new(url).is_absolute() {
        url.trim_end_matches('/').to_string()
    } else {
        std::env::current_dir()?
            .join(url.trim_end_matches('/'))
            .display()
            .to_string()
    };
    cfg.set(&format!("remote.{origin}.url"), &remote_url);
    if p.flag("tags") == Some(false) {
        cfg.set(&format!("remote.{origin}.tagOpt"), "--no-tags");
    }
    let fetch_spec = if mirror {
        Some("+refs/*:refs/*".to_string())
    } else if bare {
        None
    } else if single {
        branch
            .as_deref()
            .and_then(|b| b.strip_prefix("refs/heads/"))
            .map(|b| format!("+refs/heads/{b}:refs/remotes/{origin}/{b}"))
    } else {
        Some(format!("+refs/heads/*:refs/remotes/{origin}/*"))
    };
    if let Some(f) = &fetch_spec {
        cfg.set(&format!("remote.{origin}.fetch"), f);
    }
    if mirror {
        cfg.set(&format!("remote.{origin}.mirror"), "true");
    }
    // git records the partial-clone remote even when a local clone ignores
    // the filter (format version 1, promisor + filter on the remote).
    if let Some(f) = p.value("filter") {
        cfg.set(&format!("remote.{origin}.promisor"), "true");
        cfg.set(&format!("remote.{origin}.partialclonefilter"), &f);
    }
    if p.on("sparse") && !bare && !p.on("no-checkout") {
        cfg.set("extensions.worktreeConfig", "true");
    }
    if let Some(b) = branch
        .as_deref()
        .and_then(|b| b.strip_prefix("refs/heads/"))
    {
        if !bare {
            cfg.set(&format!("branch.{b}.remote"), origin);
            cfg.set(&format!("branch.{b}.merge"), &format!("refs/heads/{b}"));
        }
    }
    fs::write(git_dir.join("config"), cfg.render())?;

    // Objects: alternates for --shared / --reference, else hardlink or copy.
    let src_objects = src.common_dir().join("objects").canonicalize()?;
    let mut alternates: Vec<PathBuf> = Vec::new();
    if p.on("shared") {
        alternates.push(src_objects.clone());
    }
    for r in p.values("reference") {
        let o = objects_dir_of(Path::new(&r)).ok_or_else(|| {
            CliError::fatal(format!(
                "reference repository '{r}' is not a local repository."
            ))
        })?;
        alternates.push(o);
    }
    for r in p.values("reference-if-able") {
        match objects_dir_of(Path::new(&r)) {
            Some(o) => alternates.push(o),
            None => eprintln!(
                "info: Could not add alternate for '{r}': reference repository '{r}' is not a local repository."
            ),
        }
    }
    if !alternates.is_empty() {
        let body: String = alternates
            .iter()
            .map(|a| format!("{}\n", a.display()))
            .collect();
        fs::write(git_dir.join("objects/info/alternates"), body)?;
    }
    if !p.on("shared") {
        let hardlink = !p.on("no-hardlinks") && !url.starts_with("file://");
        copy_tree(&src_objects, &git_dir.join("objects"), hardlink)?;
    }
    if p.on("dissociate") && !alternates.is_empty() {
        for a in &alternates {
            copy_tree(a, &git_dir.join("objects"), false)?;
        }
        let _ = fs::remove_file(git_dir.join("objects/info/alternates"));
    }
    if src.common_dir().join("shallow").is_file() {
        fs::copy(src.common_dir().join("shallow"), git_dir.join("shallow"))?;
    }

    // Refs.
    let mut repo = Repository::open(&git_dir).map_err(|e| CliError::Msg(e.to_string()))?;
    let no_tags = p.flag("tags") == Some(false);
    let mut wrote: Vec<(String, Oid)> = Vec::new();
    for (n, oid) in src_refs {
        let dst = if mirror {
            Some(n.clone())
        } else if let Some(b) = n.strip_prefix("refs/heads/") {
            if bare {
                Some(n.clone())
            } else if single && branch.as_deref() != Some(n.as_str()) {
                None
            } else {
                Some(format!("refs/remotes/{origin}/{b}"))
            }
        } else if n.starts_with("refs/tags/") {
            (!no_tags).then(|| n.clone())
        } else {
            None
        };
        if let Some(d) = dst {
            repo.refs_mut().update(&d, oid, None)?;
            wrote.push((d, *oid));
        }
    }
    if !bare {
        if let Some(hb) = head_branch
            .as_deref()
            .and_then(|h| h.strip_prefix("refs/heads/"))
        {
            let tracking = format!("refs/remotes/{origin}/{hb}");
            let explicit_other = p.value("branch").is_some() && single;
            if wrote.iter().any(|(n, _)| *n == tracking) && !explicit_other {
                repo.refs_mut()
                    .set_symbolic(&format!("refs/remotes/{origin}/HEAD"), &tracking)?;
            }
        }
    }
    let message = format!("clone: from {remote_url}");
    let checkout_oid = match (branch, detach) {
        (Some(b), _) => {
            let oid = src.refs().resolve(b)?;
            repo.refs_mut().update(b, &oid, None)?;
            repo.refs_mut().set_symbolic("HEAD", b)?;
            if !bare {
                let _ = crate::reflog::append_reflog(&repo, b, None, &oid, &message);
                let _ = crate::reflog::append_reflog(&repo, "HEAD", None, &oid, &message);
            }
            Some(oid)
        }
        (None, Some(oid)) => {
            fs::write(git_dir.join("HEAD"), format!("{}\n", oid.to_hex()))?;
            if !bare {
                let _ = crate::reflog::append_reflog(&repo, "HEAD", None, &oid, &message);
            }
            Some(oid)
        }
        (None, None) => {
            eprintln!("warning: You appear to have cloned an empty repository.");
            None
        }
    };
    if bare || p.on("no-checkout") {
        return Ok(());
    }
    let Some(oid) = checkout_oid else {
        return Ok(());
    };
    arks_remote::checkout_commit(&repo, &dest_abs, &oid)?;
    if p.on("sparse") {
        sparse_top_level(&repo, &git_dir, &dest_abs)?;
    }
    Ok(())
}

/// `clone --sparse`: cone mode with only the top-level files checked out
/// (`/*`, `!/*/`); deeper paths get skip-worktree and leave the work tree.
fn sparse_top_level(repo: &Repository, git_dir: &Path, work: &Path) -> CliResult<()> {
    const CE_SKIP_WORKTREE: u16 = 0x4000;
    fs::write(
        git_dir.join("config.worktree"),
        "[core]\n\tsparseCheckout = true\n\tsparseCheckoutCone = true\n",
    )?;
    fs::write(git_dir.join("info/sparse-checkout"), "/*\n!/*/\n")?;
    let mut index = crate::util::read_index(repo)?;
    if index.version < 3 {
        index.version = 3;
    }
    for e in index.entries.iter_mut().filter(|e| e.path.contains('/')) {
        e.flags |= arks_index::CE_EXTENDED;
        e.flags_extended = Some(e.flags_extended.unwrap_or(0) | CE_SKIP_WORKTREE);
        let abs = work.join(&e.path);
        let _ = fs::remove_file(&abs);
        let mut dir = abs.parent();
        while let Some(d) = dir {
            if d == work || fs::remove_dir(d).is_err() {
                break;
            }
            dir = d.parent();
        }
    }
    crate::util::write_index(repo, &index)?;
    Ok(())
}
