//! `arks project …` — gh-symmetric project client (projects, fields, items).
//!
//! Projects are owner-scoped (`--owner`, default `@me`) and addressed by number.
//! `--format json` prints gh-shaped JSON (camelCase).

use super::{Args, Ctx, Fail, Res, arr, json_filtered, n, row, s, usage};
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const HELP: &str = "\
arks project — owner-scoped projects on the arks forge (gh-symmetric argv; arks-owned; host-tested Partial)

USAGE:
  arks project list [--owner <login>] [--closed] [-L <n>] [--format json]
  arks project view <number> [--owner <login>] [--format json]
  arks project create --title <title> [--owner <login>] [--format json]
  arks project edit <number> [--title <t>] [-d <desc>] [--readme <md>] [--visibility PUBLIC|PRIVATE]
  arks project delete <number> [--owner <login>]
  arks project close <number> [--undo]
  arks project copy <number> --title <t> [--source-owner <l>] [--target-owner <l>] [--drafts]
  arks project link <number> -R owner/repo     arks project unlink <number> -R owner/repo
  arks project mark-template <number> [--undo]
  arks project field-create <number> --name <n> --data-type TEXT|NUMBER|DATE|SINGLE_SELECT [--single-select-options a,b]
  arks project field-delete --id <field-id>
  arks project field-list <number> [-L <n>] [--format json]
  arks project item-add <number> --url <issue-or-pr-url>
  arks project item-create <number> --title <t> [--body <b>]
  arks project item-archive <number> --id <item-id> [--undo]
  arks project item-delete <number> --id <item-id>
  arks project item-edit --id <item-id> [--project-id <number> --field-id <id> (--text|--number|--date|--single-select-option-id <v> | --clear)] [--title <t>] [--body <b>]
  arks project item-list <number> [-L <n>] [--format json]

Teams (-T) and iteration fields are not arks forge primitives and are refused.";

const OWNER: &[&str] = &["--owner"];
const FORMAT: &[&str] = &["--format"];

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "list" | "ls" => list(ctx, rest, out),
        "view" => view(ctx, rest, out),
        "create" => create(ctx, rest, out),
        "edit" => edit(ctx, rest, out),
        "delete" => delete(ctx, rest, out),
        "close" => toggle(ctx, rest, out, "closed"),
        "mark-template" => toggle(ctx, rest, out, "template"),
        "copy" => copy(ctx, rest, out),
        "link" => link(ctx, rest, out, true),
        "unlink" => link(ctx, rest, out, false),
        "field-create" => field_create(ctx, rest, out),
        "field-delete" => field_delete(ctx, rest, out),
        "field-list" => field_list(ctx, rest, out),
        "item-add" => item_add(ctx, rest, out),
        "item-create" => item_create(ctx, rest, out),
        "item-archive" => item_archive(ctx, rest, out),
        "item-delete" => item_delete(ctx, rest, out),
        "item-edit" => item_edit(ctx, rest, out),
        "item-list" => item_list(ctx, rest, out),
        other => usage(format!("unknown project command `{other}`\n{HELP}")),
    }
}

fn vals<'a>(extra: &[&'a str]) -> Vec<&'a str> {
    let mut v: Vec<&str> = vec!["--owner", "--format", "-q", "--jq", "-t", "--template"];
    v.extend_from_slice(extra);
    v
}

fn owner(a: &Args<'_>) -> String {
    a.get(OWNER).unwrap_or("@me").to_string()
}

fn json_mode(a: &Args<'_>) -> Result<bool, Fail> {
    match a.get(FORMAT) {
        None if a.get(&["-q", "--jq", "-t", "--template"]).is_some() => {
            usage("cannot use `--jq` or `--template` without `--format json`")
        }
        None => Ok(false),
        Some("json") => Ok(true),
        Some(f) => usage(format!("invalid format {f:?} (json)")),
    }
}

fn number(a: &Args<'_>) -> Result<u64, Fail> {
    let p = a
        .positionals()
        .first()
        .copied()
        .ok_or_else(|| Fail::Usage("project number required".into()))?;
    p.parse()
        .map_err(|_| Fail::Usage(format!("invalid project number: {p}")))
}

fn base(owner: &str, number: u64) -> String {
    format!("/users/{owner}/projectsV2/{number}")
}

