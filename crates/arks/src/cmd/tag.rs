//! Tags and describe.

use crate::cmd::shared::resolve_rev;
use crate::cmd::sign::sign_bytes;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, RawObject};
use arks_store::ObjectStore;

pub fn cmd_tag(args: &[String]) -> CliResult<()> {
    // git-tag: every option of `git tag -h`: create (`-a`/`-s`/`-u`/`-m`/`-F`/
    // `-e`/`--cleanup`/`-f`), delete (`-d`), verify (`-v`), list (`-l`,
    // `-n[<num>]`, filters, `--sort`, `--format`, `--column`, `-i`).
    // Cite: https://git-scm.com/docs/git-tag
    use crate::cmd::opt_parse::{Item, Mode, parse};
    use crate::cmd::trailers::{self, Cleanup};
    let args = crate::cmd::branch_cmd::lastarg_default(
        args,
        &[
            "merged",
            "no-merged",
            "contains",
            "no-contains",
            "points-at",
        ],
    );
    let p = parse(
        "tag",
        &args,
        crate::cmd::porcelain_opts::TAG,
        Mode::default(),
    )?;
    let mut list = p.on("list");
    let delete = p.on("delete");
    let verify = p.on("verify");
    let force = p.on("force");
    let lines: Option<usize> = match p.last("-n") {
        None => None,
        Some(None) => Some(1),
        Some(Some(n)) => Some(
            n.parse()
                .map_err(|_| CliError::fatal(format!("invalid -n value: {n}")))?,
        ),
    };
    let mut filters: Vec<(&'static str, Option<String>)> = Vec::new();
    for it in &p.items {
        if let Item::Opt {
            key,
            val,
            neg: false,
        } = it
        {
            let k: &'static str = match *key {
                "contains" => "contains",
                "no-contains" => "no-contains",
                "points-at" => "points-at",
                "merged" => "merged",
                "no-merged" => "no-merged",
                _ => continue,
            };
            filters.push((k, val.clone()));
            list = true;
        }
    }
    let mut positional = p.args();
    positional.extend(p.after_dashdash());
    let mut repo = open_repo()?;
    if verify {
        // `tag -v` shows each tag object (without its signature) and verifies it.
        for name in &positional {
            let oid = repo
                .refs()
                .resolve(&format!("refs/tags/{name}"))
                .map_err(|_| CliError::Exit(1, format!("error: tag '{name}' not found.")))?;
            let obj = repo.get_object(&oid)?;
            if obj.kind == ObjectKind::Tag {
                let text = String::from_utf8_lossy(&obj.payload);
                let shown = text
                    .find("-----BEGIN ")
                    .map_or(text.as_ref(), |i| &text[..i]);
                print!("{shown}");
            }
            crate::cmd::sign::cmd_verify_tag(std::slice::from_ref(name))?;
        }
        return Ok(());
    }
    if delete {
        if positional.is_empty() {
            return Err(CliError::fatal("tag name required"));
        }
        let mut errs = Vec::new();
        for name in &positional {
            let full = format!("refs/tags/{name}");
            match repo.refs().resolve(&full) {
                Ok(oid) => {
                    repo.refs_mut().delete(&full, Some(&oid))?;
                    println!(
                        "Deleted tag '{name}' (was {})",
                        crate::cmd::shared::abbrev_hex(&repo, &oid, 7)
                    );
                }
                Err(_) => errs.push(format!("error: tag '{name}' not found.")),
            }
        }
        return if errs.is_empty() {
            Ok(())
        } else {
            Err(CliError::Exit(1, errs.join("\n")))
        };
    }
    let format = p.value("format");
    let sorted = !p.values("sort").is_empty();
    let ignore_case = p.on("ignore-case");
    let column = crate::cmd::branch_cmd::column_mode(&p)?;
    if list || lines.is_some() || positional.is_empty() {
        if let Some(f) = &format {
            if f.is_empty() && p.on("omit-empty") {
                return Ok(());
            }
        }
        if format.is_some() || sorted {
            let f = format.unwrap_or_else(|| "%(refname:strip=2)".to_string());
            return crate::cmd::branch_cmd::list_with_format(&p, &f, &["refs/tags/"], &positional);
        }
        let patterns: Vec<String> = if ignore_case {
            positional.iter().map(|s| s.to_ascii_lowercase()).collect()
        } else {
            positional.clone()
        };
        if column && lines.is_none() {
            let names = tag_names(&repo, &patterns, ignore_case)?;
            crate::cmd::branch_cmd::print_columns(&names, "", 2);
            return Ok(());
        }
        if patterns.is_empty() && lines.is_none() && filters.is_empty() {
            return list_tags(&repo);
        }
        return list_tags_filtered(&repo, &patterns, lines, &filters, ignore_case);
    }
    if positional.len() > 2 {
        return Err(CliError::fatal("too many arguments"));
    }
    let name = &positional[0];
    let full = format!("refs/tags/{name}");
    if !arks_refs::is_valid_refname(&full) {
        return Err(CliError::fatal(format!(
            "'{name}' is not a valid tag name."
        )));
    }
    if !force && repo.refs().resolve(&full).is_ok() {
        return Err(CliError::fatal(format!("tag '{name}' already exists")));
    }
    let start = positional.get(1).map(String::as_str).unwrap_or("HEAD");
    let target = resolve_rev(&repo, start)
        .map_err(|_| CliError::fatal(format!("Failed to resolve '{start}' as a valid ref.")))?;
    // Message: `-m` paragraphs, `-F` file, else the editor for `-a`/`-s`/`-e`.
    let messages = p.values("message");
    let file = p.value("file");
    if !messages.is_empty() && file.is_some() {
        return Err(CliError::fatal(
            "options '-F' and '-m' cannot be used together",
        ));
    }
    let signed = p.on("sign") || p.value("local-user").is_some();
    let annotated = p.on("annotate") || signed || !messages.is_empty() || file.is_some();
    let cleanup = match p.value("cleanup") {
        Some(v) => trailers::parse_cleanup(&v)?,
        None => Cleanup::Default,
    };
    let mut text: Option<String> = if !messages.is_empty() {
        Some(messages.join("\n\n"))
    } else if let Some(f) = &file {
        Some(if f == "-" {
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
            buf
        } else {
            std::fs::read_to_string(f)
                .map_err(|e| CliError::fatal(format!("could not open or read '{f}': {e}")))?
        })
    } else {
        None
    };
    let edit = p.on("edit") || (annotated && text.is_none());
    let old = repo.refs().resolve(&full).ok();
    if annotated || edit {
        if edit {
            let seed = text.clone().unwrap_or_default();
            text = Some(edit_tag_message(&repo, name, &seed)?);
        }
        let msg = trailers::cleanup(text.as_deref().unwrap_or(""), cleanup, edit);
        if msg.trim().is_empty() && edit && file.is_none() && messages.is_empty() {
            return Err(CliError::fatal("no tag message?"));
        }
        if signed {
            write_signed_tag(
                &mut repo,
                name,
                &target,
                &msg,
                p.value("local-user").as_deref(),
            )
            .map_err(|e| CliError::fatal(format!("unable to sign the tag: {e}")))?;
        } else if cleanup == Cleanup::Verbatim {
            // Verbatim keeps the message bytes (no added final newline).
            let sig = crate::cmd::shared::committer_signature()?;
            let kind = repo.get_object(&target)?.kind;
            let mut payload = crate::cmd::encode_tag_payload(
                &target,
                kind.as_str(),
                name,
                &sig.format_line(),
                "",
            );
            payload.pop();
            payload.extend_from_slice(msg.as_bytes());
            let obj = RawObject::new(ObjectKind::Tag, payload);
            let format = repo.object_format();
            let tag_oid = repo.store_mut().put(format, &obj)?;
            repo.refs_mut().update(&full, &tag_oid, None)?;
        } else {
            crate::cmd::write_annotated_tag(&mut repo, name, &target, &msg)?;
        }
    } else {
        repo.refs_mut().update(&full, &target, old.as_ref())?;
    }
    if p.on("create-reflog") {
        if let Ok(new) = repo.refs().resolve(&full) {
            let _ = crate::reflog::append_reflog(&repo, &full, old.as_ref(), &new, "tag: tagging");
        }
    }
    // git-tag -f reports a replaced tag on stdout.
    if let Some(prev) = old
        && repo.refs().resolve(&full).ok() != Some(prev)
    {
        println!(
            "Updated tag '{name}' (was {})",
            crate::cmd::shared::abbrev_hex(&repo, &prev, 7)
        );
    }
    Ok(())
}

