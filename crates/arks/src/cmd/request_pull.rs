//! `request-pull [-p] <start> <url> [<end>]`: the pull-request summary
//! (merge base, where to fetch, the end commit, an annotated tag's message,
//! shortlog, diffstat with the create/delete summary, optional patch), with
//! the "No match for commit" warning when the URL lacks the end commit.
//!
//! Primary: <https://git-scm.com/docs/git-request-pull>.

use crate::cmd::opt_parse::{Mode, Takes::*, o, parse};
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind};
use arks_oid::Oid;
use arks_repo::Repository;

const USAGE: &str = "usage: git request-pull [options] start url [end]\n\n    -p                    show patch text as well\n\n";

/// `%s (%ci)` of a commit.
fn subject_and_date(repo: &Repository, oid: &Oid) -> CliResult<String> {
    let c = Commit::decode(&repo.get_object(oid)?.payload)?;
    let subject = c.message.lines().next().unwrap_or("").to_string();
    let ts: i64 = c.committer.timestamp.to_string().parse().unwrap_or(0);
    let tz = c.committer.timezone.to_string();
    Ok(format!("{subject} ({})", iso_date(ts, &tz)))
}

/// `YYYY-MM-DD HH:MM:SS +zzzz` in the signature's own offset.
fn iso_date(ts: i64, tz: &str) -> String {
    let sign = if tz.starts_with('-') { -1 } else { 1 };
    let digits: String = tz.chars().filter(|c| c.is_ascii_digit()).collect();
    let off = digits.parse::<i64>().unwrap_or(0);
    let local = ts + sign * ((off / 100) * 3600 + (off % 100) * 60);
    let days = local.div_euclid(86400);
    let secs = local.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} {tz}",
        secs / 3600,
        (secs / 60) % 60,
        secs % 60
    )
}

fn merge_base(repo: &Repository, a: &Oid, b: &Oid) -> CliResult<Option<Oid>> {
    let anc = crate::cmd::shared::is_ancestor_set(repo, *a)?;
    // Newest common commit reachable from `b`.
    let mut best: Option<(i64, Oid)> = None;
    for c in crate::cmd::shared::is_ancestor_set(repo, *b)? {
        if anc.contains(&c) {
            let t: i64 = Commit::decode(&repo.get_object(&c)?.payload)?
                .committer
                .timestamp
                .to_string()
                .parse()
                .unwrap_or(0);
            if best.is_none_or(|(bt, _)| t > bt) {
                best = Some((t, c));
            }
        }
    }
    Ok(best.map(|b| b.1))
}

pub fn cmd_request_pull(args: &[String]) -> CliResult<()> {
    let p = parse(
        "request-pull",
        args,
        &[o("-p", 'p', No, false)],
        Mode::default(),
    )?;
    let operands = p.operands();
    if operands.len() < 2 || operands.len() > 3 {
        print!("{USAGE}");
        return Err(CliError::silent(1));
    }
    let (start, url_arg) = (&operands[0], &operands[1]);
    let end_arg = operands.get(2).cloned();
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let url = crate::cmd::fetch::configured_source(&cfg, url_arg)
        .map(|s| s.url)
        .unwrap_or_else(|| url_arg.clone());
    let end_name = end_arg.clone().unwrap_or_else(|| "HEAD".into());
    let end_obj = resolve_rev(&repo, &end_name)
        .map_err(|_| CliError::fatal(format!("Not a valid revision: {end_name}")))?;
    let end = crate::cmd::remote_local::peel(&repo, &end_obj);
    let start_oid = crate::cmd::remote_local::peel(
        &repo,
        &resolve_rev(&repo, start)
            .map_err(|_| CliError::fatal(format!("Not a valid revision: {start}")))?,
    );
    let Some(base) = merge_base(&repo, &start_oid, &end)? else {
        return Err(CliError::fatal(format!(
            "No commits in common between {start} and {end_name}"
        )));
    };

    // Which ref at the URL carries the end commit.
    let remote_refs: Vec<(String, Oid)> = arks_remote::ls_remote(&url)
        .map(|v| v.into_iter().map(|r| (r.name, r.oid)).collect())
        .unwrap_or_default();
    let is_tag = repo
        .get_object(&end_obj)
        .is_ok_and(|o| o.kind == ObjectKind::Tag);
    let (found, shown_ref) = match &end_arg {
        None => (remote_refs.iter().any(|(_, o)| *o == end), String::new()),
        Some(e) => {
            let cands = [
                format!("refs/heads/{e}"),
                format!("refs/tags/{e}"),
                e.clone(),
            ];
            match cands.iter().find(|c| {
                remote_refs
                    .iter()
                    .any(|(n, o)| n == *c && (*o == end_obj || *o == end))
            }) {
                Some(n) => (
                    true,
                    n.strip_prefix("refs/heads/")
                        .map(String::from)
                        .or_else(|| n.strip_prefix("refs/").map(String::from))
                        .unwrap_or_else(|| n.clone()),
                ),
                None => (false, e.clone()),
            }
        }
    };
    let mut out = String::new();
    out.push_str(&format!(
        "The following changes since commit {}:\n\n  {}\n\n",
        base.to_hex(),
        subject_and_date(&repo, &base)?
    ));
    out.push_str(&format!(
        "are available in the Git repository at:\n\n  {url} {shown_ref}\n\n"
    ));
    out.push_str(&format!(
        "for you to fetch changes up to {}:\n\n  {}\n\n",
        end.to_hex(),
        subject_and_date(&repo, &end)?
    ));
    let rule = "----------------------------------------------------------------\n";
    if is_tag {
        let payload = repo.get_object(&end_obj)?.payload;
        let text = String::from_utf8_lossy(&payload);
        let msg = text.split_once("\n\n").map(|(_, m)| m).unwrap_or("");
        out.push_str(rule);
        out.push_str(msg.trim_end_matches('\n'));
        out.push_str("\n\n");
    }
    out.push_str(rule);
    print!("{out}");
    use std::io::Write;
    std::io::stdout().flush()?;
    crate::cmd::cmd_shortlog(&[format!("^{}", base.to_hex()), end.to_hex()])?;
    crate::cmd::pull::print_diffstat(&repo, &base, &end)?;
    if p.on("-p") {
        println!();
        std::io::stdout().flush()?;
        crate::cmd::cmd_diff(&["-p".into(), base.to_hex(), end.to_hex()])?;
    }
    if !found {
        eprintln!(
            "warn: No match for commit {} found at {url_arg}",
            end.to_hex()
        );
        eprintln!("warn: Are you sure you pushed '{end_name}' there?");
        return Err(CliError::silent(1));
    }
    Ok(())
}
