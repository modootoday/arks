//! `arks discussion …` — gh-symmetric discussion client
//! (`create|list|view|edit|comment`).

use super::{Args, Ctx, Fail, Res, arr, n, print_json_fields, read_input, row, s, usage};
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const HELP: &str = "\
arks discussion — repository discussions on the arks forge (gh-symmetric argv; arks-owned; host-tested Partial)

USAGE:
  arks discussion create -t <title> [-b <body> | -F <file>] [-c <category>] [-R owner/repo]
  arks discussion list [-c <category>] [-s open|closed|all] [-A <author>] [-L <n>] [--json <fields>]
  arks discussion view <number> [--comments] [--json <fields>]
  arks discussion edit <number> [-t <title>] [-b <body> | -F <file>] [-c <category>]
  arks discussion comment <number> (-b <body> | -F <file>)

JSON fields: number, title, body, category, state, author, comments, createdAt, updatedAt, url";

const JSON_FIELDS: &[&str] = &[
    "number",
    "title",
    "body",
    "category",
    "state",
    "author",
    "comments",
    "createdAt",
    "updatedAt",
    "url",
];

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "create" | "new" => create(ctx, rest, out),
        "list" | "ls" => list(ctx, rest, out),
        "view" => view(ctx, rest, out),
        "edit" => edit(ctx, rest, out),
        "comment" => comment(ctx, rest, out),
        other => usage(format!("unknown discussion command `{other}`\n{HELP}")),
    }
}

fn gh_row(ctx: &Ctx, owner: &str, repo: &str, d: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("number".into(), json!(n(d, "number")));
    m.insert("title".into(), json!(s(d, "title")));
    m.insert("body".into(), json!(s(d, "body")));
    m.insert(
        "category".into(),
        json!({"name": d.pointer("/category/name").and_then(Value::as_str).unwrap_or("")}),
    );
    m.insert("state".into(), json!(s(d, "state").to_ascii_uppercase()));
    m.insert(
        "author".into(),
        json!({"login": d.pointer("/user/login").and_then(Value::as_str).unwrap_or("")}),
    );
    m.insert("comments".into(), json!(n(d, "comments")));
    m.insert("createdAt".into(), json!(s(d, "created_at")));
    m.insert("updatedAt".into(), json!(s(d, "updated_at")));
    m.insert(
        "url".into(),
        json!(ctx.url(&format!(
            "/repos/{owner}/{repo}/discussions/{}",
            n(d, "number")
        ))),
    );
    m
}

fn body_arg(a: &Args<'_>) -> Result<Option<String>, Fail> {
    match (a.get(&["-b", "--body"]), a.get(&["-F", "--body-file"])) {
        (Some(_), Some(_)) => usage("specify only one of --body or --body-file"),
        (Some(b), None) => Ok(Some(b.to_string())),
        (None, Some(f)) => read_input(Some(f)).map(Some),
        (None, None) => Ok(None),
    }
}

fn number_arg(a: &Args<'_>) -> Result<u64, Fail> {
    let p = a
        .positionals()
        .first()
        .copied()
        .ok_or_else(|| Fail::Usage("discussion number required".into()))?;
    p.trim_start_matches('#')
        .parse()
        .map_err(|_| Fail::Usage(format!("invalid discussion number: {p}")))
}

fn create(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[
            "-t",
            "--title",
            "-b",
            "--body",
            "-F",
            "--body-file",
            "-c",
            "--category",
            "-R",
            "--repo",
        ],
        &[],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let Some(title) = a.get(&["-t", "--title"]).filter(|t| !t.trim().is_empty()) else {
        return usage("--title required");
    };
    let mut payload = json!({"title": title, "body": body_arg(&a)?.unwrap_or_default()});
    if let Some(c) = a.get(&["-c", "--category"]) {
        payload["category"] = json!(c);
    }
    let d = ctx.api(
        "POST",
        &format!("/repos/{o}/{r}/discussions"),
        Some(&payload),
    )?;
    writeln!(
        out,
        "{}",
        ctx.url(&format!("/repos/{o}/{r}/discussions/{}", n(&d, "number")))
    )?;
    Ok(())
}