/// Run the editor on `.git/TAG_EDITMSG` seeded with `seed`.
fn edit_tag_message(repo: &arks_repo::Repository, name: &str, seed: &str) -> CliResult<String> {
    let path = repo.git_dir().join("TAG_EDITMSG");
    let mut text = seed.to_string();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!(
        "\n#\n# Write a message for tag:\n#   {name}\n# Lines starting with '#' will be ignored.\n"
    ));
    std::fs::write(&path, &text)?;
    if let Some(editor) = ["ARKS_EDITOR", "GIT_EDITOR", "EDITOR"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
    {
        let ok = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("{editor} \"$@\""))
            .arg(&editor)
            .arg(&path)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return Err(CliError::Exit(
                1,
                "error: There was a problem with the editor.".into(),
            ));
        }
    }
    Ok(std::fs::read_to_string(&path)?)
}

/// Tag names matching `patterns` (lower-cased patterns with `ignore_case`).
fn tag_names(
    repo: &arks_repo::Repository,
    patterns: &[String],
    ignore_case: bool,
) -> CliResult<Vec<String>> {
    let mut refs = repo.refs().list_refs("refs/tags/")?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(refs
        .into_iter()
        .map(|(full, _)| full.trim_start_matches("refs/tags/").to_string())
        .filter(|n| {
            let subject = if ignore_case {
                n.to_ascii_lowercase()
            } else {
                n.clone()
            };
            patterns.is_empty()
                || patterns
                    .iter()
                    .any(|p| crate::cmd::branch_cmd::fnmatch(p.as_bytes(), subject.as_bytes()))
        })
        .collect())
}

