//! `arks repo autolink …` — gh-symmetric autolink client (`create|list|view|delete`).

use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, n, row, s, usage};
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const HELP: &str = "\
arks repo autolink — repository autolink references (gh-symmetric argv; arks-owned; host-tested Partial)

USAGE:
  arks repo autolink list [-R owner/repo] [--json <fields>]
  arks repo autolink create <keyPrefix> <urlTemplate> [-n] [-R owner/repo]
  arks repo autolink view <id> [-R owner/repo] [--json <fields>]
  arks repo autolink delete <id> [--yes] [-R owner/repo]

urlTemplate must be an http(s) URL containing <num>. JSON fields: id, isAlphanumeric,
keyPrefix, urlTemplate";

const JSON_FIELDS: &[&str] = &["id", "isAlphanumeric", "keyPrefix", "urlTemplate"];

fn gh_row(v: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("id".into(), json!(n(v, "id")));
    m.insert(
        "isAlphanumeric".into(),
        json!(
            v.get("is_alphanumeric")
                .and_then(Value::as_bool)
                .unwrap_or(true)
        ),
    );
    m.insert("keyPrefix".into(), json!(s(v, "key_prefix")));
    m.insert("urlTemplate".into(), json!(s(v, "url_template")));
    m
}

fn id_arg(a: &Args<'_>) -> Result<u64, Fail> {
    let p = a
        .positionals()
        .first()
        .copied()
        .ok_or_else(|| Fail::Usage("autolink id required".into()))?;
    p.parse()
        .map_err(|_| Fail::Usage(format!("invalid autolink id: {p}")))
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "list" | "ls" => {
            let a = Args::parse(
                rest,
                &[&["-R", "--repo"][..], EXPORT].concat(),
                &["-w", "--web"],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let all = ctx.api("GET", &format!("/repos/{o}/{r}/autolinks"), None)?;
            let rows: Vec<Map<String, Value>> = arr(&all).iter().map(gh_row).collect();
            if json_out(out, &a, &rows, JSON_FIELDS, false)? {
                return Ok(());
            }
            for m in &rows {
                row(
                    out,
                    &[
                        &m["id"].to_string(),
                        m["keyPrefix"].as_str().unwrap_or(""),
                        m["urlTemplate"].as_str().unwrap_or(""),
                        &m["isAlphanumeric"].to_string(),
                    ],
                )?;
            }
            Ok(())
        }
        "create" | "new" => {
            let a = Args::parse(rest, &["-R", "--repo"], &["-n", "--numeric"])?;
            let (o, r) = ctx.repo(&a)?;
            let pos = a.positionals();
            let [prefix, template] = pos.as_slice() else {
                return usage("usage: arks repo autolink create <keyPrefix> <urlTemplate> [-n]");
            };
            crate::forge::autolink::validate(prefix, template).map_err(Fail::Usage)?;
            let body = json!({
                "key_prefix": prefix,
                "url_template": template,
                "is_alphanumeric": !a.has(&["-n", "--numeric"]),
            });
            let v = ctx.api("POST", &format!("/repos/{o}/{r}/autolinks"), Some(&body))?;
            writeln!(
                out,
                "Created repository autolink {} on {o}/{r}",
                n(&v, "id")
            )?;
            Ok(())
        }
        "view" => {
            let a = Args::parse(
                rest,
                &[&["-R", "--repo"][..], EXPORT].concat(),
                &["-w", "--web"],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let id = id_arg(&a)?;
            let v = ctx.api("GET", &format!("/repos/{o}/{r}/autolinks/{id}"), None)?;
            let m = gh_row(&v);
            if json_out(out, &a, std::slice::from_ref(&m), JSON_FIELDS, true)? {
                return Ok(());
            }
            writeln!(out, "Autolink in {o}/{r}\n")?;
            writeln!(out, "ID: {id}")?;
            writeln!(out, "Key Prefix: {}", s(&v, "key_prefix"))?;
            writeln!(out, "URL Template: {}", s(&v, "url_template"))?;
            writeln!(out, "Alphanumeric: {}", m["isAlphanumeric"])?;
            Ok(())
        }
        "delete" => {
            let a = Args::parse(rest, &["-R", "--repo"], &["--yes", "-y"])?;
            let (o, r) = ctx.repo(&a)?;
            let id = id_arg(&a)?;
            ctx.api("DELETE", &format!("/repos/{o}/{r}/autolinks/{id}"), None)?;
            writeln!(out, "Autolink {id} deleted from {o}/{r}")?;
            Ok(())
        }
        other => usage(format!("unknown autolink command `{other}`\n{HELP}")),
    }
}