fn gh_project(ctx: &Ctx, p: &Value) -> Value {
    let login = p
        .pointer("/owner/login")
        .and_then(Value::as_str)
        .unwrap_or("");
    json!({
        "number": n(p, "number"),
        "url": ctx.url(&base(login, n(p, "number"))),
        "shortDescription": s(p, "short_description"),
        "public": p.get("public").and_then(Value::as_bool).unwrap_or(false),
        "closed": p.get("closed").and_then(Value::as_bool).unwrap_or(false),
        "template": p.get("template").and_then(Value::as_bool).unwrap_or(false),
        "title": s(p, "title"),
        "id": n(p, "number").to_string(),
        "readme": s(p, "readme"),
        "items": {"totalCount": n(p, "items_count")},
        "fields": {"totalCount": n(p, "fields_count")},
        "owner": {"type": "User", "login": login},
        "linkedRepositories": p.get("linked_repositories").cloned().unwrap_or(json!([])),
    })
}

fn print_project_url(ctx: &Ctx, out: &mut dyn Write, a: &Args<'_>, p: &Value) -> Res {
    if json_mode(a)? {
        return json_filtered(out, a, &gh_project(ctx, p));
    }
    let login = p
        .pointer("/owner/login")
        .and_then(Value::as_str)
        .unwrap_or("");
    writeln!(out, "{}", ctx.url(&base(login, n(p, "number"))))?;
    Ok(())
}

fn list(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["-L", "--limit"]), &["--closed", "-w", "--web"])?;
    let json = json_mode(&a)?;
    let limit = a.limit(30)?;
    let state = if a.has(&["--closed"]) { "all" } else { "open" };
    let all = ctx.api(
        "GET",
        &format!("/users/{}/projectsV2?state={state}", owner(&a)),
        None,
    )?;
    let rows: Vec<Value> = arr(&all)
        .iter()
        .take(limit)
        .map(|p| gh_project(ctx, p))
        .collect();
    if json {
        let total = rows.len();
        return json_filtered(out, &a, &json!({"projects": rows, "totalCount": total}));
    }
    for p in &rows {
        let state = if p["closed"].as_bool().unwrap_or(false) {
            "closed"
        } else {
            "open"
        };
        row(
            out,
            &[
                &p["number"].to_string(),
                p["title"].as_str().unwrap_or(""),
                state,
                p["id"].as_str().unwrap_or(""),
            ],
        )?;
    }
    Ok(())
}

fn view(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&[]), &["-w", "--web"])?;
    let num = number(&a)?;
    let p = ctx.api("GET", &base(&owner(&a), num), None)?;
    let g = gh_project(ctx, &p);
    if json_mode(&a)? {
        return json_filtered(out, &a, &g);
    }
    if a.has(&["-w", "--web"]) {
        writeln!(out, "{}", g["url"].as_str().unwrap_or(""))?;
        return Ok(());
    }
    let vis = if g["public"].as_bool().unwrap_or(false) {
        "PUBLIC"
    } else {
        "PRIVATE"
    };
    let linked: Vec<&str> = arr(&g["linkedRepositories"])
        .iter()
        .filter_map(Value::as_str)
        .collect();
    writeln!(out, "title:\t{}", g["title"].as_str().unwrap_or(""))?;
    writeln!(out, "number:\t{num}")?;
    writeln!(out, "url:\t{}", g["url"].as_str().unwrap_or(""))?;
    writeln!(
        out,
        "owner:\t{}",
        g["owner"]["login"].as_str().unwrap_or("")
    )?;
    writeln!(out, "closed:\t{}", g["closed"])?;
    writeln!(out, "visibility:\t{vis}")?;
    writeln!(out, "template:\t{}", g["template"])?;
    writeln!(out, "items:\t{}", g["items"]["totalCount"])?;
    writeln!(out, "fields:\t{}", g["fields"]["totalCount"])?;
    writeln!(out, "repositories:\t{}", linked.join(", "))?;
    writeln!(
        out,
        "description:\t{}",
        g["shortDescription"].as_str().unwrap_or("")
    )?;
    writeln!(out, "--")?;
    writeln!(out, "{}", g["readme"].as_str().unwrap_or(""))?;
    Ok(())
}

fn create(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["--title"]), &[])?;
    let Some(title) = a.get(&["--title"]).filter(|t| !t.trim().is_empty()) else {
        return usage("--title required");
    };
    let p = ctx.api(
        "POST",
        &format!("/users/{}/projectsV2", owner(&a)),
        Some(&json!({"title": title})),
    )?;
    print_project_url(ctx, out, &a, &p)
}