fn list(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[
            "-c",
            "--category",
            "-s",
            "--state",
            "-A",
            "--author",
            "-L",
            "--limit",
            "--json",
            "-R",
            "--repo",
        ],
        &["-w", "--web"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let state = a.get(&["-s", "--state"]).unwrap_or("open");
    if !matches!(state, "open" | "closed" | "all") {
        return usage(format!("invalid state {state:?} (open|closed|all)"));
    }
    let limit = a.limit(30)?;
    let mut path = format!("/repos/{o}/{r}/discussions?state={state}");
    if let Some(c) = a.get(&["-c", "--category"]) {
        path.push_str("&category=");
        for b in c.bytes() {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                path.push(char::from(b));
            } else {
                path.push_str(&format!("%{b:02X}"));
            }
        }
    }
    let all = ctx.api("GET", &path, None)?;
    let author = a.get(&["-A", "--author"]);
    let rows: Vec<Map<String, Value>> = arr(&all)
        .iter()
        .filter(|d| {
            author.is_none_or(|w| d.pointer("/user/login").and_then(Value::as_str) == Some(w))
        })
        .take(limit)
        .map(|d| gh_row(ctx, &o, &r, d))
        .collect();
    if let Some(fields) = a.get(&["--json"]) {
        return print_json_fields(out, &rows, fields, JSON_FIELDS, false);
    }
    for m in &rows {
        row(
            out,
            &[
                &m["number"].to_string(),
                m["title"].as_str().unwrap_or(""),
                m["category"]["name"].as_str().unwrap_or(""),
                m["state"].as_str().unwrap_or(""),
                m["updatedAt"].as_str().unwrap_or(""),
            ],
        )?;
    }
    Ok(())
}

fn view(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &["--json", "-R", "--repo"],
        &["--comments", "-c", "-w", "--web"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let num = number_arg(&a)?;
    let d = ctx.api("GET", &format!("/repos/{o}/{r}/discussions/{num}"), None)?;
    let m = gh_row(ctx, &o, &r, &d);
    if let Some(fields) = a.get(&["--json"]) {
        return print_json_fields(out, &[m], fields, JSON_FIELDS, true);
    }
    writeln!(out, "title:\t{}", s(&d, "title"))?;
    writeln!(out, "number:\t{num}")?;
    writeln!(out, "state:\t{}", m["state"].as_str().unwrap_or(""))?;
    writeln!(
        out,
        "category:\t{}",
        m["category"]["name"].as_str().unwrap_or("")
    )?;
    writeln!(
        out,
        "author:\t{}",
        m["author"]["login"].as_str().unwrap_or("")
    )?;
    writeln!(out, "comments:\t{}", n(&d, "comments"))?;
    writeln!(out, "url:\t{}", m["url"].as_str().unwrap_or(""))?;
    writeln!(out, "--")?;
    writeln!(out, "{}", s(&d, "body"))?;
    if a.has(&["--comments", "-c"]) {
        let cs = ctx.api(
            "GET",
            &format!("/repos/{o}/{r}/discussions/{num}/comments"),
            None,
        )?;
        for c in arr(&cs) {
            writeln!(out, "--")?;
            writeln!(
                out,
                "author:\t{}",
                c.pointer("/user/login")
                    .and_then(Value::as_str)
                    .unwrap_or("")
            )?;
            writeln!(out, "{}", s(c, "body"))?;
        }
    }
    Ok(())
}

fn edit(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[
            "-t",
            "--title",
            "-b",
            "--body",
            "-F",
            "--body-file",
            "-c",
            "--category",
            "-R",
            "--repo",
        ],
        &[],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let num = number_arg(&a)?;
    let mut patch = Map::new();
    if let Some(t) = a.get(&["-t", "--title"]) {
        patch.insert("title".into(), json!(t));
    }
    if let Some(b) = body_arg(&a)? {
        patch.insert("body".into(), json!(b));
    }
    if let Some(c) = a.get(&["-c", "--category"]) {
        patch.insert("category".into(), json!(c));
    }
    if patch.is_empty() {
        return usage("nothing to edit: pass --title, --body/--body-file or --category");
    }
    ctx.api(
        "PATCH",
        &format!("/repos/{o}/{r}/discussions/{num}"),
        Some(&Value::Object(patch)),
    )?;
    writeln!(
        out,
        "{}",
        ctx.url(&format!("/repos/{o}/{r}/discussions/{num}"))
    )?;
    Ok(())
}

fn comment(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &["-b", "--body", "-F", "--body-file", "-R", "--repo"],
        &[],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let num = number_arg(&a)?;
    let Some(body) = body_arg(&a)?.filter(|b| !b.trim().is_empty()) else {
        return usage("--body or --body-file required (no editor is spawned)");
    };
    let c = ctx.api(
        "POST",
        &format!("/repos/{o}/{r}/discussions/{num}/comments"),
        Some(&json!({"body": body})),
    )?;
    writeln!(
        out,
        "{}#discussioncomment-{}",
        ctx.url(&format!("/repos/{o}/{r}/discussions/{num}")),
        n(&c, "id")
    )?;
    Ok(())
}
