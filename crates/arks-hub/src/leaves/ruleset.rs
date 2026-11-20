//! `arks ruleset …` / `arks rs …` — rulesets (branch protection rules stored by
//! the arks forge): `list`, `view`, `check`. Rulesets are created with the
//! forge API (`arks api -X POST repos/{owner}/{repo}/rulesets --input ruleset.json`),
//! as with gh. Active rulesets gate `pr merge` on the forge.

use super::common::{Local, seg};
use super::{Args, Ctx, Fail, Res, arr, n, row, s, usage, web};
use serde_json::Value;
use std::io::Write;

pub(crate) const HELP: &str = "\
arks ruleset (alias rs) — rulesets on the arks forge (gh-symmetric argv; host-tested Partial)

USAGE:
  arks ruleset list [-L <n>] [-o <org>] [-p=false] [-w]
  arks ruleset view [<id>] [-o <org>] [-p=false] [-w]
  arks ruleset check [<branch>] [--default] [-w]

Create or change rulesets through the forge API, e.g.
  arks api -X POST repos/OWNER/REPO/rulesets --input ruleset.json
Active rulesets gate `pr merge` (approvals, required status checks, linear
history); push-time enforcement in the Git server is not wired.";

const R: &[&str] = &["-R", "--repo"];

fn base(ctx: &Ctx, a: &Args<'_>) -> Result<(String, String), Fail> {
    match a.get(&["-o", "--org"]) {
        Some(org) => Ok((format!("/orgs/{}/rulesets", seg(org)), org.to_string())),
        None => {
            let (o, r) = ctx.repo(a)?;
            Ok((format!("/repos/{o}/{r}/rulesets"), format!("{o}/{r}")))
        }
    }
}