fn edit(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&["--title", "-d", "--description", "--readme", "--visibility"]),
        &[],
    )?;
    let num = number(&a)?;
    let mut patch = Map::new();
    if let Some(t) = a.get(&["--title"]) {
        patch.insert("title".into(), json!(t));
    }
    if let Some(d) = a.get(&["-d", "--description"]) {
        patch.insert("short_description".into(), json!(d));
    }
    if let Some(r) = a.get(&["--readme"]) {
        patch.insert("readme".into(), json!(r));
    }
    if let Some(v) = a.get(&["--visibility"]) {
        let public = match v.to_ascii_uppercase().as_str() {
            "PUBLIC" => true,
            "PRIVATE" => false,
            _ => return usage(format!("invalid visibility {v:?} (PUBLIC|PRIVATE)")),
        };
        patch.insert("public".into(), json!(public));
    }
    if patch.is_empty() {
        return usage("nothing to edit: pass --title, --description, --readme or --visibility");
    }
    let p = ctx.api("PATCH", &base(&owner(&a), num), Some(&Value::Object(patch)))?;
    print_project_url(ctx, out, &a, &p)
}

fn delete(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&[]), &[])?;
    let num = number(&a)?;
    let p = ctx.api("DELETE", &base(&owner(&a), num), None)?;
    if json_mode(&a)? {
        return json_filtered(out, &a, &gh_project(ctx, &p));
    }
    writeln!(out, "Deleted project {num}")?;
    Ok(())
}

/// `close` / `mark-template` (with `--undo`).
fn toggle(ctx: &Ctx, raw: &[String], out: &mut dyn Write, key: &str) -> Res {
    let a = Args::parse(raw, &vals(&[]), &["--undo"])?;
    let num = number(&a)?;
    let p = ctx.api(
        "PATCH",
        &base(&owner(&a), num),
        Some(&json!({key: !a.has(&["--undo"])})),
    )?;
    print_project_url(ctx, out, &a, &p)
}

fn copy(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&["--source-owner", "--target-owner", "--title"]),
        &["--drafts"],
    )?;
    let num = number(&a)?;
    let Some(title) = a.get(&["--title"]).filter(|t| !t.trim().is_empty()) else {
        return usage("--title required");
    };
    let src = a.get(&["--source-owner"]).unwrap_or("@me");
    let target = a.get(&["--target-owner"]).unwrap_or("@me");
    let p = ctx.api(
        "POST",
        &format!("{}/copy", base(src, num)),
        Some(&json!({
            "owner": target,
            "title": title,
            "include_draft_issues": a.has(&["--drafts"]),
        })),
    )?;
    print_project_url(ctx, out, &a, &p)
}

fn link(ctx: &Ctx, raw: &[String], out: &mut dyn Write, on: bool) -> Res {
    let a = Args::parse(raw, &vals(&["-R", "--repo", "-T", "--team"]), &[])?;
    if a.get(&["-T", "--team"]).is_some() {
        return usage("teams are not an arks forge primitive; link a repository with -R");
    }
    let num = number(&a)?;
    let (o, r) = ctx.repo(&a)?;
    let method = if on { "PUT" } else { "DELETE" };
    ctx.api(
        method,
        &format!("{}/repositories/{o}/{r}", base(&owner(&a), num)),
        None,
    )?;
    let verb = if on { "Linked" } else { "Unlinked" };
    writeln!(
        out,
        "{verb} '{o}/{r}' {} project #{num}",
        if on { "to" } else { "from" }
    )?;
    Ok(())
}

fn gh_field(f: &Value) -> Value {
    let dt = s(f, "data_type");
    let kind = if dt == "SINGLE_SELECT" {
        "ProjectV2SingleSelectField"
    } else {
        "ProjectV2Field"
    };
    let mut v =
        json!({"id": n(f, "id").to_string(), "name": s(f, "name"), "type": kind, "dataType": dt});
    if dt == "SINGLE_SELECT" {
        v["options"] = f.get("options").cloned().unwrap_or(json!([]));
    }
    v
}

fn field_create(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&["--name", "--data-type", "--single-select-options"]),
        &[],
    )?;
    let num = number(&a)?;
    let Some(name) = a.get(&["--name"]) else {
        return usage("--name required");
    };
    let Some(dt) = a.get(&["--data-type"]) else {
        return usage("--data-type required (TEXT|NUMBER|DATE|SINGLE_SELECT)");
    };
    let dt = dt.to_ascii_uppercase();
    if !crate::forge::project::FIELD_TYPES.contains(&dt.as_str()) {
        return usage(format!(
            "invalid data type {dt:?} (TEXT|NUMBER|DATE|SINGLE_SELECT)"
        ));
    }
    let options: Vec<&str> = a
        .get(&["--single-select-options"])
        .map(|o| {
            o.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if dt == "SINGLE_SELECT" && options.is_empty() {
        return usage("--single-select-options required for SINGLE_SELECT");
    }
    let f = ctx.api(
        "POST",
        &format!("{}/fields", base(&owner(&a), num)),
        Some(&json!({"name": name, "data_type": dt, "options": options})),
    )?;
    if json_mode(&a)? {
        return json_filtered(out, &a, &gh_field(&f));
    }
    row(
        out,
        &[&n(&f, "id").to_string(), s(&f, "name"), s(&f, "data_type")],
    )
}

fn field_delete(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["--id"]), &[])?;
    let Some(id) = a.get(&["--id"]) else {
        return usage("--id required");
    };
    json_mode(&a)?;
    ctx.api("DELETE", &format!("/projectsV2/fields/{id}"), None)?;
    writeln!(out, "Deleted field {id}")?;
    Ok(())
}

