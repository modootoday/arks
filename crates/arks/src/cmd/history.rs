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
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let name = resolve_reflog_name(args)?;
    show_reflog(&repo, &name)
}
