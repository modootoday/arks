//! `arks discussion …` — gh-symmetric discussion client
//! (`create|list|view|edit|comment`).

use super::common::{body_of, edit_text, parse_number, url_repo};
use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, n, row, s, usage, web};
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const HELP: &str = "\
arks discussion — repository discussions on the arks forge (gh-symmetric argv; arks-owned; host-tested Partial)

USAGE:
  arks discussion create -t <title> [-b <body> | -F <file>] [-c <category>] [-l <label>] [-R owner/repo]
  arks discussion list [-c <category>] [-s open|closed|all] [-A <author>] [-l <label>] [-S <query>]
                       [--answered] [--sort created|updated] [--order asc|desc] [-L <n>] [--after <cursor>]
                       [--json <fields>] [-q <jq>] [-t <template>] [-w]
  arks discussion view {<number> | <url> | <comment-url>} [-c] [-L <n>] [--order oldest|newest] [--json …] [-w]
  arks discussion edit <number> [-t <title>] [-b <body> | -F <file>] [-c <category>] [--add-label …] [--remove-label …]
  arks discussion comment {<number> | <comment-url>} (-b <body> | -F <file>) [--edit | --delete [--yes]]

A `…#discussioncomment-<id>` URL (or `DC_<id>`) addresses a comment.";

const LIST_FIELDS: &[&str] = &[
    "answerChosenAt",
    "answerChosenBy",
    "answered",
    "author",
    "body",
    "category",
    "closed",
    "closedAt",
    "createdAt",
    "id",
    "labels",
    "locked",
    "number",
    "stateReason",
    "title",
    "updatedAt",
    "url",
];
const VIEW_FIELDS: &[&str] = &[
    "answerChosenAt",
    "answerChosenBy",
    "answered",
    "author",
    "body",
    "category",
    "closed",
    "closedAt",
    "comments",
    "createdAt",
    "id",
    "labels",
    "locked",
    "number",
    "reactionGroups",
    "state",
    "stateReason",
    "title",
    "updatedAt",
    "url",
];

const R: &[&str] = &["-R", "--repo"];

/// `((owner, name), discussion, comment)` named by a discussion argument.
type Target = ((String, String), u64, Option<u64>);

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

fn url_of(ctx: &Ctx, o: &str, r: &str, d: &Value) -> String {
    ctx.url(&format!("/repos/{o}/{r}/discussions/{}", n(d, "number")))
}

fn gh_row(ctx: &Ctx, owner: &str, repo: &str, d: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    let closed = s(d, "state") == "closed";
    for (k, v) in [
        ("answerChosenAt", Value::Null),
        ("answerChosenBy", Value::Null),
        ("answered", json!(d["answered"].as_bool().unwrap_or(false))),
        (
            "author",
            json!({"login": d.pointer("/user/login").and_then(Value::as_str).unwrap_or("")}),
        ),
        ("body", json!(s(d, "body"))),
        (
            "category",
            json!({"name": d.pointer("/category/name").and_then(Value::as_str).unwrap_or("")}),
        ),
        ("closed", json!(closed)),
        ("closedAt", d["closed_at"].clone()),
        ("comments", json!(n(d, "comments"))),
        ("createdAt", json!(s(d, "created_at"))),
        ("id", json!(format!("D_{}", n(d, "number")))),
        (
            "labels",
            Value::Array(
                arr(&d["labels"])
                    .iter()
                    .map(|l| json!({"name": l}))
                    .collect(),
            ),
        ),
        ("locked", json!(false)),
        ("number", json!(n(d, "number"))),
        ("reactionGroups", json!([])),
        ("state", json!(s(d, "state").to_ascii_uppercase())),
        ("stateReason", json!(if closed { "RESOLVED" } else { "" })),
        ("title", json!(s(d, "title"))),
        ("updatedAt", json!(s(d, "updated_at"))),
        ("url", json!(url_of(ctx, owner, repo, d))),
    ] {
        m.insert(k.into(), v);
    }
    m
}