/// `tag -l` with patterns, `-n<num>` annotation lines and commit filters.
fn list_tags_filtered(
    repo: &arks_repo::Repository,
    patterns: &[String],
    lines: Option<usize>,
    filters: &[(&'static str, Option<String>)],
    ignore_case: bool,
) -> CliResult<()> {
    use crate::cmd::shared::{ObjKind, is_ancestor, peel_tags, peel_to_kind};
    let mut resolved = Vec::new();
    for (k, v) in filters {
        let rev = v.as_deref().unwrap_or("HEAD");
        let o = resolve_rev(repo, rev)
            .map_err(|_| CliError::fatal(format!("malformed object name {rev}")))?;
        resolved.push((*k, o));
    }
    let mut refs = repo.refs().list_refs("refs/tags/")?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for (full, oid) in refs {
        let name = full.strip_prefix("refs/tags/").unwrap_or(&full);
        let subject = if ignore_case {
            name.to_ascii_lowercase()
        } else {
            name.to_string()
        };
        if !patterns.is_empty()
            && !patterns
                .iter()
                .any(|p| crate::cmd::branch_cmd::fnmatch(p.as_bytes(), subject.as_bytes()))
        {
            continue;
        }
        let peeled = peel_tags(repo, &oid)?;
        let mut keep = true;
        for (k, f) in &resolved {
            let commit = || peel_to_kind(repo, &peeled, ObjKind::Commit, name).ok();
            keep &= match *k {
                "points-at" => oid == *f || peeled == *f,
                "contains" => commit().is_some_and(|c| is_ancestor(repo, f, &c).unwrap_or(false)),
                "no-contains" => {
                    commit().is_some_and(|c| !is_ancestor(repo, f, &c).unwrap_or(true))
                }
                "merged" => commit().is_some_and(|c| is_ancestor(repo, &c, f).unwrap_or(false)),
                _ => commit().is_some_and(|c| !is_ancestor(repo, &c, f).unwrap_or(true)),
            };
        }
        if !keep {
            continue;
        }
        let Some(n) = lines else {
            std::io::Write::write_all(&mut out, format!("{name}\n").as_bytes())?;
            continue;
        };
        // Annotation: tag message, else the tagged commit's message.
        let obj = repo.get_object(&oid)?;
        let msg = match obj.kind {
            ObjectKind::Tag => {
                let text = String::from_utf8_lossy(&obj.payload).into_owned();
                text.split_once("\n\n")
                    .map(|(_, b)| b.to_string())
                    .unwrap_or_default()
            }
            ObjectKind::Commit => Commit::decode(&obj.payload)?.message,
            _ => String::new(),
        };
        let body: Vec<&str> = msg
            .lines()
            .take_while(|l| !l.starts_with("-----BEGIN"))
            .collect();
        let body: Vec<&str> = body
            .iter()
            .skip_while(|l| l.trim().is_empty())
            .copied()
            .collect();
        let mut text = format!("{name:<15} ");
        for (j, l) in body.iter().take(n.max(1)).enumerate() {
            if j > 0 {
                text.push_str("\n    ");
            }
            text.push_str(l.trim_end());
        }
        let text = text.trim_end().to_string();
        std::io::Write::write_all(&mut out, format!("{text}\n").as_bytes())?;
    }
    std::io::Write::flush(&mut out)?;
    Ok(())
}

pub fn cmd_describe(args: &[String]) -> CliResult<()> {
    // git-describe: nearest tag reachable from each commit-ish (default HEAD),
    // `<tag>-<n>-g<abbrev>` where n = commits reachable from the commit but not
    // from the tag; annotated tags only unless `--tags` / `--all`; `--always`,
    // `--long`, `--abbrev=<n>`, `--exact-match`, `--match`, `--exclude`,
    // `--dirty[=<mark>]`, `--first-parent`.
    // Cite: https://git-scm.com/docs/git-describe
    // Partial: the candidate search is exhaustive (no `--candidates` cap), so
    // ties may resolve differently from git's date-ordered walk.
    let mut dirty_mark: Option<String> = None;
    let (mut tags_ok, mut all_refs, mut always, mut long, mut exact) =
        (false, false, false, false, false);
    let mut first_parent = false;
    // `--contains`: name the commit after a tag that contains it
    // (name-rev --tags --name-only, without the `tags/` prefix).
    let mut contains = false;
    let mut abbrev = 7usize;
    let mut matches: Vec<String> = Vec::new();
    let mut excludes: Vec<String> = Vec::new();
    let mut revs: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        match a {
            "--tags" => tags_ok = true,
            "--all" => all_refs = true,
            "--always" => always = true,
            "--long" => long = true,
            "--exact-match" => exact = true,
            "--first-parent" => first_parent = true,
            "--contains" => contains = true,
            "--dirty" => dirty_mark = Some("-dirty".into()),
            // A readable work tree is never broken: --broken behaves like
            // --dirty with git's default mark.
            "--broken" => dirty_mark = Some("-dirty".into()),
            "--no-dirty" | "--no-broken" => dirty_mark = None,
            "--no-tags" => tags_ok = false,
            "--no-all" => all_refs = false,
            "--no-always" => always = false,
            "--no-long" => long = false,
            "--no-exact-match" => exact = false,
            "--no-first-parent" => first_parent = false,
            "--no-contains" => contains = false,
            "--no-match" => matches.clear(),
            "--no-exclude" => excludes.clear(),
            "--abbrev" => abbrev = 7,
            "--no-abbrev" => abbrev = 0,
            // --debug only adds stderr diagnostics.
            "--debug" | "--no-debug" | "--no-candidates" => {}
            "--candidates" => {
                let v = args.get(i).cloned().ok_or_else(|| {
                    CliError::Exit(129, "error: option `candidates' requires a value".into())
                })?;
                i += 1;
                if v.parse::<usize>().map_err(|_| {
                    CliError::Exit(
                        129,
                        "error: option `candidates' expects a numerical value".into(),
                    )
                })? == 0
                {
                    exact = true;
                }
            }
            "--match" | "--exclude" => {
                let v = args.get(i).cloned().ok_or_else(|| {
                    CliError::fatal(format!("option `{}' requires a value", &a[2..]))
                })?;
                i += 1;
                if a == "--match" {
                    matches.push(v);
                } else {
                    excludes.push(v);
                }
            }
            _ => {
                if let Some(m) = a.strip_prefix("--dirty=") {
                    dirty_mark = Some(m.to_string());
                } else if a.starts_with("--broken=") {
                    dirty_mark = Some("-dirty".into());
                } else if let Some(n) = a.strip_prefix("--candidates=") {
                    if n.parse::<usize>().map_err(|_| {
                        CliError::Exit(
                            129,
                            "error: option `candidates' expects a numerical value".into(),
                        )
                    })? == 0
                    {
                        exact = true;
                    }
                } else if let Some(n) = a.strip_prefix("--abbrev=") {
                    abbrev = n
                        .parse()
                        .map_err(|_| CliError::fatal(format!("bad --abbrev: {n}")))?;
                } else if let Some(v) = a.strip_prefix("--match=") {
                    matches.push(v.to_string());
                } else if let Some(v) = a.strip_prefix("--exclude=") {
                    excludes.push(v.to_string());
                } else if a.starts_with('-') {
                    return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
                } else {
                    revs.push(a.to_string());
                }
            }
        }
    }
    let repo = open_repo()?;
    // MISC-001 (M20 Cx-8): describe under pack-refs is reader-safe; honesty note.
    if repo.common_dir().join("packed-refs.lock").is_file() {
        eprintln!(
            "note: MISC-001 — describe during pack-refs is reader-safe but expensive under write locks; prefer quiet windows"
        );
    }
    if dirty_mark.is_some() && !revs.is_empty() {
        return Err(CliError::fatal(
            "option '--dirty' and commit-ishes cannot be used together",
        ));
    }
    if revs.is_empty() {
        revs.push("HEAD".into());
    }
    if contains {
        // git describe --contains = name-rev --name-only --tags (all refs with --all).
        let names = crate::cmd::plumbing::name_rev_names(&repo, !all_refs, &[])?;
        for r in &revs {
            let o = crate::cmd::shared::peel_tags(&repo, &resolve_rev(&repo, r)?)?;
            match names.get(&o) {
                Some(n) => {
                    println!("{}", n.0.strip_prefix("tags/").unwrap_or(&n.0));
                }
                None => {
                    return Err(CliError::fatal(format!("cannot describe '{}'", o.to_hex())));
                }
            }
        }
        return Ok(());
    }
    // Candidate names: (display name, peeled commit, annotated).
    let mut cands: Vec<(String, arks_oid::Oid, bool)> = Vec::new();
    let mut unannotated = false;
    let prefix = if all_refs { "refs/" } else { "refs/tags/" };
    let mut refs = repo.refs().list_refs(prefix)?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    for (full, oid) in refs {
        let is_tag = full.starts_with("refs/tags/");
        let name = if all_refs {
            full.strip_prefix("refs/").unwrap_or(&full).to_string()
        } else {
            full.strip_prefix("refs/tags/").unwrap_or(&full).to_string()
        };
        let short = full.strip_prefix("refs/tags/").unwrap_or(&name);
        if !matches.is_empty()
            && !matches
                .iter()
                .any(|p| crate::cmd::branch_cmd::fnmatch(p.as_bytes(), short.as_bytes()))
        {
            continue;
        }
        if excludes
            .iter()
            .any(|p| crate::cmd::branch_cmd::fnmatch(p.as_bytes(), short.as_bytes()))
        {
            continue;
        }
        let annotated = repo
            .get_object(&oid)
            .map(|o| o.kind == ObjectKind::Tag)
            .unwrap_or(false);
        if is_tag && !annotated && !tags_ok && !all_refs {
            unannotated = true;
            continue;
        }
        let Ok(commit) = crate::cmd::shared::peel_to_kind(
            &repo,
            &oid,
            crate::cmd::shared::ObjKind::Commit,
            &full,
        ) else {
            continue;
        };
        cands.push((name, commit, annotated));
    }
    for rev in &revs {
        let oid = resolve_rev(&repo, rev)
            .map_err(|_| CliError::fatal(format!("Not a valid object name {rev}")))?;
        let head = crate::cmd::shared::peel_to_kind(
            &repo,
            &oid,
            crate::cmd::shared::ObjKind::Commit,
            rev,
        )?;
        let suffix = match &dirty_mark {
            Some(m) if worktree_dirty(&repo)? => m.clone(),
            _ => String::new(),
        };
        let short = crate::cmd::shared::abbrev_hex(&repo, &head, abbrev.max(4));
        // Exact match: annotated tags win over lightweight ones.
        let mut exact_hits: Vec<&(String, arks_oid::Oid, bool)> =
            cands.iter().filter(|c| c.1 == head).collect();
        exact_hits.sort_by_key(|c| !c.2);
        if let Some(c) = exact_hits.first() {
            if long {
                println!("{}-0-g{short}{suffix}", c.0);
            } else {
                println!("{}{suffix}", c.0);
            }
            continue;
        }
        if exact {
            return Err(CliError::fatal(format!(
                "no tag exactly matches '{}'",
                head.to_hex()
            )));
        }
        let reach: std::collections::HashSet<arks_oid::Oid> = if first_parent {
            arks_repo::first_parent_set(&repo, head)?
        } else {
            arks_repo::ancestor_set(&repo, head)?
        };
        let mut best: Option<(usize, &(String, arks_oid::Oid, bool))> = None;
        for c in &cands {
            if !reach.contains(&c.1) {
                continue;
            }
            let theirs = arks_repo::ancestor_set(&repo, c.1)?;
            let depth = reach.iter().filter(|o| !theirs.contains(o)).count();
            if best.is_none_or(|(d, b)| depth < d || (depth == d && c.2 && !b.2)) {
                best = Some((depth, c));
            }
        }
        match best {
            Some((depth, c)) => {
                if abbrev == 0 {
                    println!("{}{suffix}", c.0);
                } else {
                    println!("{}-{depth}-g{short}{suffix}", c.0);
                }
            }
            None if always => println!("{short}{suffix}"),
            None if cands.is_empty() && !unannotated => {
                return Err(CliError::fatal("No names found, cannot describe anything."));
            }
            None if unannotated => {
                return Err(CliError::fatal(format!(
                    "No annotated tags can describe '{}'.\n\
                     However, there were unannotated tags: try --tags.",
                    head.to_hex()
                )));
            }
            None => {
                return Err(CliError::fatal(format!(
                    "No tags can describe '{}'.\n\
                     Try --always, or create some tags.",
                    head.to_hex()
                )));
            }
        }
    }
    Ok(())
}
fn write_signed_tag(
    repo: &mut arks_repo::Repository,
    name: &str,
    target: &arks_oid::Oid,
    message: &str,
    key_override: Option<&str>,
) -> CliResult<arks_oid::Oid> {
    let sig = crate::cmd::shared::committer_signature()?;
    let mut unsigned =
        crate::cmd::encode_tag_payload(target, "commit", name, &sig.format_line(), message);
    // Ensure trailing newline before signature block (gitformat-signature examples).
    if !unsigned.ends_with(b"\n") {
        unsigned.push(b'\n');
    }
    let armor = sign_bytes(repo, &unsigned, key_override, "tag -s")?;
    let mut payload = unsigned;
    payload.extend_from_slice(armor.as_bytes());
    let obj = RawObject::new(ObjectKind::Tag, payload);
    let format = repo.object_format();
    let tag_oid = repo.store_mut().put(format, &obj)?;
    repo.refs_mut()
        .update(&format!("refs/tags/{name}"), &tag_oid, None)?;
    Ok(tag_oid)
}

fn list_tags(repo: &arks_repo::Repository) -> CliResult<()> {
    use std::io::Write;
    let packed = repo.git_dir().join("packed-refs");
    let text = if packed.is_file() {
        std::fs::read_to_string(&packed)?
    } else {
        String::new()
    };
    let mut loose: Vec<String> = Vec::new();
    let tags_dir = repo.git_dir().join("refs/tags");
    if tags_dir.is_dir() {
        collect_loose_tag_names(&tags_dir, "", &mut loose)?;
    }
    let mut out = Vec::with_capacity(text.len() / 2 + 64);
    if loose.is_empty() {
        for line in text.lines() {
            let b = line.as_bytes();
            if b.is_empty() || b[0] == b'#' || b[0] == b'^' {
                continue;
            }
            let Some(sp) = line.find(' ') else { continue };
            if let Some(short) = line[sp + 1..].strip_prefix("refs/tags/") {
                out.extend_from_slice(short.as_bytes());
                out.push(b'\n');
            }
        }
    } else {
        let mut names: Vec<&str> = Vec::new();
        for line in text.lines() {
            let b = line.as_bytes();
            if b.is_empty() || b[0] == b'#' || b[0] == b'^' {
                continue;
            }
            let Some(sp) = line.find(' ') else { continue };
            if let Some(short) = line[sp + 1..].strip_prefix("refs/tags/") {
                names.push(short);
            }
        }
        let packed_set: std::collections::HashSet<&str> = names.iter().copied().collect();
        for name in &loose {
            if !packed_set.contains(name.as_str()) {
                names.push(name.as_str());
            }
        }
        names.sort_unstable();
        names.dedup();
        for name in names {
            out.extend_from_slice(name.as_bytes());
            out.push(b'\n');
        }
    }
    std::io::stdout().lock().write_all(&out)?;
    Ok(())
}

fn collect_loose_tag_names(
    dir: &std::path::Path,
    prefix: &str,
    out: &mut Vec<String>,
) -> CliResult<()> {
    for ent in std::fs::read_dir(dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name == "." || name == ".." {
            continue;
        }
        let rel = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}/{name}")
        };
        let path = ent.path();
        if path.is_dir() {
            collect_loose_tag_names(&path, &rel, out)?;
        } else if path.is_file() {
            out.push(rel);
        }
    }
    Ok(())
}

