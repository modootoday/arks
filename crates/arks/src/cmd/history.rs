//! History summaries (shortlog) and reflog command.

use crate::error::{CliError, CliResult};
use crate::reflog::{resolve_reflog_name, show_reflog};
use crate::util::{open_repo, refuse_unknown_flags};
use arks_object::{Commit, ObjectKind};
use std::collections::BTreeMap;
pub fn cmd_shortlog(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let Ok(mut oid) = repo.head_oid() else {
        return Ok(());
    };
    loop {
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Commit {
            break;
        }
        let c = Commit::decode(&obj.payload)?;
        *counts.entry(c.author.name.clone()).or_default() += 1;
        match c.parents.first() {
            Some(p) => oid = *p,
            None => break,
        }
    }
    for (name, n) in counts {
        println!("{n:6}\t{name}");
    }
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
