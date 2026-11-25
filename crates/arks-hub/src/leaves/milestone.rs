//! `arks milestone …` — hub-native milestone client against the arks forge.
//!
//! gh has no milestone command; the argv is arks-defined in gh style
//! ([[sot:root/arks-hub-gh-command-matrix]] rule 12a). A milestone is named by its
//! number or its exact title. Never spawns git/gh.

use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, n, row, s, usage, web};
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const HELP: &str = "\
arks milestone — repository milestones on the arks forge (hub-native; arks-defined gh-style argv)

USAGE:
  arks milestone list   [-R repo] [--state open|closed|all] [-L n] [--json fields] [-w]
  arks milestone create <title> [-d description] [-R repo]
  arks milestone edit   <milestone> [--title t] [-d description] [--state open|closed] [-R repo]
  arks milestone close  <milestone> [-R repo]
  arks milestone delete <milestone> [--yes] [-R repo]

<milestone> is a milestone number or its exact title.";

const FIELDS: &[&str] = &["description", "number", "state", "title", "url"];
const R: &[&str] = &["-R", "--repo"];

fn vals(extra: &[&'static str]) -> Vec<&'static str> {
    let mut v: Vec<&str> = R.to_vec();
    v.extend_from_slice(extra);
    v
}

fn state_arg(v: &str, all: bool) -> Result<&str, Fail> {
    match v {
        "open" | "closed" => Ok(v),
        "all" if all => Ok(v),
        _ => usage(format!(
            "invalid argument {v:?} for \"--state\" flag: valid values are {{{}}}",
            if all {
                "open|closed|all"
            } else {
                "open|closed"
            }
        )),
    }
}

/// Resolve a number or an exact title to the forge milestone object.
fn find(ctx: &Ctx, o: &str, r: &str, key: &str) -> Result<Value, Fail> {
    if let Ok(num) = key.parse::<u64>() {
        return ctx
            .api("GET", &format!("/repos/{o}/{r}/milestones/{num}"), None)
            .map_err(|e| match e {
                Fail::Api(m) if m.contains("404") => {
                    Fail::Api(format!("milestone {key:?} not found in {o}/{r}"))
                }
                other => other,
            });
    }
    let all = ctx.get(&format!("/repos/{o}/{r}/milestones?state=all"))?;
    arr(&all)
        .iter()
        .find(|m| s(m, "title") == key)
        .cloned()
        .ok_or_else(|| Fail::Api(format!("milestone {key:?} not found in {o}/{r}")))
}

fn target<'a>(a: &Args<'a>, verb: &str) -> Result<&'a str, Fail> {
    match a.positionals().as_slice() {
        [one] => Ok(one),
        [] => usage(format!(
            "cannot {verb} milestone: milestone argument required"
        )),
        _ => usage(format!(
            "cannot {verb} milestone: expected one milestone argument"
        )),
    }
}