fn field_list(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["-L", "--limit"]), &[])?;
    let num = number(&a)?;
    let limit = a.limit(30)?;
    let all = ctx.api("GET", &format!("{}/fields", base(&owner(&a), num)), None)?;
    let rows: Vec<Value> = arr(&all).iter().take(limit).map(gh_field).collect();
    if json_mode(&a)? {
        let total = rows.len();
        return json_filtered(out, &a, &json!({"fields": rows, "totalCount": total}));
    }
    for f in &rows {
        row(
            out,
            &[
                f["name"].as_str().unwrap_or(""),
                f["dataType"].as_str().unwrap_or(""),
                f["id"].as_str().unwrap_or(""),
            ],
        )?;
    }
    Ok(())
}

fn gh_item(i: &Value) -> Value {
    let mut v = json!({
        "id": n(i, "id").to_string(),
        "title": s(i, "title"),
        "archived": i.get("archived").and_then(Value::as_bool).unwrap_or(false),
        "content": {
            "type": s(i, "content_type"),
            "title": s(i, "title"),
            "body": s(i, "body"),
            "url": s(i, "url"),
        },
    });
    if let Some(fv) = i.get("field_values").and_then(Value::as_object) {
        for (name, val) in fv {
            v[name.to_ascii_lowercase()] = val.clone();
        }
    }
    v
}

/// Projects filter syntax subset: `field:value` (comma = any of), `-field:value`,
/// `no:field`, `is:archived|open`, and free text matched against the title.
fn item_matches(i: &Value, query: &str) -> bool {
    let text = |v: &Value| match v {
        Value::String(s) => s.to_ascii_lowercase(),
        Value::Null => String::new(),
        other => other.to_string().to_ascii_lowercase(),
    };
    for tok in query.split_whitespace() {
        let (neg, tok) = match tok.strip_prefix('-') {
            Some(t) => (true, t),
            None => (false, tok),
        };
        let hit = match tok.split_once(':') {
            Some(("no", f)) => text(&i[f.to_ascii_lowercase()]).is_empty(),
            Some(("is", "archived")) => i["archived"].as_bool() == Some(true),
            Some(("is", "open")) => i["archived"].as_bool() != Some(true),
            Some(("is", kind)) => i["content"]["type"]
                .as_str()
                .is_some_and(|t| t.eq_ignore_ascii_case(kind)),
            Some(("title", v)) => text(&i["title"]).contains(&v.to_ascii_lowercase()),
            Some((f, v)) => {
                let have = text(&i[f.to_ascii_lowercase()]);
                v.split(',')
                    .any(|x| have == x.trim_matches('"').to_ascii_lowercase())
            }
            None => text(&i["title"]).contains(&tok.to_ascii_lowercase()),
        };
        if hit == neg {
            return false;
        }
    }
    true
}

fn print_item(out: &mut dyn Write, a: &Args<'_>, verb: &str, i: &Value) -> Res {
    if json_mode(a)? {
        return json_filtered(out, a, &gh_item(i));
    }
    writeln!(out, "{verb} item {}", n(i, "id"))?;
    Ok(())
}

fn item_add(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["--url"]), &[])?;
    let num = number(&a)?;
    let Some(url) = a.get(&["--url"]) else {
        return usage("--url required");
    };
    let i = ctx.api(
        "POST",
        &format!("{}/items", base(&owner(&a), num)),
        Some(&json!({"url": url})),
    )?;
    print_item(out, &a, "Added", &i)
}

fn item_create(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["--title", "--body"]), &[])?;
    let num = number(&a)?;
    let Some(title) = a.get(&["--title"]).filter(|t| !t.trim().is_empty()) else {
        return usage("--title required");
    };
    let i = ctx.api(
        "POST",
        &format!("{}/items", base(&owner(&a), num)),
        Some(&json!({"title": title, "body": a.get(&["--body"]).unwrap_or("")})),
    )?;
    print_item(out, &a, "Created", &i)
}

