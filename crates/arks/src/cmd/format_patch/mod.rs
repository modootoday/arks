//! `git format-patch`: every option of its `-h` list.
//!
//! Each commit becomes one mailbox message (`From <oid> Mon Sep 17 00:00:00
//! 2001`, `From:`, `Date:`, `Subject: [<prefix> n/m] <subject>`, optional
//! threading / `To:` / `Cc:` / extra headers, the body, `---`, the diffstat,
//! the patch and the `-- ` signature); `--cover-letter` adds message 0 with
//! the shortlog and diffstat of the series; `--attach` / `--inline` wrap the
//! patch in MIME parts; `--base` records the base commit and the patch ids
//! of prerequisite commits. A non-ASCII subject or author name is Q-encoded
//! unless `--no-encode-email-headers` / `format.encodeEmailHeaders=false`,
//! and an 8-bit message declares its charset (`mail_header`).
//! Cite: <https://git-scm.com/docs/git-format-patch>,
//! <https://git-scm.com/docs/git-patch-id> (behaviour, not code).

mod mail_header;
mod wrap;

use crate::cmd::opt_parse::{Item, Mode, Parsed, parse};
use crate::cmd::shared::{committer_signature, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{cwd, open_repo};
use arks_object::Commit;
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Default `-- ` signature and MIME boundary suffix: the peer Git version arks is pinned to. git-format-patch(1)
/// `--signature`: when omitted the signature defaults to the Git version number (pin: `2.39.5`).
const SIGNATURE_VERSION: &str = "2.39.5";

#[derive(Default)]
struct Opts {
    numbered: Option<bool>,
    signoff: bool,
    stdout: bool,
    cover_letter: bool,
    numbered_files: bool,
    suffix: String,
    start_number: usize,
    reroll: Option<String>,
    max_len: usize,
    subject_prefix: String,
    keep_subject: bool,
    no_binary: bool,
    zero_commit: bool,
    ignore_in_upstream: bool,
    no_stat: bool,
    headers: Vec<String>,
    to: Vec<String>,
    cc: Vec<String>,
    from: Option<String>,
    in_reply_to: Option<String>,
    attach: Option<Option<String>>,
    inline: bool,
    thread: Option<String>,
    signature: Option<String>,
    base: Option<String>,
    quiet: bool,
    force_in_body_from: bool,
    encode_headers: bool,
    cover_from_description: String,
    description: Option<String>,
    out_dir: Option<PathBuf>,
    interdiff: Option<String>,
    range_diff: Option<String>,
    creation_factor: i64,
}

fn options(p: &Parsed) -> CliResult<Opts> {
    let cfg = arks_config::ConfigSet::load(
        open_repo()
            .ok()
            .map(|r| r.common_dir().to_path_buf())
            .as_deref(),
    );
    let mut o = Opts {
        numbered: None,
        signoff: p.on("signoff"),
        stdout: p.on("stdout"),
        cover_letter: p.on("cover-letter"),
        numbered_files: p.on("numbered-files"),
        suffix: p.value("suffix").unwrap_or_else(|| ".patch".into()),
        start_number: 1,
        reroll: p.value("reroll-count"),
        max_len: 64,
        subject_prefix: p
            .value("subject-prefix")
            .or_else(|| cfg.get("format.subjectPrefix"))
            .unwrap_or_else(|| "PATCH".into()),
        keep_subject: p.on("keep-subject"),
        no_binary: p.on("no-binary"),
        zero_commit: p.on("zero-commit"),
        ignore_in_upstream: p.on("ignore-if-in-upstream"),
        no_stat: p.on("no-stat"),
        headers: p.values("add-header"),
        to: p.values("to"),
        cc: p.values("cc"),
        from: None,
        in_reply_to: p.value("in-reply-to"),
        attach: None,
        inline: false,
        thread: None,
        signature: None,
        base: p.value("base"),
        quiet: p.on("quiet"),
        force_in_body_from: p.on("force-in-body-from"),
        encode_headers: cfg.get_bool("format.encodeEmailHeaders").unwrap_or(true),
        cover_from_description: p
            .value("cover-from-description")
            .unwrap_or_else(|| "message".into()),
        description: None,
        out_dir: p.value("output-directory").map(PathBuf::from),
        ..Opts::default()
    };
    for it in &p.items {
        let Item::Opt { key, val, neg } = it else {
            continue;
        };
        match (*key, neg) {
            ("numbered", false) => o.numbered = Some(true),
            ("numbered", true) | ("no-numbered", _) => o.numbered = Some(false),
            ("from", false) => {
                let me = committer_signature()?;
                o.from = Some(
                    val.clone()
                        .unwrap_or_else(|| format!("{} <{}>", me.name, me.email)),
                );
            }
            ("from", true) => o.from = None,
            ("attach", false) => {
                o.attach = Some(val.clone());
                o.inline = false;
            }
            ("attach", true) => o.attach = None,
            ("inline", _) => {
                o.attach = Some(val.clone());
                o.inline = true;
            }
            ("thread", false) => o.thread = Some(val.clone().unwrap_or_else(|| "shallow".into())),
            ("thread", true) => o.thread = None,
            ("rfc", false) => o.subject_prefix = format!("RFC {}", o.subject_prefix),
            ("encode-email-headers", neg) => o.encode_headers = !neg,
            _ => {}
        }
    }
    if let Some(t) = &o.thread {
        if !matches!(t.as_str(), "shallow" | "deep") {
            return Err(CliError::Exit(
                129,
                format!("error: invalid value for '--thread': '{t}'"),
            ));
        }
    }
    o.signature = match p.last("signature") {
        Some(Some(s)) => Some(s),
        _ if p.flag("signature") == Some(false) => None,
        _ => match p.value("signature-file") {
            Some(f) => Some(fs::read_to_string(&f).map_err(|e| {
                CliError::fatal(format!("unable to read signature file '{f}': {e}"))
            })?),
            None => Some(SIGNATURE_VERSION.to_string()),
        },
    };
    if let Some(n) = p.value("start-number") {
        o.start_number = n.parse().map_err(|_| {
            CliError::Exit(
                129,
                "error: option `start-number' expects a numerical value".into(),
            )
        })?;
    }
    if let Some(n) = p.value("filename-max-length") {
        o.max_len = n.parse().map_err(|_| {
            CliError::Exit(
                129,
                "error: option `filename-max-length' expects a numerical value".into(),
            )
        })?;
    }
    if !matches!(
        o.cover_from_description.as_str(),
        "message" | "default" | "subject" | "auto" | "none"
    ) {
        return Err(CliError::fatal(format!(
            "{}: invalid cover from description mode",
            o.cover_from_description
        )));
    }
    if let Some(f) = p.value("description-file") {
        o.description = Some(
            fs::read_to_string(&f)
                .map_err(|e| CliError::fatal(format!("could not read '{f}': {e}")))?,
        );
    }
    if o.keep_subject && o.numbered == Some(true) {
        return Err(CliError::fatal(
            "options '-n' and '-k' cannot be used together",
        ));
    }
    if o.stdout && o.out_dir.is_some() {
        return Err(CliError::fatal(
            "options '--stdout' and '--output-directory ...' cannot be used together",
        ));
    }
    o.interdiff = p.value("interdiff");
    o.range_diff = p.value("range-diff");
    o.creation_factor = crate::cmd::range_diff::embed::DEFAULT_CREATION_FACTOR;
    if let Some(v) = p.value("creation-factor") {
        if o.range_diff.is_none() {
            return Err(CliError::fatal(
                "the option '--creation-factor' requires '--range-diff'",
            ));
        }
        o.creation_factor = crate::cmd::range_diff::parse_factor(Some(&v))?;
    }
    Ok(o)
}

pub fn cmd_format_patch(args: &[String]) -> CliResult<()> {
    let p = parse(
        "format-patch",
        args,
        crate::cmd::porcelain_opts::FORMAT_PATCH,
        Mode {
            numbers: true,
            ..Mode::default()
        },
    )?;
    let o = options(&p)?;
    let count = p.items.iter().rev().find_map(|it| match it {
        Item::Number(n) => n.trim_start_matches('-').parse::<usize>().ok(),
        _ => None,
    });
    let positional = p.args();
    let repo = open_repo()?;
    let resolve_commit = |rev: &str| -> CliResult<Oid> {
        let oid = resolve_rev(&repo, rev)?;
        crate::cmd::shared::peel_to_kind(&repo, &oid, crate::cmd::shared::ObjKind::Commit, rev)
    };
    // (series commits oldest first, upstream tip for --ignore-if-in-upstream / --base auto)
    let (commits, upstream): (Vec<Oid>, Option<Oid>) = if let Some(n) = count {
        if positional.len() > 1 {
            return Err(CliError::Usage(
                "format-patch: -<n> accepts at most one commit".into(),
            ));
        }
        let tip = resolve_commit(positional.first().map(String::as_str).unwrap_or("HEAD"))?;
        let mut picked: Vec<Oid> = crate::cmd::log_format::walk(&repo, &[tip], &[], false)?
            .into_iter()
            .filter(|(_, c)| c.parents.len() <= 1)
            .take(n)
            .map(|(o, _)| o)
            .collect();
        picked.reverse();
        (picked, None)
    } else if positional.len() == 1 {
        let spec = positional[0].clone();
        let (left, right) = match spec.split_once("..") {
            Some((l, r)) => (
                if l.is_empty() {
                    "HEAD".to_string()
                } else {
                    l.to_string()
                },
                if r.is_empty() {
                    "HEAD".to_string()
                } else {
                    r.to_string()
                },
            ),
            None => (spec.clone(), "HEAD".to_string()),
        };
        if spec.contains("...") {
            return Err(CliError::NotImplemented(
                "format-patch symmetric difference is not implemented".into(),
            ));
        }
        let start = resolve_commit(&left)?;
        let end = resolve_commit(&right)?;
        let hidden = arks_repo::ancestor_set(&repo, start)?;
        let mut picked: Vec<Oid> = crate::cmd::log_format::walk(&repo, &[end], &[], false)?
            .into_iter()
            .filter(|(o, c)| !hidden.contains(o) && c.parents.len() <= 1)
            .map(|(o, _)| o)
            .collect();
        picked.reverse();
        (picked, Some(start))
    } else if positional.is_empty() {
        return Err(CliError::Usage(
            "format-patch: -<n> [<commit>] or <since> or <A>..<B>".into(),
        ));
    } else {
        return Err(CliError::NotImplemented(
            "format-patch with several revision arguments is not implemented".into(),
        ));
    };
    let mut commits = commits;
    if let (true, Some(u)) = (o.ignore_in_upstream, upstream) {
        // Drop commits whose patch id appears on the upstream side.
        let ours = arks_repo::ancestor_set(&repo, *commits.last().unwrap_or(&u))?;
        let mut theirs = HashSet::new();
        for (oid, c) in crate::cmd::log_format::walk(&repo, &[u], &[], false)? {
            if ours.contains(&oid) {
                break;
            }
            if c.parents.len() <= 1 {
                theirs.insert(patch_id(&repo, &oid)?);
            }
        }
        let mut kept = Vec::new();
        for c in commits {
            if !theirs.contains(&patch_id(&repo, &c)?) {
                kept.push(c);
            }
        }
        commits = kept;
    }
    if commits.is_empty() {
        return Ok(());
    }
    let total = commits.len();
    // `--interdiff` / `--range-diff` sections (cover letter or single patch).
    let reroll = reroll_sections(&repo, &o, &commits, upstream)?;
    let numbered = !o.keep_subject && o.numbered.unwrap_or(total > 1 || o.cover_letter);
    let version = o
        .reroll
        .as_ref()
        .map(|v| format!(" v{v}"))
        .unwrap_or_default();
    let file_version = o
        .reroll
        .as_ref()
        .map(|v| format!("v{v}-"))
        .unwrap_or_default();
    // Threading: Message-ID per message from the current time, like git.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let me = committer_signature()?;
    let msg_id = |what: &str| format!("<{what}.{stamp}.git.{}>", me.email);
    let base_info = match &o.base {
        None => None,
        Some(b) => {
            let base = if b == "auto" {
                upstream.ok_or_else(|| {
                    CliError::fatal("base commit should be the ancestor of revision list")
                })?
            } else {
                resolve_commit(b)?
            };
            Some(base_section(&repo, &base, &commits)?)
        }
    };
    let mut rendered: Vec<(String, String)> = Vec::new();
    let mut first_id: Option<String> = None;
    let mut prev_id: Option<String> = None;
    let mut thread_ids: Vec<(Option<String>, Option<String>, Option<String>)> = Vec::new();
    let messages = total + usize::from(o.cover_letter);
    for k in 0..messages {
        let what = if o.cover_letter && k == 0 {
            "cover".to_string()
        } else {
            commits[k - usize::from(o.cover_letter)].to_hex()
        };
        if o.thread.is_some() {
            let id = msg_id(&what);
            let (irt, refs) = match (&o.in_reply_to, &first_id, &prev_id) {
                (_, None, _) => (o.in_reply_to.clone(), o.in_reply_to.clone()),
                (_, Some(first), Some(prev)) => {
                    let deep = o.thread.as_deref() == Some("deep");
                    let parent = if deep { prev.clone() } else { first.clone() };
                    let refs = match &o.in_reply_to {
                        Some(r) => format!("{r}\n {first}"),
                        None => first.clone(),
                    };
                    (Some(parent), Some(refs))
                }
                _ => (None, None),
            };
            if first_id.is_none() {
                first_id = Some(id.clone());
            }
            prev_id = Some(id.clone());
            thread_ids.push((Some(id), irt, refs));
        } else {
            thread_ids.push((None, o.in_reply_to.clone(), o.in_reply_to.clone()));
        }
    }
    if o.cover_letter {
        let body = render_cover(
            &repo,
            &commits,
            &o,
            numbered,
            &version,
            &thread_ids[0],
            base_info.as_deref(),
            &reroll.cover,
        )?;
        let name = if o.numbered_files {
            format!("{}", o.start_number - 1)
        } else {
            format!("{file_version}0000-cover-letter{}", o.suffix)
        };
        rendered.push((name, body));
    }
    for (n, oid) in commits.iter().enumerate() {
        let num = o.start_number + n;
        let shown_total = total + o.start_number - 1;
        let prefix = if o.keep_subject {
            String::new()
        } else if numbered {
            // The number is zero-padded to the width of the total.
            let w = shown_total.to_string().len();
            format!("[{}{version} {num:0w$}/{shown_total}] ", o.subject_prefix)
        } else {
            format!("[{}{version}] ", o.subject_prefix)
        };
        let commit = read_commit(&repo, oid)?;
        let slug = patch_slug(&crate::cmd::log_format::subject_line(&commit.message));
        let name = if o.numbered_files {
            format!("{num}")
        } else {
            // The name before the suffix is cut to `--filename-max-length`
            // minus the suffix and one, but never below four characters.
            let full = format!("{file_version}{num:04}-{slug}");
            let keep = o.max_len.saturating_sub(o.suffix.len() + 1).max(4);
            let cut: String = full.chars().take(keep).collect();
            format!("{cut}{}", o.suffix)
        };
        let ids = &thread_ids[n + usize::from(o.cover_letter)];
        let last = n + 1 == total;
        let body = render_patch(
            &repo,
            oid,
            &commit,
            &prefix,
            &o,
            ids,
            &name,
            if last && !o.cover_letter {
                base_info.as_deref()
            } else {
                None
            },
            if o.cover_letter { "" } else { &reroll.patch },
        )?;
        rendered.push((name, body));
    }
    drop(repo);
    if o.stdout {
        // Patches after the first are separated by a blank line (the cover
        // letter is not part of that sequence).
        let skip = usize::from(o.cover_letter);
        for (k, (_, body)) in rendered.into_iter().enumerate() {
            if k > skip {
                println!();
            }
            print!("{body}");
        }
        return Ok(());
    }
    if let Some(dir) = &o.out_dir {
        fs::create_dir_all(dir)?;
    }
    let base = match &o.out_dir {
        Some(d) => d.clone(),
        None => cwd()?,
    };
    for (name, body) in rendered {
        let target = base.join(&name);
        fs::write(&target, body).map_err(|e| {
            CliError::fatal(format!(
                "cannot open patch file {}: {e}\nfatal: failed to create output files",
                target.display()
            ))
        })?;
        match &o.out_dir {
            _ if o.quiet => {}
            Some(d) => println!("{}", d.join(&name).display()),
            None => println!("{name}"),
        }
    }
    Ok(())
}

fn read_commit(repo: &Repository, oid: &Oid) -> CliResult<Commit> {
    crate::cmd::log_format::read_commit(repo, oid)
}

/// The `--interdiff` / `--range-diff` text for the cover letter and for a
/// single patch (after its `---` line).
#[derive(Default)]
struct RerollSections {
    cover: String,
    patch: String,
}

fn reroll_sections(
    repo: &Repository,
    o: &Opts,
    commits: &[Oid],
    upstream: Option<Oid>,
) -> CliResult<RerollSections> {
    use crate::cmd::range_diff::embed;
    let mut out = RerollSections::default();
    if o.interdiff.is_none() && o.range_diff.is_none() {
        return Ok(out);
    }
    for (k, given) in [("interdiff", &o.interdiff), ("range-diff", &o.range_diff)] {
        if given.is_some() && !o.cover_letter && commits.len() != 1 {
            return Err(CliError::fatal(format!(
                "--{k} requires --cover-letter or single patch"
            )));
        }
    }
    let tip = *commits.last().expect("non-empty series");
    // The series' base: the bottom of `<since>..<until>`, else the parent of
    // its first commit.
    let origin = match upstream {
        Some(u) => Some(u),
        None => read_commit(repo, &commits[0])?.parents.first().copied(),
    };
    let reroll = o.reroll.as_deref();
    if let Some(prev) = &o.interdiff {
        let title = embed::title("Interdiff", reroll);
        let diff = embed::interdiff(repo, prev, &tip, !o.no_binary)?;
        out.cover.push_str(&format!("{title}\n{diff}"));
        out.patch
            .push_str(&format!("{title}\n{}\n", embed::indent(&diff)));
    }
    if let Some(prev) = &o.range_diff {
        let title = embed::title("Range-diff", reroll);
        let rd = embed::range_diff(repo, prev, origin.as_ref(), commits, o.creation_factor)?;
        out.cover.push_str(&format!("{title}\n{rd}"));
        out.patch.push_str(&format!("{title}\n{rd}\n"));
    }
    Ok(out)
}

/// The commit's patch text (`--stat` and/or `-p`), git's mail layout.
fn diff_text(
    repo: &Repository,
    oid: &Oid,
    c: &Commit,
    stat: bool,
    patch: bool,
    no_binary: bool,
) -> CliResult<String> {
    use crate::cmd::diff_render::{DiffFormat, changes_between, render};
    let old = match c.parents.first() {
        Some(p) => crate::cmd::shared::tree_blob_map(repo, p)?,
        None => Default::default(),
    };
    let new = crate::cmd::shared::tree_blob_map(repo, oid)?;
    let changes = changes_between(&old, &new, None);
    let fmt = DiffFormat {
        stat,
        patch,
        ..Default::default()
    };
    let mut buf = Vec::new();
    render(&mut buf, repo, &fmt, &changes)?;
    let mut text = String::from_utf8_lossy(&buf).to_string();
    let _ = no_binary;
    if stat {
        // `--summary` lines follow the stat in a mail.
        let before = match c.parents.first() {
            Some(p) => mode_map(repo, &read_commit(repo, p)?.tree)?,
            None => Default::default(),
        };
        let after = mode_map(repo, &c.tree)?;
        text.push_str(&summary_lines(&before, &after));
    }
    Ok(text)
}

fn mode_map(
    repo: &Repository,
    tree: &Oid,
) -> CliResult<std::collections::BTreeMap<String, (Oid, String)>> {
    let mut out = std::collections::BTreeMap::new();
    let mut stack = vec![(String::new(), *tree)];
    while let Some((prefix, t)) = stack.pop() {
        let obj = repo.get_object(&t)?;
        for e in arks_object::Tree::decode(&obj.payload, repo.oid_kind())?.entries {
            let name = format!("{prefix}{}", e.name);
            if e.mode.is_tree() {
                stack.push((format!("{name}/"), e.oid));
            } else {
                out.insert(name, (e.oid, e.mode.as_str().to_string()));
            }
        }
    }
    Ok(out)
}

fn summary_lines(
    a: &std::collections::BTreeMap<String, (Oid, String)>,
    b: &std::collections::BTreeMap<String, (Oid, String)>,
) -> String {
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut s = String::new();
    for k in keys {
        match (a.get(k), b.get(k)) {
            (None, Some((_, m))) => s.push_str(&format!(" create mode {m} {k}\n")),
            (Some((_, m)), None) => s.push_str(&format!(" delete mode {m} {k}\n")),
            (Some((_, x)), Some((_, y))) if x != y => {
                s.push_str(&format!(" mode change {x} => {y} {k}\n"))
            }
            _ => {}
        }
    }
    s
}

/// `git patch-id`: SHA-1 of the patch with `index` and `@@` lines dropped and
/// all whitespace removed (single-file and concatenated multi-file form).
fn patch_id(repo: &Repository, oid: &Oid) -> CliResult<String> {
    let c = read_commit(repo, oid)?;
    let text = diff_text(repo, oid, &c, false, true, false)?;
    let mut acc = Vec::new();
    for l in text.lines() {
        if l.starts_with("index ") || l.starts_with("@@") {
            continue;
        }
        acc.extend(l.bytes().filter(|b| !b.is_ascii_whitespace()));
    }
    Ok(sha1_hex(&acc))
}

fn sha1_hex(bytes: &[u8]) -> String {
    arks_hash::hash_bytes(arks_hash::ObjectFormat::Sha1, bytes)
        .map(|d| d.as_bytes().iter().map(|b| format!("{b:02x}")).collect())
        .unwrap_or_default()
}

/// `base-commit:` plus one `prerequisite-patch-id:` per commit between the
/// base and the series.
fn base_section(repo: &Repository, base: &Oid, commits: &[Oid]) -> CliResult<String> {
    let mut out = format!("base-commit: {}\n", base.to_hex());
    let Some(first) = commits.first() else {
        return Ok(out);
    };
    let series: HashSet<Oid> = commits.iter().copied().collect();
    let base_set = arks_repo::ancestor_set(repo, *base)?;
    let first_c = read_commit(repo, first)?;
    let mut prereq = Vec::new();
    if let Some(p) = first_c.parents.first() {
        for (oid, c) in crate::cmd::log_format::walk(repo, &[*p], &[], false)? {
            if base_set.contains(&oid) {
                break;
            }
            if !series.contains(&oid) && c.parents.len() <= 1 {
                prereq.push(oid);
            }
        }
        if !arks_repo::ancestor_set(repo, *p)?.contains(base) {
            return Err(CliError::fatal(
                "base commit should be the ancestor of revision list",
            ));
        }
    }
    prereq.reverse();
    for p in prereq {
        out.push_str(&format!("prerequisite-patch-id: {}\n", patch_id(repo, &p)?));
    }
    Ok(out)
}

fn rfc2822(sig: &arks_object::Signature) -> String {
    crate::cmd::log_format::DateMode::Rfc2822.format(sig)
}

fn header_block(o: &Opts, ids: &(Option<String>, Option<String>, Option<String>)) -> String {
    let mut s = String::new();
    if let Some(id) = &ids.0 {
        s.push_str(&format!("Message-ID: {id}\n"));
    }
    if let Some(irt) = &ids.1 {
        s.push_str(&format!("In-Reply-To: {irt}\n"));
    }
    if let Some(r) = &ids.2 {
        s.push_str(&format!("References: {r}\n"));
    }
    let _ = o;
    s
}

fn extra_headers(o: &Opts) -> String {
    let mut s = String::new();
    for h in &o.headers {
        s.push_str(h);
        s.push('\n');
    }
    if !o.to.is_empty() {
        s.push_str(&format!("To: {}\n", o.to.join(",\n    ")));
    }
    if !o.cc.is_empty() {
        s.push_str(&format!("Cc: {}\n", o.cc.join(",\n    ")));
    }
    s
}

#[allow(clippy::too_many_arguments)]
fn render_patch(
    repo: &Repository,
    oid: &Oid,
    commit: &Commit,
    prefix: &str,
    o: &Opts,
    ids: &(Option<String>, Option<String>, Option<String>),
    file_name: &str,
    base: Option<&str>,
    reroll: &str,
) -> CliResult<String> {
    if commit.parents.len() > 1 {
        return Err(CliError::NotImplemented(
            "format-patch of merge commits is not implemented".into(),
        ));
    }
    let subject = crate::cmd::log_format::subject_line(&commit.message);
    let mut body: String = {
        let msg = commit.message.trim_end_matches('\n');
        match msg.split_once("\n\n") {
            Some((_, rest)) => format!("{}\n", rest.trim_start_matches('\n')),
            None => String::new(),
        }
    };
    if o.signoff {
        let me = committer_signature()?;
        let line = format!("Signed-off-by: {} <{}>", me.name, me.email);
        let full = crate::cmd::core::add_signoff(&format!("{subject}\n\n{body}"));
        body = full
            .split_once("\n\n")
            .map(|(_, b)| b.trim_start_matches('\n').to_string())
            .unwrap_or_else(|| format!("{line}\n"));
    }
    let author = format!("{} <{}>", commit.author.name, commit.author.email);
    let sender = o.from.clone().unwrap_or_else(|| author.clone());
    let in_body_from = o.from.is_some() && (sender != author || o.force_in_body_from);
    let mut out = String::new();
    let from_oid = if o.zero_commit {
        "0".repeat(oid.to_hex().len())
    } else {
        oid.to_hex()
    };
    out.push_str(&format!("From {from_oid} Mon Sep 17 00:00:00 2001\n"));
    out.push_str(&header_block(o, ids));
    out.push_str(&mail_header::address("From", &sender, o.encode_headers));
    out.push_str(&format!("Date: {}\n", rfc2822(&commit.author)));
    out.push_str(&mail_header::subject(prefix, &subject, o.encode_headers));
    let mut text = String::new();
    if in_body_from {
        text.push_str(&format!("From: {author}\n\n"));
    }
    text.push_str(&body);
    // An 8-bit message (subject, in-body From, body; not the diff) declares
    // its charset; a MIME multipart (`--attach`) declares it per part.
    if o.attach.is_none() && !(subject.is_ascii() && text.is_ascii()) {
        out.push_str(mail_header::MIME_8BIT);
    }
    out.push_str(&extra_headers(o));
    if o.no_stat && reroll.is_empty() {
        text.push('\n');
    } else if o.no_stat {
        text.push_str("---\n");
        text.push_str(reroll);
    } else {
        text.push_str("---\n");
        text.push_str(reroll);
        text.push_str(&diff_text(repo, oid, commit, true, false, o.no_binary)?);
        text.push('\n');
    }
    let patch = diff_text(repo, oid, commit, false, true, o.no_binary)?;
    let mut tail = String::new();
    if let Some(b) = base {
        tail.push_str(&format!("\n{b}"));
    }
    if let Some(sig) = &o.signature {
        tail.push_str(&format!("-- \n{}\n\n", sig.trim_end_matches('\n')));
    }
    match &o.attach {
        None => {
            out.push('\n');
            out.push_str(&text);
            out.push_str(&patch);
            out.push_str(&tail);
        }
        Some(boundary) => {
            let b = boundary
                .clone()
                .map(|x| format!("------------{x}"))
                .unwrap_or_else(|| format!("------------{SIGNATURE_VERSION}"));
            out.push_str("MIME-Version: 1.0\n");
            out.push_str(&format!(
                "Content-Type: multipart/mixed; boundary=\"{b}\"\n\n"
            ));
            out.push_str("This is a multi-part message in MIME format.\n");
            out.push_str(&format!(
                "--{b}\nContent-Type: text/plain; charset=UTF-8; format=fixed\nContent-Transfer-Encoding: 8bit\n\n"
            ));
            out.push_str(&text);
            out.push_str(&format!(
                "\n--{b}\nContent-Type: text/x-patch; name=\"{file_name}\"\nContent-Transfer-Encoding: 8bit\nContent-Disposition: {}; filename=\"{file_name}\"\n\n",
                if o.inline { "inline" } else { "attachment" }
            ));
            out.push_str(&patch);
            // A MIME message carries no `-- ` signature.
            out.push_str(&format!("\n--{b}--\n\n\n"));
            if let Some(b) = base {
                out.push_str(b);
            }
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn render_cover(
    repo: &Repository,
    commits: &[Oid],
    o: &Opts,
    numbered: bool,
    version: &str,
    ids: &(Option<String>, Option<String>, Option<String>),
    base: Option<&str>,
    reroll: &str,
) -> CliResult<String> {
    let tip = *commits.last().unwrap_or(&commits[0]);
    let me = committer_signature()?;
    let total = commits.len() + o.start_number - 1;
    let mut out = format!("From {} Mon Sep 17 00:00:00 2001\n", tip.to_hex());
    out.push_str(&header_block(o, ids));
    // The cover letter is from the committer even with `--from`; the name
    // is quoted as a phrase but never Q-encoded.
    out.push_str(&mail_header::address(
        "From",
        &format!("{} <{}>", me.name, me.email),
        false,
    ));
    out.push_str(&format!("Date: {}\n", rfc2822(&me)));
    let (mut subject, mut blurb) = (
        "*** SUBJECT HERE ***".to_string(),
        "*** BLURB HERE ***".to_string(),
    );
    if let Some(d) = &o.description {
        let d = d.trim_end_matches('\n');
        let (first, rest) = match d.split_once("\n\n") {
            Some((a, b)) => (a.to_string(), b.to_string()),
            None => (d.to_string(), String::new()),
        };
        match o.cover_from_description.as_str() {
            "none" => blurb = d.to_string(),
            "subject" => {
                subject = first;
                blurb = rest;
            }
            "auto" if first.len() <= 100 => {
                subject = first;
                blurb = rest;
            }
            _ => blurb = d.to_string(),
        }
    }
    let prefix = if numbered {
        let w = total.to_string().len();
        format!("[{}{version} {:0w$}/{total}]", o.subject_prefix, 0)
    } else {
        format!("[{}{version}]", o.subject_prefix)
    };
    out.push_str(&format!("Subject: {prefix} {subject}\n"));
    // The cover declares 8-bit text when a commit of the series (identities
    // or message) is not ASCII; its own subject and blurb do not count.
    let mut eight_bit = false;
    for c in commits {
        let c = read_commit(repo, c)?;
        eight_bit |= [&c.author, &c.committer]
            .iter()
            .any(|s| !(s.name.is_ascii() && s.email.is_ascii()))
            || !c.message.is_ascii();
    }
    if eight_bit {
        out.push_str(mail_header::MIME_8BIT);
    }
    out.push_str(&extra_headers(o));
    out.push('\n');
    if blurb.is_empty() {
        out.push('\n');
    } else {
        out.push_str(&blurb);
        out.push_str("\n\n");
    }
    // Shortlog of the series by author.
    let mut by_author: Vec<(String, Vec<String>)> = Vec::new();
    for c in commits {
        let commit = read_commit(repo, c)?;
        let name = commit.author.name.clone();
        let subj = crate::cmd::log_format::subject_line(&commit.message);
        match by_author.iter_mut().find(|(n, _)| *n == name) {
            Some((_, v)) => v.push(subj),
            None => by_author.push((name, vec![subj])),
        }
    }
    by_author.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, subjects) in &by_author {
        out.push_str(&format!("{name} ({}):\n", subjects.len()));
        for s in subjects {
            // `git shortlog` layout: width 72, indents 2 and 4.
            out.push_str(&wrap::wrap(s, wrap::Lead::Indent(2), 4, 72));
            out.push('\n');
        }
        out.push('\n');
    }
    // Diffstat of the whole series with --summary lines.
    let first = read_commit(repo, &commits[0])?;
    let from_tree = match first.parents.first() {
        Some(p) => crate::cmd::shared::tree_blob_map(repo, p)?,
        None => Default::default(),
    };
    let to_tree = crate::cmd::shared::tree_blob_map(repo, &tip)?;
    let changes = crate::cmd::diff_render::changes_between(&from_tree, &to_tree, None);
    let fmt = crate::cmd::diff_render::DiffFormat {
        stat: true,
        ..Default::default()
    };
    let mut buf = Vec::new();
    crate::cmd::diff_render::render(&mut buf, repo, &fmt, &changes)?;
    out.push_str(&String::from_utf8_lossy(&buf));
    let before = match first.parents.first() {
        Some(p) => mode_map(repo, &read_commit(repo, p)?.tree)?,
        None => Default::default(),
    };
    let after = mode_map(repo, &read_commit(repo, &tip)?.tree)?;
    out.push_str(&summary_lines(&before, &after));
    out.push('\n');
    out.push_str(reroll);
    if let Some(b) = base {
        // A blank line separates the base information from what precedes.
        out.push('\n');
        out.push_str(b);
    }
    if let Some(sig) = &o.signature {
        out.push_str(&format!("-- \n{}\n\n", sig.trim_end_matches('\n')));
    }
    Ok(out)
}

/// git's file-name slug: runs of characters other than letters, digits,
/// `.` and `_` become one `-`.
fn patch_slug(subject: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in subject.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' {
            slug.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "patch".to_owned()
    } else {
        slug
    }
}
