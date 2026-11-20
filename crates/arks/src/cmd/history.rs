//! History summaries (shortlog) and reflog command.

use crate::error::{CliError, CliResult};
use crate::reflog::{resolve_reflog_name, show_reflog};
use crate::util::open_repo;
use std::collections::BTreeMap;
/// One `--group` key.
#[derive(Clone)]
enum Group {
    Author,
    Committer,
    Trailer(String),
    Format(String),
}

/// git's `strbuf_add_wrapped_text`: greedy word fill to `width`, the first
/// line indented by `i1` and the rest by `i2`.
fn wrap(text: &str, width: usize, i1: usize, i2: usize) -> String {
    let mut out = String::new();
    let mut line = " ".repeat(i1);
    let mut col = i1;
    let mut first_word = true;
    for w in text.split_whitespace() {
        if !first_word && width > 0 && col + 1 + w.len() > width {
            out.push_str(&line);
            out.push('\n');
            line = " ".repeat(i2);
            col = i2;
            first_word = true;
        }
        if !first_word {
            line.push(' ');
            col += 1;
        }
        line.push_str(w);
        col += w.len();
        first_word = false;
    }
    out.push_str(&line);
    out.push('\n');
    out
}

pub fn cmd_shortlog(args: &[String]) -> CliResult<()> {
    // Summary per author (or `--group` key) of the commits the revision
    // arguments select: `-s`, `-n`, `-e`, `-c`, `-w`, `--format`, mailmap.
    // Cite: https://git-scm.com/docs/git-shortlog
    use crate::cmd::diffcore::options::Args;
    use crate::cmd::revwalk::RevOpts;
    let args = crate::util::split_short_bundles(args, "snec", "");
    let (mut summary, mut numbered, mut email) = (false, false, false);
    let mut groups: Vec<Group> = Vec::new();
    let mut wrap_opt: Option<(usize, usize, usize)> = None;
    let mut format: Option<String> = None;
    let mut rev = RevOpts::default();
    let (head, paths): (Vec<String>, Vec<String>) = match args.iter().position(|a| a == "--") {
        Some(i) => (args[..i].to_vec(), args[i + 1..].to_vec()),
        None => (args.clone(), Vec::new()),
    };
    let mut a = Args::new(&head);
    while a.i < head.len() {
        let cur = a.cur().to_string();
        match cur.as_str() {
            "-s" | "--summary" => summary = true,
            "--no-summary" => summary = false,
            "-n" | "--numbered" => numbered = true,
            "--no-numbered" => numbered = false,
            "-e" | "--email" => email = true,
            "--no-email" => email = false,
            "-c" | "--committer" => groups.push(Group::Committer),
            "--no-committer" => groups.retain(|g| !matches!(g, Group::Committer)),
            "--no-group" => groups.clear(),
            "-w" => wrap_opt = Some((76, 6, 9)),
            x if x.starts_with("-w") => {
                let nums: Vec<usize> = x[2..]
                    .split(',')
                    .map(|n| n.parse::<usize>())
                    .collect::<Result<_, _>>()
                    .map_err(|_| CliError::Exit(129, format!("error: bad -w argument: {x}")))?;
                let d = (76, 6, 9);
                wrap_opt = Some((
                    *nums.first().unwrap_or(&d.0),
                    *nums.get(1).unwrap_or(&d.1),
                    *nums.get(2).unwrap_or(&d.2),
                ));
            }
            _ => {
                if let Some(v) = a.long_req("--group")? {
                    groups.push(match v.as_str() {
                        "author" => Group::Author,
                        "committer" => Group::Committer,
                        x if x.starts_with("trailer:") => Group::Trailer(x[8..].to_string()),
                        x if x.starts_with("format:") => Group::Format(x[7..].to_string()),
                        x => {
                            return Err(CliError::Exit(
                                129,
                                format!("error: unknown group type: {x}"),
                            ));
                        }
                    });
                } else if let Some(v) = a.long_req("--format")?.or(a.long_req("--pretty")?) {
                    format = Some(v.strip_prefix("format:").unwrap_or(&v).to_string());
                } else if rev.take(&mut a)? {
                } else if cur.starts_with('-') && cur.len() > 1 {
                    return Err(CliError::NotImplemented(format!(
                        "unsupported option: {cur}"
                    )));
                } else {
                    rev.push_rev(&cur);
                }
            }
        }
        a.i += 1;
    }
    if groups.is_empty() {
        groups.push(Group::Author);
    }
    let repo = open_repo()?;
    rev.specs = paths;
    let no_revs = rev.revs.is_empty() && !rev.has_ref_selection() && !rev.stdin;
    if no_revs && repo.head_oid().is_err() {
        return Ok(());
    }
    if no_revs && !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        // git reads `git log` output from a non-terminal stdin; an empty
        // stdin summarises nothing.
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
        if s.trim().is_empty() {
            return Ok(());
        }
        return Err(CliError::NotImplemented(
            "shortlog: summarising log text from stdin is not implemented".into(),
        ));
    }
    let tips = crate::cmd::revwalk::resolve_tips(&repo, &rev, Some("HEAD"))?;
    let list = crate::cmd::revwalk::walk(&repo, &rev, &tips)?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let mailmap =
        crate::cmd::mailmap::Mailmap::load(crate::util::work_tree(&repo).ok().as_deref(), &cfg);
    let ctx = crate::cmd::log_format::FmtCtx::default();
    let mut table: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for w in &list {
        let c = &w.commit;
        let ident = |sig: &arks_object::Signature| {
            let (n, e) = mailmap.map(&sig.name, &sig.email);
            if email { format!("{n} <{e}>") } else { n }
        };
        let mut keys: Vec<String> = Vec::new();
        for g in &groups {
            match g {
                Group::Author => keys.push(ident(&c.author)),
                Group::Committer => keys.push(ident(&c.committer)),
                Group::Trailer(t) => {
                    for line in c.message.lines() {
                        if let Some((k, v)) = line.split_once(':') {
                            if k.trim().eq_ignore_ascii_case(t) {
                                keys.push(v.trim().to_string());
                            }
                        }
                    }
                }
                Group::Format(f) => {
                    keys.push(crate::cmd::log_format::expand_ctx(f, &w.oid, c, &ctx))
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        let line = match &format {
            Some(f) => crate::cmd::log_format::expand_ctx(f, &w.oid, c, &ctx),
            None => crate::cmd::log_format::subject_line(&c.message),
        };
        for k in keys {
            if seen.insert(k.clone()) {
                table.entry(k).or_default().push(line.clone());
            }
        }
    }
    let mut rows: Vec<(String, Vec<String>)> = table.into_iter().collect();
    if numbered {
        rows.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    }
    let mut out = String::new();
    for (name, subjects) in rows {
        if summary {
            out.push_str(&format!("{:6}\t{name}\n", subjects.len()));
        } else {
            out.push_str(&format!("{name} ({}):\n", subjects.len()));
            for s in subjects.iter().rev() {
                match wrap_opt {
                    Some((w, i1, i2)) => out.push_str(&wrap(s, w, i1, i2)),
                    None => out.push_str(&format!("      {s}\n")),
                }
            }
            out.push('\n');
        }
    }
    print!("{out}");
    Ok(())
}

pub fn cmd_reflog(args: &[String]) -> CliResult<()> {
    if args.iter().any(|a| a == "expire") {
        // MISC-014 (M20 Cx-4): reflog expire under contention refuse-closed.
        // Cite: https://git-scm.com/docs/git-reflog
        let repo = open_repo()?;
        if let Err(CliError::Msg(m)) = crate::reflog::refuse_if_reflog_locked(&repo, "HEAD") {
            return Err(CliError::Msg(format!(
                "{m}; MISC-014 — reflog expire under logs lock refuse (no concurrent expire dialect)"
            )));
        }
        return Err(CliError::Msg(
            "reflog expire refused (MISC-014 — concurrent expire vs append contention; coordinate single maintainer; lease ≠ skip)".into(),
        ));
    }
    if args.iter().any(|a| a == "delete" || a == "exists") {
        return Err(CliError::NotImplemented(
            "reflog delete/exists not claimed".into(),
        ));
    }
    // `reflog [show] [-n <n>|-<n>] [<ref>]`: newest first,
    // `<abbrev> <ref>@{<i>}: <message>` (git reflog show = log -g --oneline).
    // Cite: https://git-scm.com/docs/git-reflog
    let mut max: Option<usize> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut raw = false;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        if a == "-n" || a == "--max-count" {
            let v = args
                .get(i)
                .ok_or_else(|| CliError::fatal("switch `n' requires a value"))?;
            i += 1;
            max = Some(
                v.parse()
                    .map_err(|_| CliError::fatal(format!("'{v}': not an integer")))?,
            );
        } else if let Some(v) = a.strip_prefix("--max-count=") {
            max = Some(
                v.parse()
                    .map_err(|_| CliError::fatal(format!("'{v}': not an integer")))?,
            );
        } else if a.len() > 1 && a.starts_with('-') && a[1..].bytes().all(|b| b.is_ascii_digit()) {
            max = Some(a[1..].parse().unwrap_or(0));
        } else if a == "--raw-log" {
            // arks-only: print the raw log file (historical behavior).
            raw = true;
        } else {
            rest.push(a.to_string());
        }
    }
    let repo = open_repo()?;
    let name = resolve_reflog_name(&rest)?;
    if raw {
        return show_reflog(&repo, &name);
    }
    let shown = match name.as_str() {
        "HEAD" => "HEAD".to_string(),
        n => n.strip_prefix("refs/heads/").unwrap_or(n).to_string(),
    };
    let entries = crate::reflog::read_entries(&repo, &name);
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for (idx, (_, new, msg)) in entries.iter().rev().enumerate() {
        if max.is_some_and(|m| idx >= m) {
            break;
        }
        std::io::Write::write_all(
            &mut out,
            format!(
                "{} {shown}@{{{idx}}}: {msg}\n",
                crate::cmd::shared::abbrev_hex(&repo, new, 7)
            )
            .as_bytes(),
        )?;
    }
    std::io::Write::flush(&mut out)?;
    Ok(())
}