fn rule_line(rule: &Value) -> String {
    let t = s(rule, "type");
    let params = rule.get("parameters").and_then(Value::as_object);
    match params {
        Some(p) if !p.is_empty() => {
            let parts: Vec<String> = p
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{k}: {}",
                        match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        }
                    )
                })
                .collect();
            format!("- {t}: [{}]", parts.join("] ["))
        }
        _ => format!("- {t}"),
    }
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    match sub {
        "list" | "ls" => {
            let a = Args::parse(
                raw,
                &[R, &["-L", "--limit", "-o", "--org"]].concat(),
                &["-p", "--parents", "-w", "--web"],
            )?;
            let (path, label) = base(ctx, &a)?;
            if a.has(&["-w", "--web"]) {
                let url = match a.get(&["-o", "--org"]) {
                    Some(org) => format!("/organizations/{org}/settings/rules"),
                    None => format!("/{label}/rules"),
                };
                return web(out, &ctx.web_url(&url));
            }
            let parents = a.bool_opt(&["-p", "--parents"]).unwrap_or(true);
            let list = ctx.get(&format!("{path}?includes_parents={parents}"))?;
            let mut items = arr(&list).to_vec();
            items.truncate(a.limit(30)?);
            if items.is_empty() {
                return Err(Fail::Api(format!("no rulesets found in {label}")));
            }
            eprintln!(
                "\nShowing {} of {} rulesets in {label}\n",
                items.len(),
                arr(&list).len()
            );
            for rs in &items {
                let full = ctx
                    .get(&format!("{}/{}", path, n(rs, "id")))
                    .unwrap_or(Value::Null);
                let rules = arr(&full["rules"]).len();
                row(
                    out,
                    &[
                        &n(rs, "id").to_string(),
                        s(rs, "name"),
                        &format!(
                            "{} ({})",
                            s(rs, "source"),
                            s(rs, "source_type").to_ascii_lowercase()
                        ),
                        s(rs, "enforcement"),
                        &rules.to_string(),
                    ],
                )?;
            }
            Ok(())
        }
        "view" => {
            let a = Args::parse(
                raw,
                &[R, &["-o", "--org"]].concat(),
                &["-p", "--parents", "-w", "--web"],
            )?;
            let (path, _) = base(ctx, &a)?;
            let id = a.positionals().first().copied().ok_or_else(|| {
                Fail::Usage("a ruleset ID must be provided when not running interactively".into())
            })?;
            if id.parse::<u64>().is_err() {
                return usage(format!(
                    "invalid value for ruleset ID: {id} is not an integer"
                ));
            }
            let rs = ctx.get(&format!("{path}/{id}"))?;
            if a.has(&["-w", "--web"]) {
                return web(out, rs["_links"]["html"]["href"].as_str().unwrap_or(""));
            }
            writeln!(
                out,
                "\n{}\nID: {}\nSource: {} ({})\nEnforcement: {}",
                s(&rs, "name"),
                n(&rs, "id"),
                s(&rs, "source"),
                s(&rs, "source_type"),
                s(&rs, "enforcement")
            )?;
            let bypass = arr(&rs["bypass_actors"]);
            writeln!(out, "\nBypass List")?;
            if bypass.is_empty() {
                writeln!(out, "This ruleset cannot be bypassed")?;
            }
            for b in bypass {
                writeln!(
                    out,
                    "- {} (ID: {}), mode: {}",
                    s(b, "actor_type"),
                    b["actor_id"],
                    s(b, "bypass_mode")
                )?;
            }
            writeln!(out, "\nConditions")?;
            if let Some(c) = rs["conditions"].as_object() {
                for (k, v) in c {
                    for (kk, vv) in v.as_object().into_iter().flatten() {
                        let list: Vec<&str> = arr(vv).iter().filter_map(Value::as_str).collect();
                        writeln!(out, "- {k}: [{kk}: {}]", list.join(", "))?;
                    }
                }
            }
            writeln!(out, "\nRules")?;
            let rules = arr(&rs["rules"]);
            if rules.is_empty() {
                writeln!(out, "No rules configured")?;
            }
            for rule in rules {
                writeln!(out, "{}", rule_line(rule))?;
            }
            Ok(())
        }
        "check" => {
            let a = Args::parse(raw, R, &["--default", "-w", "--web"])?;
            let (o, r) = ctx.repo(&a)?;
            let branch = match (a.positionals().first(), a.has(&["--default"])) {
                (Some(_), true) => {
                    return usage("specify only one of `--default` or a branch name");
                }
                (Some(b), false) => b.to_string(),
                (None, true) => {
                    s(&ctx.get(&format!("/repos/{o}/{r}"))?, "default_branch").to_string()
                }
                (None, false) => Local::discover().and_then(|l| l.branch()).ok_or_else(|| {
                    Fail::Usage(
                        "could not determine the current branch; pass a branch or --default".into(),
                    )
                })?,
            };
            if a.has(&["-w", "--web"]) {
                return web(
                    out,
                    &ctx.web_url(&format!("/{o}/{r}/rules/?ref=refs/heads/{branch}")),
                );
            }
            let rules = ctx.get(&format!("/repos/{o}/{r}/rules/branches/{}", seg(&branch)))?;
            let list = arr(&rules);
            writeln!(
                out,
                "{} rules apply to branch {branch} in repo {o}/{r}",
                list.len()
            )?;
            for rule in list {
                writeln!(out)?;
                writeln!(out, "{}", rule_line(rule))?;
                writeln!(
                    out,
                    "  (configured in ruleset {} from {} {})",
                    n(rule, "ruleset_id"),
                    s(rule, "ruleset_source_type").to_ascii_lowercase(),
                    s(rule, "ruleset_source")
                )?;
            }
            if !list.is_empty() {
                eprintln!(
                    "note: the arks forge enforces these rules when merging pull requests; pushes are not checked"
                );
            }
            Ok(())
        }
        other => usage(format!(
            "unknown command {other:?} for \"arks ruleset\"\n{HELP}"
        )),
    }
}