fn body_arg(a: &Args<'_>) -> Result<Option<String>, Fail> {
    if a.get(&["-b", "--body"]).is_some() && a.get(&["-F", "--body-file"]).is_some() {
        return usage("specify only one of --body or --body-file");
    }
    body_of(a, &["-b", "--body"], &["-F", "--body-file"])
}

fn target(ctx: &Ctx, a: &Args<'_>) -> Result<Target, Fail> {
    let p = a
        .positionals()
        .first()
        .copied()
        .ok_or_else(|| Fail::Usage("discussion number required".into()))?;
    let (o, r) = match url_repo(p) {
        Some(x) => x,
        None => ctx.repo(a)?,
    };
    if let Some((_, c)) = p.split_once("#discussioncomment-") {
        let id = c
            .parse()
            .map_err(|_| Fail::Usage(format!("invalid comment URL: {p}")))?;
        return Ok(((o, r), parse_number(p).unwrap_or(0), Some(id)));
    }
    if let Some(id) = p.strip_prefix("DC_").and_then(|x| x.parse().ok()) {
        return Ok(((o, r), 0, Some(id)));
    }
    let num =
        parse_number(p).ok_or_else(|| Fail::Usage(format!("invalid discussion number: {p}")))?;
    Ok(((o, r), num, None))
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
            "-l",
            "--label",
            "-R",
            "--repo",
        ],
        &[],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let Some(title) = a.get(&["-t", "--title"]).filter(|t| !t.trim().is_empty()) else {
        return usage("--title required");
    };
    let mut payload = json!({"title": title, "body": body_arg(&a)?.unwrap_or_default(), "labels": a.list(&["-l", "--label"])});
    if let Some(c) = a.get(&["-c", "--category"]) {
        payload["category"] = json!(c);
    }
    let d = ctx.api(
        "POST",
        &format!("/repos/{o}/{r}/discussions"),
        Some(&payload),
    )?;
    writeln!(out, "{}", url_of(ctx, &o, &r, &d))?;
    Ok(())
}