fn item_id(a: &Args<'_>) -> Result<String, Fail> {
    a.get(&["--id"])
        .map(str::to_string)
        .ok_or_else(|| Fail::Usage("--id required".into()))
}

fn item_archive(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["--id"]), &["--undo"])?;
    let num = number(&a)?;
    let id = item_id(&a)?;
    let undo = a.has(&["--undo"]);
    let i = ctx.api(
        "PATCH",
        &format!("{}/items/{id}", base(&owner(&a), num)),
        Some(&json!({"archived": !undo})),
    )?;
    print_item(out, &a, if undo { "Unarchived" } else { "Archived" }, &i)
}

fn item_delete(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["--id"]), &[])?;
    let num = number(&a)?;
    let id = item_id(&a)?;
    json_mode(&a)?;
    ctx.api(
        "DELETE",
        &format!("{}/items/{id}", base(&owner(&a), num)),
        None,
    )?;
    writeln!(out, "Deleted item {id}")?;
    Ok(())
}

fn item_edit(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&[
            "--id",
            "--project-id",
            "--field-id",
            "--text",
            "--number",
            "--date",
            "--single-select-option-id",
            "--iteration-id",
            "--title",
            "--body",
        ]),
        &["--clear"],
    )?;
    let id = item_id(&a)?;
    if a.get(&["--iteration-id"]).is_some() {
        return usage("iteration fields are not an arks forge primitive");
    }
    let mut body = Map::new();
    if let Some(t) = a.get(&["--title"]) {
        body.insert("title".into(), json!(t));
    }
    if let Some(b) = a.get(&["--body"]) {
        body.insert("body".into(), json!(b));
    }
    let values: Vec<(&str, Value)> = [
        ("--text", a.get(&["--text"]).map(|v| json!(v))),
        ("--number", a.get(&["--number"]).map(|v| json!(v))),
        ("--date", a.get(&["--date"]).map(|v| json!(v))),
        (
            "--single-select-option-id",
            a.get(&["--single-select-option-id"]).map(|v| json!(v)),
        ),
    ]
    .into_iter()
    .filter_map(|(k, v)| v.map(|v| (k, v)))
    .collect();
    let clear = a.has(&["--clear"]);
    if values.len() > 1 || (clear && !values.is_empty()) {
        return usage(
            "pass only one of --text, --number, --date, --single-select-option-id, --clear",
        );
    }
    if !values.is_empty() || clear {
        let (Some(pid), Some(fid)) = (a.get(&["--project-id"]), a.get(&["--field-id"])) else {
            return usage("--project-id and --field-id are required to set a field value");
        };
        let pid: u64 = pid
            .parse()
            .map_err(|_| Fail::Usage(format!("invalid --project-id {pid}")))?;
        let fid: u64 = fid
            .parse()
            .map_err(|_| Fail::Usage(format!("invalid --field-id {fid}")))?;
        body.insert("project".into(), json!(pid));
        body.insert("field_id".into(), json!(fid));
        match values.into_iter().next() {
            Some((_, v)) => {
                body.insert("value".into(), v);
            }
            None => {
                body.insert("clear".into(), json!(true));
            }
        }
    }
    if body.is_empty() {
        return usage("nothing to edit: pass a field value, --clear, --title or --body");
    }
    let i = ctx.api(
        "PATCH",
        &format!("/projectsV2/items/{id}"),
        Some(&Value::Object(body)),
    )?;
    print_item(out, &a, "Edited", &i)
}

fn item_list(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["-L", "--limit", "--query"]), &[])?;
    let num = number(&a)?;
    let limit = a.limit(30)?;
    let all = ctx.api("GET", &format!("{}/items", base(&owner(&a), num)), None)?;
    let query = a.get(&["--query"]).unwrap_or("");
    let rows: Vec<Value> = arr(&all)
        .iter()
        .map(gh_item)
        .filter(|i| item_matches(i, query))
        .take(limit)
        .collect();
    if json_mode(&a)? {
        let total = rows.len();
        return json_filtered(out, &a, &json!({"items": rows, "totalCount": total}));
    }
    for i in &rows {
        row(
            out,
            &[
                i["content"]["type"].as_str().unwrap_or(""),
                i["title"].as_str().unwrap_or(""),
                i["content"]["url"].as_str().unwrap_or(""),
                i["id"].as_str().unwrap_or(""),
            ],
        )?;
    }
    Ok(())
}