fn web_url(ctx: &Ctx, o: &str, r: &str, m: &Value) -> String {
    ctx.web_url(&format!("/{o}/{r}/milestone/{}", n(m, "number")))
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "list" | "ls" => {
            let a = Args::parse(
                rest,
                &[vals(&["-L", "--limit", "--state"]), EXPORT.to_vec()].concat(),
                &["-w", "--web"],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let state = state_arg(a.get(&["--state"]).unwrap_or("open"), true)?;
            let limit = a.limit(30)?;
            if a.has(&["-w", "--web"]) {
                return web(out, &ctx.web_url(&format!("/{o}/{r}/milestones")));
            }
            let mut items = ctx
                .get(&format!("/repos/{o}/{r}/milestones?state={state}"))?
                .as_array()
                .cloned()
                .unwrap_or_default();
            items.sort_by_key(|m| n(m, "number"));
            items.truncate(limit);
            let rows: Vec<Map<String, Value>> = items
                .iter()
                .map(|m| {
                    let mut x = Map::new();
                    x.insert("description".into(), m["description"].clone());
                    x.insert("number".into(), m["number"].clone());
                    x.insert("state".into(), m["state"].clone());
                    x.insert("title".into(), m["title"].clone());
                    x.insert("url".into(), json!(web_url(ctx, &o, &r, m)));
                    x
                })
                .collect();
            if json_out(out, &a, &rows, FIELDS, false)? {
                return Ok(());
            }
            if items.is_empty() {
                eprintln!("no milestones found in {o}/{r}");
            }
            for m in &items {
                row(
                    out,
                    &[
                        &n(m, "number").to_string(),
                        s(m, "title"),
                        s(m, "state"),
                        s(m, "description"),
                    ],
                )?;
            }
            Ok(())
        }
        "create" => {
            let a = Args::parse(rest, &vals(&["--title", "-d", "--description"]), &[])?;
            let (o, r) = ctx.repo(&a)?;
            let pos = a.positionals();
            let title = match (a.get(&["--title"]), pos.as_slice()) {
                (Some(t), []) => t,
                (None, [t]) => t,
                (None, []) => {
                    return usage("cannot create milestone: title argument required");
                }
                _ => return usage("cannot create milestone: give the title once"),
            };
            if title.trim().is_empty() {
                return usage("cannot create milestone: title must not be empty");
            }
            let body = json!({
                "title": title,
                "description": a.get(&["-d", "--description"]).unwrap_or(""),
            });
            let m = ctx.api("POST", &format!("/repos/{o}/{r}/milestones"), Some(&body))?;
            eprintln!(
                "✓ Created milestone #{} \"{}\" in {o}/{r}",
                n(&m, "number"),
                s(&m, "title")
            );
            writeln!(out, "{}", web_url(ctx, &o, &r, &m))?;
            Ok(())
        }
        "edit" => {
            let a = Args::parse(
                rest,
                &vals(&["--title", "-d", "--description", "--state"]),
                &[],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let key = target(&a, "edit")?;
            let mut body = Map::new();
            if let Some(t) = a.get(&["--title"]) {
                if t.trim().is_empty() {
                    return usage("cannot edit milestone: title must not be empty");
                }
                body.insert("title".into(), json!(t));
            }
            if let Some(d) = a.get(&["-d", "--description"]) {
                body.insert("description".into(), json!(d));
            }
            if let Some(st) = a.get(&["--state"]) {
                body.insert("state".into(), json!(state_arg(st, false)?));
            }
            if body.is_empty() {
                return usage("specify at least one of `--title`, `--description`, or `--state`");
            }
            let m = find(ctx, &o, &r, key)?;
            let m = ctx.api(
                "PATCH",
                &format!("/repos/{o}/{r}/milestones/{}", n(&m, "number")),
                Some(&Value::Object(body)),
            )?;
            eprintln!("✓ Edited milestone #{} in {o}/{r}", n(&m, "number"));
            writeln!(out, "{}", web_url(ctx, &o, &r, &m))?;
            Ok(())
        }
        "close" => {
            let a = Args::parse(rest, R, &[])?;
            let (o, r) = ctx.repo(&a)?;
            let m = find(ctx, &o, &r, target(&a, "close")?)?;
            let num = n(&m, "number");
            if s(&m, "state") == "closed" {
                eprintln!("! Milestone #{num} in {o}/{r} is already closed");
                return Ok(());
            }
            ctx.api(
                "PATCH",
                &format!("/repos/{o}/{r}/milestones/{num}"),
                Some(&json!({"state": "closed"})),
            )?;
            eprintln!(
                "✓ Closed milestone #{num} \"{}\" in {o}/{r}",
                s(&m, "title")
            );
            Ok(())
        }
        "delete" => {
            let a = Args::parse(rest, R, &["--yes", "-y"])?;
            let (o, r) = ctx.repo(&a)?;
            let m = find(ctx, &o, &r, target(&a, "delete")?)?;
            let num = n(&m, "number");
            ctx.api("DELETE", &format!("/repos/{o}/{r}/milestones/{num}"), None)?;
            eprintln!(
                "✓ Deleted milestone #{num} \"{}\" from {o}/{r}",
                s(&m, "title")
            );
            Ok(())
        }
        other => usage(format!(
            "unknown command {other:?} for \"arks milestone\"\n{HELP}"
        )),
    }
}