fn list(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[
            &[
                "-c",
                "--category",
                "-s",
                "--state",
                "-A",
                "--author",
                "-L",
                "--limit",
                "-l",
                "--label",
                "-S",
                "--search",
                "--sort",
                "--order",
                "--after",
            ][..],
            R,
            EXPORT,
        ]
        .concat(),
        &["-w", "--web", "--answered"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    if a.has(&["-w", "--web"]) {
        return web(out, &ctx.url(&format!("/{o}/{r}/discussions")));
    }
    let state = a.get(&["-s", "--state"]).unwrap_or("open");
    if !matches!(state, "open" | "closed" | "all") {
        return usage(format!("invalid state {state:?} (open|closed|all)"));
    }
    let sort = a.get(&["--sort"]).unwrap_or("updated");
    if !matches!(sort, "created" | "updated") {
        return usage(format!(
            "invalid argument {sort:?} for \"--sort\" flag: valid values are {{created|updated}}"
        ));
    }
    let order = a.get(&["--order"]).unwrap_or("desc");
    if !matches!(order, "asc" | "desc") {
        return usage(format!(
            "invalid argument {order:?} for \"--order\" flag: valid values are {{asc|desc}}"
        ));
    }
    let limit = a.limit(30)?;
    let mut path = format!("/repos/{o}/{r}/discussions?state={state}");
    if let Some(c) = a.get(&["-c", "--category"]) {
        path.push_str(&format!("&category={}", super::common::enc(c)));
    }
    let all = ctx.api("GET", &path, None)?;
    let author = a.get(&["-A", "--author"]).map(|x| ctx.who(x)).transpose()?;
    let labels = a.list(&["-l", "--label"]);
    let search = a.get(&["-S", "--search"]).map(str::to_ascii_lowercase);
    let answered = a.bool_opt(&["--answered"]);
    let mut items: Vec<&Value> = arr(&all)
        .iter()
        .filter(|d| {
            author
                .as_deref()
                .is_none_or(|w| d.pointer("/user/login").and_then(Value::as_str) == Some(w))
        })
        .filter(|d| {
            labels.iter().all(|l| {
                arr(&d["labels"])
                    .iter()
                    .any(|x| x.as_str().is_some_and(|y| y.eq_ignore_ascii_case(l)))
            })
        })
        .filter(|d| {
            search.as_deref().is_none_or(|q| {
                s(d, "title").to_ascii_lowercase().contains(q)
                    || s(d, "body").to_ascii_lowercase().contains(q)
            })
        })
        .filter(|d| answered.is_none_or(|w| d["answered"].as_bool().unwrap_or(false) == w))
        .collect();
    let key = if sort == "created" {
        "created_at"
    } else {
        "updated_at"
    };
    items.sort_by(|x, y| {
        s(x, key)
            .cmp(s(y, key))
            .then(n(x, "number").cmp(&n(y, "number")))
    });
    if order == "desc" {
        items.reverse();
    }
    // `--after` is a cursor; arks cursors are discussion numbers.
    if let Some(after) = a.get(&["--after"]) {
        if let Some(pos) = items
            .iter()
            .position(|d| n(d, "number").to_string() == after)
        {
            items.drain(..=pos);
        }
    }
    items.truncate(limit);
    let rows: Vec<Map<String, Value>> = items.iter().map(|d| gh_row(ctx, &o, &r, d)).collect();
    if json_out(out, &a, &rows, LIST_FIELDS, false)? {
        return Ok(());
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
        &[&["-L", "--limit", "--order", "--after"][..], R, EXPORT].concat(),
        &["--comments", "-c", "-w", "--web"],
    )?;
    let ((o, r), num, comment) = target(ctx, &a)?;
    let order = a.get(&["--order"]).unwrap_or("newest");
    if !matches!(order, "oldest" | "newest") {
        return usage(format!(
            "invalid argument {order:?} for \"--order\" flag: valid values are {{oldest|newest}}"
        ));
    }
    let limit = a.limit(30)?;
    if let Some(cid) = comment {
        let list = ctx.api(
            "GET",
            &format!("/repos/{o}/{r}/discussions/{num}/comments"),
            None,
        )?;
        let c = arr(&list)
            .iter()
            .find(|c| n(c, "id") == cid)
            .cloned()
            .ok_or_else(|| Fail::Api(format!("comment {cid} not found")))?;
        writeln!(
            out,
            "author:\t{}\n--\n{}",
            s(&c["user"], "login"),
            s(&c, "body")
        )?;
        return Ok(());
    }
    let d = ctx.api("GET", &format!("/repos/{o}/{r}/discussions/{num}"), None)?;
    if a.has(&["-w", "--web"]) {
        return web(out, &url_of(ctx, &o, &r, &d));
    }
    let mut comments = ctx
        .api(
            "GET",
            &format!("/repos/{o}/{r}/discussions/{num}/comments"),
            None,
        )?
        .as_array()
        .cloned()
        .unwrap_or_default();
    if order == "newest" {
        comments.reverse();
    }
    if let Some(after) = a.get(&["--after"]) {
        if let Some(pos) = comments
            .iter()
            .position(|c| n(c, "id").to_string() == after)
        {
            comments.drain(..=pos);
        }
    }
    comments.truncate(limit);
    let mut m = gh_row(ctx, &o, &r, &d);
    m.insert("comments".into(), Value::Array(comments.iter().map(|c| json!({"id": n(c, "id"), "author": {"login": s(&c["user"], "login")}, "body": s(c, "body"), "createdAt": s(c, "created_at")})).collect()));
    if json_out(out, &a, std::slice::from_ref(&m), VIEW_FIELDS, true)? {
        return Ok(());
    }
    if a.has(&["-c", "--comments"]) {
        writeln!(out, "title:\t{}\n--", s(&d, "title"))?;
        for c in &comments {
            writeln!(
                out,
                "author:\t{}\nid:\t{}\n{}\n--",
                s(&c["user"], "login"),
                n(c, "id"),
                s(c, "body")
            )?;
        }
        return Ok(());
    }
    writeln!(out, "title:\t{}", s(&d, "title"))?;
    writeln!(out, "number:\t{num}")?;
    writeln!(out, "state:\t{}", m["state"].as_str().unwrap_or(""))?;
    writeln!(
        out,
        "category:\t{}",
        d.pointer("/category/name")
            .and_then(Value::as_str)
            .unwrap_or("")
    )?;
    writeln!(
        out,
        "author:\t{}",
        d.pointer("/user/login")
            .and_then(Value::as_str)
            .unwrap_or("")
    )?;
    writeln!(
        out,
        "labels:\t{}",
        arr(&d["labels"])
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    )?;
    writeln!(out, "comments:\t{}", n(&d, "comments"))?;
    writeln!(out, "url:\t{}", url_of(ctx, &o, &r, &d))?;
    writeln!(out, "--")?;
    writeln!(out, "{}", s(&d, "body"))?;
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
            "--add-label",
            "--remove-label",
            "-R",
            "--repo",
        ],
        &[],
    )?;
    let ((o, r), num, _) = target(ctx, &a)?;
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
    let (add, rm) = (a.list(&["--add-label"]), a.list(&["--remove-label"]));
    if !add.is_empty() || !rm.is_empty() {
        let d = ctx.api("GET", &format!("/repos/{o}/{r}/discussions/{num}"), None)?;
        let mut labels: Vec<String> = arr(&d["labels"])
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .filter(|l| !rm.contains(l))
            .collect();
        for l in add {
            if !labels.contains(&l) {
                labels.push(l);
            }
        }
        patch.insert("labels".into(), json!(labels));
    }
    if patch.is_empty() {
        return usage(
            "nothing to edit: pass --title, --body/--body-file, --category or label flags",
        );
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
        &["--delete", "--edit", "--yes"],
    )?;
    let ((o, r), num, cid) = target(ctx, &a)?;
    if a.has(&["--edit"]) && a.has(&["--delete"]) {
        return usage("specify only one of --edit or --delete");
    }
    if a.has(&["--edit"]) || a.has(&["--delete"]) {
        let id = cid.ok_or_else(|| {
            Fail::Usage("--edit and --delete need a comment URL or DC_<id>".into())
        })?;
        let path = format!("/repos/{o}/{r}/discussions/comments/{id}");
        if a.has(&["--delete"]) {
            if !a.has(&["--yes"]) {
                return usage("--yes required to delete a comment when not running interactively");
            }
            ctx.api("DELETE", &path, None)?;
            eprintln!("✓ Deleted comment {id}");
            return Ok(());
        }
        let body = body_arg(&a)?
            .filter(|b| !b.trim().is_empty())
            .ok_or_else(|| Fail::Usage("--body or --body-file required".into()))?;
        ctx.api("PATCH", &path, Some(&json!({"body": body})))?;
        writeln!(
            out,
            "{}#discussioncomment-{id}",
            ctx.url(&format!("/repos/{o}/{r}/discussions/{num}"))
        )?;
        return Ok(());
    }
    let body = match body_arg(&a)? {
        Some(b) => b,
        None if std::env::var_os("GH_EDITOR").is_some()
            || std::env::var_os("VISUAL").is_some()
            || std::env::var_os("EDITOR").is_some() =>
        {
            edit_text("")?
        }
        None => String::new(),
    };
    if body.trim().is_empty() {
        return usage("--body or --body-file required (or set an editor)");
    }
    // A comment URL replies in the same discussion (arks keeps threads flat).
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