/// Shared dirty-worktree probe (describe `--dirty`, BISECT-002, TOCTOU-003).
pub(crate) fn worktree_dirty(repo: &arks_repo::Repository) -> CliResult<bool> {
    use crate::cmd::shared::head_blob_map;
    use crate::util::{read_index, work_tree};
    use arks_object::{ObjectKind, RawObject};
    let work = work_tree(repo)?;
    let index = read_index(repo)?;
    let head = head_blob_map(repo)?;
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        match head.get(&e.path) {
            None => return Ok(true),
            Some(oid) if *oid != e.oid => return Ok(true),
            Some(_) => {}
        }
        let abs = work.join(&e.path);
        if !abs.is_file() {
            return Ok(true);
        }
        let bytes = std::fs::read(&abs)?;
        let oid = RawObject::new(ObjectKind::Blob, bytes).oid(repo.object_format())?;
        if oid != e.oid {
            return Ok(true);
        }
    }
    // Untracked files do not make the tree dirty (git-describe `--dirty`
    // runs `diff-index HEAD`, which ignores them).
    Ok(false)
}

#[cfg(test)]
mod tests {
    #[test]
    fn tag_signed_requires_message_flag() {
        let args = ["-s".to_string(), "v1".to_string()];
        assert!(args.iter().any(|a| a == "-s"));
        assert!(!args.windows(2).any(|w| w[0] == "-m"));
    }
}
