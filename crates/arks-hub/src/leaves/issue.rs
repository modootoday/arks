//! `arks issue …` — gh-symmetric issue client against the arks forge, including
//! `issue develop` (linked branches) and `issue artifact *` (issue artifacts).

use super::common::{
    ListEdits, body_of, edit_text, enc, load_recover, names, parse_number, save_recover,
    title_body, url_repo,
};
use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, n, read_input, row, s, usage, web};
use base64::Engine as _;
use serde_json::{Map, Value, json};
use std::io::Write;
use std::path::Path;

pub(crate) const HELP: &str = "\
arks issue — issues on the arks forge (gh-symmetric argv; host-tested Partial)

USAGE:
  arks issue <command> [flags]

COMMANDS:
  create, list, status, view, close, reopen, comment, edit, delete, lock, unlock,
  pin, unpin, transfer, develop, artifact {create|list|view|edit|delete|download}

Flags are gh's; `-R/--repo [HOST/]OWNER/REPO` selects the repository. `develop`
creates and links a branch on the forge and prints the `arks` commands to check it
out (never spawns git). `--web` prints the URL instead of opening a browser.";

pub(crate) const JSON_FIELDS: &[&str] = &[
    "assignees",
    "author",
    "body",
    "closed",
    "closedAt",
    "closedByPullRequestsReferences",
    "comments",
    "createdAt",
    "id",
    "isPinned",
    "labels",
    "milestone",
    "number",
    "projectCards",
    "projectItems",
    "reactionGroups",
    "state",
    "stateReason",
    "title",
    "updatedAt",
    "url",
];

const ART_FIELDS: &[&str] = &[
    "body",
    "bodyHtml",
    "createdAt",
    "creator",
    "description",
    "id",
    "name",
    "number",
    "type",
    "updatedAt",
    "updatedByActor",
    "url",
];

const R: &[&str] = &["-R", "--repo"];

fn vals(extra: &[&'static str]) -> Vec<&'static str> {
    let mut v: Vec<&str> = R.to_vec();
    v.extend_from_slice(extra);
    v
}

pub(crate) fn gh_row(ctx: &Ctx, i: &Value) -> Map<String, Value> {
    let (o, r) = (
        s(&i["repository"]["owner"], "login"),
        s(&i["repository"], "name"),
    );
    let mut m = Map::new();
    let mut put = |k: &str, v: Value| {
        m.insert(k.to_string(), v);
    };
    put(
        "assignees",
        Value::Array(
            arr(&i["assignees"])
                .iter()
                .map(|a| json!({"login": a["login"], "name": ""}))
                .collect(),
        ),
    );
    put(
        "author",
        json!({"login": i["user"]["login"], "is_bot": false}),
    );
    put("body", i["body"].clone());
    put("closed", json!(s(i, "state") == "closed"));
    put("closedAt", i["closed_at"].clone());
    put("closedByPullRequestsReferences", json!([]));
    put("comments", json!([]));
    put("createdAt", i["created_at"].clone());
    put("id", i["node_id"].clone());
    put("isPinned", i["pinned"].clone());
    put("labels", Value::Array(arr(&i["labels"]).iter().map(|l| json!({"name": l["name"], "color": l["color"], "description": l["description"], "id": l["node_id"]})).collect()));
    put(
        "milestone",
        match &i["milestone"] {
            Value::Null => Value::Null,
            m => {
                json!({"number": m["number"], "title": m["title"], "description": m["description"]})
            }
        },
    );
    put("number", i["number"].clone());
    put("projectCards", json!([]));
    put(
        "projectItems",
        Value::Array(
            arr(&i["projects"])
                .iter()
                .map(|t| json!({"title": t}))
                .collect(),
        ),
    );
    put("reactionGroups", json!([]));
    put("state", json!(s(i, "state").to_ascii_uppercase()));
    put(
        "stateReason",
        json!(
            i["state_reason"]
                .as_str()
                .map(|x| x.to_ascii_uppercase().replace(' ', "_"))
                .unwrap_or_default()
        ),
    );
    put("title", i["title"].clone());
    put("updatedAt", i["updated_at"].clone());
    put(
        "url",
        json!(ctx.web_url(&format!("/{o}/{r}/issues/{}", n(i, "number")))),
    );
    m
}

/// Resolve an issue argument (number, `#n` or URL).
fn resolve(ctx: &Ctx, a: &Args<'_>, arg: Option<&str>) -> Result<((String, String), Value), Fail> {
    let arg = arg.ok_or_else(|| Fail::Usage("issue number or URL required".into()))?;
    let (o, r) = match url_repo(arg) {
        Some(x) => x,
        None => ctx.repo(a)?,
    };
    let num =
        parse_number(arg).ok_or_else(|| Fail::Usage(format!("invalid issue format: {arg:?}")))?;
    let i = ctx.get(&format!("/repos/{o}/{r}/issues/{num}"))?;
    Ok(((o, r), i))
}

fn ipath(o: &str, r: &str, i: &Value) -> String {
    format!("/repos/{o}/{r}/issues/{}", n(i, "number"))
}

fn iurl(ctx: &Ctx, o: &str, r: &str, i: &Value) -> String {
    ctx.web_url(&format!("/{o}/{r}/issues/{}", n(i, "number")))
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "create" | "new" => create(ctx, rest, out),
        "list" | "ls" => list(ctx, rest, out),
        "status" => status(ctx, rest, out),
        "view" => view(ctx, rest, out),
        "close" => close(ctx, rest, out),
        "reopen" => reopen(ctx, rest, out),
        "comment" => {
            let a = Args::parse(rest, super::pr::COMMENT_VALUES, super::pr::COMMENT_BOOLS)?;
            let ((o, r), i) = resolve(ctx, &a, a.positionals().first().copied())?;
            super::pr::comment_flow(
                ctx,
                &a,
                &format!("{}/comments", ipath(&o, &r, &i)),
                &iurl(ctx, &o, &r, &i),
                out,
            )
        }
        "edit" => edit(ctx, rest, out),
        "delete" => delete(ctx, rest, out),
        "lock" | "unlock" => lock(ctx, rest, out, sub == "lock"),
        "pin" | "unpin" => pin(ctx, rest, out, sub == "pin"),
        "transfer" => transfer(ctx, rest, out),
        "develop" => develop(ctx, rest, out),
        "artifact" => artifact(ctx, rest, out),
        "-R" | "--repo" => Err(Fail::Help),
        other => usage(format!(
            "unknown command {other:?} for \"arks issue\"\n{HELP}"
        )),
    }
}

/// Body of a named issue template from the repository's `.github/ISSUE_TEMPLATE/`.
fn template_body(ctx: &Ctx, o: &str, r: &str, name: &str) -> Result<String, Fail> {
    let dir = ctx.get(&format!("/repos/{o}/{r}/contents/.github/ISSUE_TEMPLATE"))?;
    for e in arr(&dir) {
        let f = ctx.get(&format!("/repos/{o}/{r}/contents/{}", s(e, "path")))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(s(&f, "content"))
            .unwrap_or_default();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let tname = text
            .lines()
            .find_map(|l| l.strip_prefix("name:"))
            .map(|x| x.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| s(e, "name").to_string());
        if tname.eq_ignore_ascii_case(name) || s(e, "name").eq_ignore_ascii_case(name) {
            let body = match text.strip_prefix("---") {
                Some(rest) => rest
                    .split_once("\n---")
                    .map(|(_, b)| b.trim_start().to_string())
                    .unwrap_or(text.clone()),
                None => text,
            };
            return Ok(body);
        }
    }
    usage(format!("no template named {name:?} in {o}/{r}"))
}

fn create(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&[
            "-a",
            "--assignee",
            "-b",
            "--body",
            "-F",
            "--body-file",
            "-l",
            "--label",
            "-m",
            "--milestone",
            "-p",
            "--project",
            "--recover",
            "-T",
            "--template",
            "-t",
            "--title",
        ]),
        &["-e", "--editor", "-w", "--web"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let rec = match a.get(&["--recover"]) {
        Some(p) => load_recover(p)?,
        None => json!({}),
    };
    let mut title = a
        .get(&["-t", "--title"])
        .map(str::to_string)
        .or(rec["title"].as_str().map(str::to_string));
    let mut body = body_of(&a, &["-b", "--body"], &["-F", "--body-file"])?
        .or(rec["body"].as_str().map(str::to_string));
    if body.is_none() {
        if let Some(t) = a.get(&["-T", "--template"]) {
            body = Some(template_body(ctx, &o, &r, t)?);
        }
    }
    if a.has(&["-e", "--editor"]) {
        let (t, b) = title_body(&edit_text(&format!(
            "{}\n\n{}",
            title.clone().unwrap_or_default(),
            body.clone().unwrap_or_default()
        ))?);
        title = Some(t);
        body = Some(b);
    }
    if a.has(&["-w", "--web"]) {
        let mut url = ctx.web_url(&format!("/{o}/{r}/issues/new"));
        if let Some(t) = &title {
            url.push_str(&format!("?title={}", enc(t)));
        }
        return web(out, &url);
    }
    let Some(title) = title.filter(|t| !t.trim().is_empty()) else {
        return usage("must provide `--title` and `--body` when not running interactively");
    };
    let mut payload = json!({
        "title": title,
        "body": body.unwrap_or_default(),
        "labels": a.list(&["-l", "--label"]),
        "assignees": ctx.whos(&a.list(&["-a", "--assignee"]))?,
        "projects": a.list(&["-p", "--project"]),
    });
    if let Some(m) = a.get(&["-m", "--milestone"]) {
        payload["milestone"] = json!(m);
    }
    match ctx.api("POST", &format!("/repos/{o}/{r}/issues"), Some(&payload)) {
        Ok(i) => {
            writeln!(out, "{}", iurl(ctx, &o, &r, &i))?;
            Ok(())
        }
        Err(e) => {
            if let Some(p) = save_recover("issue", &payload) {
                eprintln!("Failed to create issue. Input saved to: {}", p.display());
                eprintln!(
                    "To recover, run: arks issue create --recover {}",
                    p.display()
                );
            }
            Err(e)
        }
    }
}

fn list(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[
            vals(&[
                "--app",
                "-a",
                "--assignee",
                "-A",
                "--author",
                "-l",
                "--label",
                "-L",
                "--limit",
                "--mention",
                "-m",
                "--milestone",
                "-S",
                "--search",
                "-s",
                "--state",
            ]),
            EXPORT.to_vec(),
        ]
        .concat(),
        &["-w", "--web"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let state = a.get(&["-s", "--state"]).unwrap_or("open");
    if !matches!(state, "open" | "closed" | "all") {
        return usage(format!(
            "invalid argument {state:?} for \"--state\" flag: valid values are {{open|closed|all}}"
        ));
    }
    let limit = a.limit(30)?;
    if a.has(&["-w", "--web"]) {
        return web(out, &ctx.web_url(&format!("/{o}/{r}/issues")));
    }
    let mut q = format!("/repos/{o}/{r}/issues?state={state}");
    let labels = a.list(&["-l", "--label"]);
    if !labels.is_empty() {
        q.push_str(&format!("&labels={}", enc(&labels.join(","))));
    }
    for (flag, key) in [
        (&["-a", "--assignee"][..], "assignee"),
        (&["-A", "--author"], "creator"),
        (&["--mention"], "mentioned"),
        (&["-m", "--milestone"], "milestone"),
    ] {
        if let Some(v) = a.get(flag) {
            q.push_str(&format!("&{key}={}", enc(&ctx.who(v)?)));
        }
    }
    let mut items: Vec<Value> = if let Some(sq) = a.get(&["-S", "--search"]) {
        let query = format!("{sq} repo:{o}/{r} is:issue");
        ctx.get(&format!("/search/issues?q={}", enc(&query)))?["items"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else {
        ctx.get(&q)?.as_array().cloned().unwrap_or_default()
    };
    if let Some(app) = a.get(&["--app"]) {
        items.retain(|i| i["user"]["login"].as_str() == Some(&format!("{app}[bot]")));
    }
    items.truncate(limit);
    let rows: Vec<Map<String, Value>> = items.iter().map(|i| gh_row(ctx, i)).collect();
    if json_out(out, &a, &rows, JSON_FIELDS, false)? {
        return Ok(());
    }
    if rows.is_empty() {
        eprintln!("no issues match your search in {o}/{r}");
        return Ok(());
    }
    for i in &items {
        row(
            out,
            &[
                &n(i, "number").to_string(),
                &s(i, "state").to_ascii_uppercase(),
                s(i, "title"),
                &names(i, "labels"),
                s(i, "updated_at"),
            ],
        )?;
    }
    Ok(())
}

fn status(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &[R.to_vec(), EXPORT.to_vec()].concat(), &[])?;
    let (o, r) = ctx.repo(&a)?;
    let me = ctx.me()?;
    let get = |q: &str| -> Result<Vec<Value>, Fail> {
        Ok(ctx
            .get(&format!(
                "/repos/{o}/{r}/issues?state=open&{q}={}",
                enc(&me)
            ))?
            .as_array()
            .cloned()
            .unwrap_or_default())
    };
    let (assigned, mentioned, opened) = (get("assignee")?, get("mentioned")?, get("creator")?);
    if a.get(&["--json"]).is_some() {
        let sec =
            |v: &[Value]| Value::Array(v.iter().map(|i| Value::Object(gh_row(ctx, i))).collect());
        let v = json!({"assigned": sec(&assigned), "mentioned": sec(&mentioned), "authored": sec(&opened)});
        return super::emit(
            out,
            &v,
            a.get(&["-q", "--jq"]),
            a.get(&["-t", "--template"]),
        );
    }
    writeln!(out, "\nRelevant issues in {o}/{r}\n")?;
    for (title, list, none) in [
        (
            "Issues assigned to you",
            &assigned,
            "There are no issues assigned to you",
        ),
        (
            "Issues mentioning you",
            &mentioned,
            "There are no issues mentioning you",
        ),
        (
            "Issues opened by you",
            &opened,
            "There are no issues opened by you",
        ),
    ] {
        writeln!(out, "{title}")?;
        if list.is_empty() {
            writeln!(out, "  {none}")?;
        }
        for i in list {
            let labels = names(i, "labels");
            writeln!(
                out,
                "  #{}  {}{}",
                n(i, "number"),
                s(i, "title"),
                if labels.is_empty() {
                    String::new()
                } else {
                    format!(" ({labels})")
                }
            )?;
        }
        writeln!(out)?;
    }
    Ok(())
}

fn view(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[R.to_vec(), EXPORT.to_vec()].concat(),
        &["-c", "--comments", "-w", "--web"],
    )?;
    let ((o, r), i) = resolve(ctx, &a, a.positionals().first().copied())?;
    if a.has(&["-w", "--web"]) {
        return web(out, &iurl(ctx, &o, &r, &i));
    }
    let comments = || ctx.get(&format!("{}/comments", ipath(&o, &r, &i)));
    if a.get(&["--json"]).is_some() {
        let mut m = gh_row(ctx, &i);
        if a.get(&["--json"]).is_some_and(|j| j.contains("comments")) {
            let c = comments()?;
            m.insert("comments".into(), Value::Array(arr(&c).iter().map(|c| json!({"author": {"login": c["user"]["login"]}, "body": c["body"], "createdAt": c["created_at"]})).collect()));
        }
        json_out(out, &a, std::slice::from_ref(&m), JSON_FIELDS, true)?;
        return Ok(());
    }
    if a.has(&["-c", "--comments"]) {
        for c in arr(&comments()?) {
            writeln!(
                out,
                "author:\t{}\nassociation:\towner\nedited:\tfalse\nstatus:\tnone\n--\n{}\n--",
                s(&c["user"], "login"),
                s(c, "body")
            )?;
        }
        return Ok(());
    }
    writeln!(out, "title:\t{}", s(&i, "title"))?;
    writeln!(out, "state:\t{}", s(&i, "state").to_ascii_uppercase())?;
    writeln!(out, "author:\t{}", s(&i["user"], "login"))?;
    writeln!(out, "labels:\t{}", names(&i, "labels"))?;
    writeln!(out, "comments:\t{}", n(&i, "comments"))?;
    writeln!(out, "assignees:\t{}", names(&i, "assignees"))?;
    writeln!(
        out,
        "projects:\t{}",
        arr(&i["projects"])
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    )?;
    writeln!(
        out,
        "milestone:\t{}",
        i["milestone"]["title"].as_str().unwrap_or("")
    )?;
    writeln!(out, "number:\t{}", n(&i, "number"))?;
    writeln!(out, "--")?;
    writeln!(out, "{}", s(&i, "body"))?;
    Ok(())
}

fn close(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&["-c", "--comment", "--duplicate-of", "-r", "--reason"]),
        &[],
    )?;
    let reason = a.get(&["-r", "--reason"]).map(|r| r.to_ascii_lowercase());
    if let Some(rs) = &reason {
        if !matches!(rs.as_str(), "completed" | "not planned" | "duplicate") {
            return usage(format!(
                "invalid argument {rs:?} for \"-r, --reason\" flag: valid values are {{completed|not planned|duplicate}}"
            ));
        }
    }
    let ((o, r), i) = resolve(ctx, &a, a.positionals().first().copied())?;
    if s(&i, "state") == "closed" {
        eprintln!(
            "! Issue {o}/{r}#{} ({}) is already closed",
            n(&i, "number"),
            s(&i, "title")
        );
        return Ok(());
    }
    let mut reason = reason.map(|x| x.replace(' ', "_"));
    if let Some(dup) = a.get(&["--duplicate-of"]) {
        let num = parse_number(dup)
            .ok_or_else(|| Fail::Usage(format!("invalid issue format: {dup:?}")))?;
        if num == n(&i, "number") {
            return usage("an issue can't be closed as a duplicate of itself");
        }
        ctx.get(&format!("/repos/{o}/{r}/issues/{num}"))?;
        ctx.api(
            "POST",
            &format!("{}/comments", ipath(&o, &r, &i)),
            Some(&json!({"body": format!("Duplicate of #{num}")})),
        )?;
        reason = Some("duplicate".into());
    }
    if let Some(c) = a.get(&["-c", "--comment"]) {
        ctx.api(
            "POST",
            &format!("{}/comments", ipath(&o, &r, &i)),
            Some(&json!({"body": c})),
        )?;
    }
    let mut body = json!({"state": "closed"});
    if let Some(rs) = reason {
        body["state_reason"] = json!(rs);
    }
    ctx.api("PATCH", &ipath(&o, &r, &i), Some(&body))?;
    eprintln!(
        "✓ Closed issue {o}/{r}#{} ({})",
        n(&i, "number"),
        s(&i, "title")
    );
    let _ = out;
    Ok(())
}

fn reopen(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["-c", "--comment"]), &[])?;
    let ((o, r), i) = resolve(ctx, &a, a.positionals().first().copied())?;
    if s(&i, "state") == "open" {
        eprintln!(
            "! Issue {o}/{r}#{} ({}) is already open",
            n(&i, "number"),
            s(&i, "title")
        );
        return Ok(());
    }
    if let Some(c) = a.get(&["-c", "--comment"]) {
        ctx.api(
            "POST",
            &format!("{}/comments", ipath(&o, &r, &i)),
            Some(&json!({"body": c})),
        )?;
    }
    ctx.api("PATCH", &ipath(&o, &r, &i), Some(&json!({"state": "open"})))?;
    eprintln!(
        "✓ Reopened issue {o}/{r}#{} ({})",
        n(&i, "number"),
        s(&i, "title")
    );
    let _ = out;
    Ok(())
}

fn edit(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&[
            "--add-assignee",
            "--add-label",
            "--add-project",
            "-b",
            "--body",
            "-F",
            "--body-file",
            "-m",
            "--milestone",
            "--remove-assignee",
            "--remove-label",
            "--remove-project",
            "-t",
            "--title",
        ]),
        &["--remove-milestone"],
    )?;
    let targets = a.positionals();
    if targets.is_empty() {
        return usage("issue number or URL required");
    }
    for t in targets {
        let ((o, r), i) = resolve(ctx, &a, Some(t))?;
        let mut patch = Map::new();
        if let Some(t) = a.get(&["-t", "--title"]) {
            patch.insert("title".into(), json!(t));
        }
        if let Some(b) = body_of(&a, &["-b", "--body"], &["-F", "--body-file"])? {
            patch.insert("body".into(), json!(b));
        }
        let edits = ListEdits {
            labels: (a.list(&["--add-label"]), a.list(&["--remove-label"])),
            assignees: (
                ctx.whos(&a.list(&["--add-assignee"]))?,
                ctx.whos(&a.list(&["--remove-assignee"]))?,
            ),
            reviewers: Default::default(),
        };
        let (add, rm) = (a.list(&["--add-project"]), a.list(&["--remove-project"]));
        if !add.is_empty() || !rm.is_empty() {
            let mut v: Vec<String> = arr(&i["projects"])
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .filter(|x| !rm.contains(x))
                .collect();
            for x in add {
                if !v.contains(&x) {
                    v.push(x);
                }
            }
            patch.insert("projects".into(), json!(v));
        }
        if let Some(m) = a.get(&["-m", "--milestone"]) {
            patch.insert("milestone".into(), json!(m));
        }
        if a.has(&["--remove-milestone"]) {
            patch.insert("milestone".into(), Value::Null);
        }
        if patch.is_empty() && edits.is_empty() {
            return usage("specify at least one field to edit (see --help)");
        }
        if !patch.is_empty() {
            ctx.api("PATCH", &ipath(&o, &r, &i), Some(&Value::Object(patch)))?;
        }
        edits.apply(ctx, &ipath(&o, &r, &i))?;
        writeln!(out, "{}", iurl(ctx, &o, &r, &i))?;
    }
    Ok(())
}

fn delete(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, R, &["--yes", "-y"])?;
    let ((o, r), i) = resolve(ctx, &a, a.positionals().first().copied())?;
    if !a.has(&["--yes", "-y"]) {
        eprintln!("note: deleting without a prompt (non-interactive); gh asks for --yes");
    }
    ctx.api("DELETE", &ipath(&o, &r, &i), None)?;
    eprintln!(
        "✓ Deleted issue {o}/{r}#{} ({})",
        n(&i, "number"),
        s(&i, "title")
    );
    writeln!(out, "deleted={}", n(&i, "number"))?;
    Ok(())
}

fn lock(ctx: &Ctx, raw: &[String], out: &mut dyn Write, on: bool) -> Res {
    let a = if on {
        Args::parse(raw, &vals(&["-r", "--reason"]), &[])?
    } else {
        Args::parse(raw, R, &[])?
    };
    let reason = a.get(&["-r", "--reason"]);
    if let Some(rs) = reason {
        if !matches!(rs, "off_topic" | "resolved" | "spam" | "too_heated") {
            return usage(format!(
                "invalid reason {rs:?}: valid reasons are off_topic, resolved, spam, too_heated"
            ));
        }
    }
    let ((o, r), i) = resolve(ctx, &a, a.positionals().first().copied())?;
    let path = format!("{}/lock", ipath(&o, &r, &i));
    if on {
        ctx.api("PUT", &path, Some(&json!({"lock_reason": reason})))?;
        eprintln!(
            "✓ Locked as {} conversation on issue {o}/{r}#{}",
            reason.unwrap_or("").replace('_', " "),
            n(&i, "number")
        );
    } else {
        ctx.api("DELETE", &path, None)?;
        eprintln!(
            "✓ Unlocked conversation on issue {o}/{r}#{}",
            n(&i, "number")
        );
    }
    writeln!(
        out,
        "number={}\nlocked={on}\npinned={}",
        n(&i, "number"),
        i["pinned"].as_bool().unwrap_or(false)
    )?;
    Ok(())
}

fn pin(ctx: &Ctx, raw: &[String], out: &mut dyn Write, on: bool) -> Res {
    let a = Args::parse(raw, R, &[])?;
    let ((o, r), i) = resolve(ctx, &a, a.positionals().first().copied())?;
    ctx.api(
        if on { "PUT" } else { "DELETE" },
        &format!("{}/pin", ipath(&o, &r, &i)),
        None,
    )?;
    eprintln!(
        "✓ {} issue {o}/{r}#{} ({})",
        if on { "Pinned" } else { "Unpinned" },
        n(&i, "number"),
        s(&i, "title")
    );
    writeln!(
        out,
        "number={}\nlocked={}\npinned={on}",
        n(&i, "number"),
        i["locked"].as_bool().unwrap_or(false)
    )?;
    Ok(())
}

fn transfer(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    // `--to` / `-t` is the legacy arks spelling; gh takes the destination positionally.
    let a = Args::parse(raw, &vals(&["--to", "-t"]), &[])?;
    let pos = a.positionals();
    let dest = pos
        .get(1)
        .copied()
        .or(a.get(&["--to", "-t"]))
        .ok_or_else(|| {
            Fail::Usage("usage: arks issue transfer {<number> | <url>} <destination-repo>".into())
        })?;
    let (dop, dn) = super::common::parse_slug(dest)
        .ok_or_else(|| Fail::Usage(format!("invalid destination {dest:?}")))?;
    let ((o, r), i) = resolve(ctx, &a, pos.first().copied())?;
    let v = ctx.api(
        "POST",
        &format!("{}/transfer", ipath(&o, &r, &i)),
        Some(&json!({"new_repository": format!("{dop}/{dn}")})),
    )?;
    writeln!(
        out,
        "{}",
        ctx.web_url(&format!("/{dop}/{dn}/issues/{}", n(&v, "number")))
    )?;
    eprintln!("number={}\nrepository={dop}/{dn}", n(&v, "number"));
    Ok(())
}

fn develop(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&["-b", "--base", "--branch-repo", "-n", "--name"]),
        &["-c", "--checkout", "-l", "--list"],
    )?;
    let ((o, r), i) = resolve(ctx, &a, a.positionals().first().copied())?;
    let path = format!("{}/branches", ipath(&o, &r, &i));
    if a.has(&["-l", "--list"]) {
        let list = ctx.get(&path)?;
        if arr(&list).is_empty() {
            eprintln!("No linked branches found for {o}/{r}#{}", n(&i, "number"));
        }
        for b in arr(&list) {
            row(out, &[s(b, "name"), s(b, "url")])?;
        }
        return Ok(());
    }
    let mut body = json!({});
    if let Some(n) = a.get(&["-n", "--name"]) {
        body["name"] = json!(n);
    }
    if let Some(b) = a.get(&["-b", "--base"]) {
        body["base"] = json!(b);
    }
    if let Some(br) = a.get(&["--branch-repo"]) {
        let (bo, bn) = super::common::parse_slug(br)
            .ok_or_else(|| Fail::Usage(format!("invalid --branch-repo {br:?}")))?;
        body["repo"] = json!(format!("{bo}/{bn}"));
    }
    let v = ctx.api("POST", &path, Some(&body))?;
    let name = s(&v, "name").to_string();
    writeln!(
        out,
        "{}",
        ctx.web_url(&format!(
            "/{}/tree/{name}",
            v["repository"]["full_name"].as_str().unwrap_or("")
        ))
    )?;
    if a.has(&["-c", "--checkout"]) {
        writeln!(
            out,
            "recipe=arks fetch origin {name} && arks checkout -b {name} FETCH_HEAD"
        )?;
        eprintln!("note: arks never spawns git; run the recipe above to check the branch out");
    }
    Ok(())
}

/// `<file>[#<name>]` → (path, optional name).
fn split_name(arg: &str) -> (&str, Option<&str>) {
    match arg.rsplit_once('#') {
        Some((p, n)) if !p.is_empty() && !arg.starts_with("http") => (p, Some(n)),
        _ => (arg, None),
    }
}

fn attachments(a: &Args<'_>) -> Result<Vec<Value>, Fail> {
    let mut out = Vec::new();
    for spec in a.get_all(&["--attach"]) {
        let (path, alt) = split_name(spec);
        let bytes =
            std::fs::read(path).map_err(|e| Fail::Usage(format!("failed to read {path}: {e}")))?;
        let name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment");
        out.push(json!({"name": name, "alt": alt.unwrap_or(""), "content": base64::engine::general_purpose::STANDARD.encode(bytes)}));
    }
    Ok(out)
}

/// `((owner, name), issue, artifact)` named by an artifact argument.
type ArtTarget = ((String, String), u64, Option<u64>);

fn art_issue(ctx: &Ctx, a: &Args<'_>, arg: &str) -> Result<ArtTarget, Fail> {
    // `<issue>` or an artifact URL `…/issues/<n>/artifacts/<m>`.
    let (o, r) = match url_repo(arg) {
        Some(x) => x,
        None => ctx.repo(a)?,
    };
    let num =
        parse_number(arg).ok_or_else(|| Fail::Usage(format!("invalid issue format: {arg:?}")))?;
    let art = arg
        .split_once("/artifacts/")
        .and_then(|(_, m)| m.trim_end_matches('/').parse().ok());
    Ok(((o, r), num, art))
}

fn artifact(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    let art_json = |m: &Value| -> Map<String, Value> {
        let mut x = Map::new();
        for f in ART_FIELDS.iter().chain(["versions"].iter()) {
            x.insert(f.to_string(), m.get(*f).cloned().unwrap_or(Value::Null));
        }
        x
    };
    match sub {
        "create" | "new" => {
            let a = Args::parse(
                raw,
                &vals(&[
                    "--attach",
                    "-b",
                    "--body",
                    "-F",
                    "--body-file",
                    "--name",
                    "--type",
                ]),
                &[],
            )?;
            let pos = a.positionals();
            let (&first, items) = pos.split_first().ok_or_else(|| {
                Fail::Usage(
                    "usage: arks issue artifact create <issue> [<file>[#<name>] | <url>]...".into(),
                )
            })?;
            let ((o, r), num, _) = art_issue(ctx, &a, first)?;
            let path = format!("/repos/{o}/{r}/issues/{num}/artifacts");
            let att = attachments(&a)?;
            let kind = a.get(&["--type"]);
            let mut specs: Vec<Value> = Vec::new();
            if let Some(b) = body_of(&a, &["-b", "--body"], &["-F", "--body-file"])? {
                specs.push(json!({"name": a.get(&["--name"]).unwrap_or("document"), "body": b, "type": kind}));
            }
            for item in items {
                let (p, name) = split_name(item);
                if p.starts_with("http://") || p.starts_with("https://") {
                    specs.push(
                        json!({"name": a.get(&["--name"]).or(name), "body": p, "type": "link"}),
                    );
                    continue;
                }
                let text = read_input(Some(p))?;
                let fname = Path::new(p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(p)
                    .to_string();
                let (body, t) = if fname.ends_with(".url") {
                    let url = text
                        .lines()
                        .find_map(|l| l.strip_prefix("URL="))
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    (url, "link")
                } else if fname.ends_with(".html") || fname.ends_with(".htm") {
                    (text, "embedded_html")
                } else {
                    (text, "generic")
                };
                specs.push(json!({"name": name.map(str::to_string).or(a.get(&["--name"]).map(str::to_string)).unwrap_or(fname), "body": body, "type": kind.unwrap_or(t)}));
            }
            if specs.is_empty() {
                return usage("nothing to create: pass files, URLs, --body or --body-file");
            }
            for (k, mut spec) in specs.into_iter().enumerate() {
                if k == 0 && !att.is_empty() {
                    spec["attachments"] = json!(att);
                }
                if spec["type"].is_null() {
                    spec.as_object_mut().map(|m| m.remove("type"));
                }
                if spec["name"].is_null() {
                    spec.as_object_mut().map(|m| m.remove("name"));
                }
                let v = ctx.api("POST", &path, Some(&spec))?;
                writeln!(out, "{}", s(&v, "url"))?;
            }
            Ok(())
        }
        "list" | "ls" => {
            let a = Args::parse(
                raw,
                &[vals(&["-L", "--limit", "--type"]), EXPORT.to_vec()].concat(),
                &[],
            )?;
            let first = a
                .positionals()
                .first()
                .copied()
                .ok_or_else(|| Fail::Usage("issue number or URL required".into()))?;
            let ((o, r), num, _) = art_issue(ctx, &a, first)?;
            let mut path = format!("/repos/{o}/{r}/issues/{num}/artifacts");
            if let Some(t) = a.get(&["--type"]) {
                if !crate::forge::artifacts::TYPES.contains(&t) {
                    return usage(format!(
                        "invalid argument {t:?} for \"--type\" flag: valid values are {{generic|plan|link|embedded_html}}"
                    ));
                }
                path.push_str(&format!("?type={t}"));
            }
            let mut items = ctx.get(&path)?.as_array().cloned().unwrap_or_default();
            if let Some(l) = a.get(&["-L", "--limit"]) {
                let l: usize = l
                    .parse()
                    .map_err(|_| Fail::Usage(format!("invalid limit: {l}")))?;
                if l > 0 {
                    items.truncate(l);
                }
            }
            let rows: Vec<Map<String, Value>> = items.iter().map(art_json).collect();
            if json_out(out, &a, &rows, ART_FIELDS, false)? {
                return Ok(());
            }
            for m in &items {
                row(
                    out,
                    &[
                        &n(m, "number").to_string(),
                        s(m, "name"),
                        s(m, "type"),
                        s(&m["updatedByActor"], "login"),
                        s(m, "updatedAt"),
                    ],
                )?;
            }
            Ok(())
        }
        "view" => {
            let a = Args::parse(
                raw,
                &[vals(&["--version"]), EXPORT.to_vec()].concat(),
                &["-w", "--web"],
            )?;
            let pos = a.positionals();
            let first = pos.first().copied().ok_or_else(|| {
                Fail::Usage(
                    "usage: arks issue artifact view {<issue> <number> | <artifact-url>}".into(),
                )
            })?;
            let ((o, r), num, art) = art_issue(ctx, &a, first)?;
            let art = art
                .or_else(|| pos.get(1).and_then(|x| x.parse().ok()))
                .ok_or_else(|| Fail::Usage("artifact number required".into()))?;
            let mut path = format!("/repos/{o}/{r}/issues/{num}/artifacts/{art}");
            if let Some(v) = a.get(&["--version"]).filter(|v| *v != "0") {
                path.push_str(&format!("?version={v}"));
            }
            let m = ctx.get(&path)?;
            if a.has(&["-w", "--web"]) {
                let url = if s(&m, "type") == "link" {
                    s(&m, "body").trim().to_string()
                } else {
                    s(&m, "url").to_string()
                };
                return web(out, &url);
            }
            let mut fields: Vec<&str> = ART_FIELDS.to_vec();
            fields.push("versions");
            if json_out(out, &a, std::slice::from_ref(&art_json(&m)), &fields, true)? {
                return Ok(());
            }
            writeln!(
                out,
                "name:\t{}\ntype:\t{}\nnumber:\t{}\nversion:\t{}\ncreator:\t{}\nupdated-by:\t{}\nurl:\t{}\n--",
                s(&m, "name"),
                s(&m, "type"),
                n(&m, "number"),
                n(&m, "version"),
                s(&m["creator"], "login"),
                s(&m["updatedByActor"], "login"),
                s(&m, "url")
            )?;
            writeln!(out, "{}", s(&m, "body"))?;
            let versions = arr(&m["versions"]);
            if versions.len() > 1 {
                writeln!(out, "--\nhistory:")?;
                for v in versions {
                    writeln!(
                        out,
                        "  {}\t{}\t{}\t{}",
                        n(v, "version"),
                        s(v, "name"),
                        s(&v["actor"], "login"),
                        s(v, "createdAt")
                    )?;
                }
            }
            Ok(())
        }
        "edit" => {
            let a = Args::parse(
                raw,
                &vals(&[
                    "--attach",
                    "-b",
                    "--body",
                    "-F",
                    "--body-file",
                    "--name",
                    "--restore-version",
                ]),
                &[],
            )?;
            let pos = a.positionals();
            let first = pos.first().copied().ok_or_else(|| {
                Fail::Usage(
                    "usage: arks issue artifact edit {<issue> <number> | <artifact-url>}".into(),
                )
            })?;
            let ((o, r), num, art) = art_issue(ctx, &a, first)?;
            let art = art
                .or_else(|| pos.get(1).and_then(|x| x.parse().ok()))
                .ok_or_else(|| Fail::Usage("artifact number required".into()))?;
            let mut body = json!({});
            if let Some(nm) = a.get(&["--name"]) {
                body["name"] = json!(nm);
            }
            if let Some(b) = body_of(&a, &["-b", "--body"], &["-F", "--body-file"])? {
                let b = if b.contains("URL=") {
                    b.lines()
                        .find_map(|l| l.strip_prefix("URL="))
                        .unwrap_or(&b)
                        .trim()
                        .to_string()
                } else {
                    b
                };
                body["body"] = json!(b);
            }
            if let Some(v) = a.get(&["--restore-version"]).filter(|v| *v != "0") {
                body["restore_version"] = json!(
                    v.parse::<u64>()
                        .map_err(|_| Fail::Usage(format!("invalid --restore-version {v:?}")))?
                );
            }
            let att = attachments(&a)?;
            if !att.is_empty() {
                body["attachments"] = json!(att);
            }
            if body.as_object().is_some_and(|m| m.is_empty()) {
                return usage("specify --name, --body, --body-file, --attach or --restore-version");
            }
            let v = ctx.api(
                "PATCH",
                &format!("/repos/{o}/{r}/issues/{num}/artifacts/{art}"),
                Some(&body),
            )?;
            writeln!(out, "{}", s(&v, "url"))?;
            Ok(())
        }
        "delete" => {
            let a = Args::parse(raw, R, &["--yes"])?;
            let pos = a.positionals();
            let first = pos.first().copied().ok_or_else(|| {
                Fail::Usage(
                    "usage: arks issue artifact delete {<issue> <number>... | <artifact-url>...}"
                        .into(),
                )
            })?;
            let ((o, r), num, art) = art_issue(ctx, &a, first)?;
            let mut targets: Vec<u64> = art.into_iter().collect();
            for p in &pos[1..] {
                match art_issue(ctx, &a, p)? {
                    (_, _, Some(m)) => targets.push(m),
                    _ => targets.push(
                        p.parse()
                            .map_err(|_| Fail::Usage(format!("invalid artifact number {p:?}")))?,
                    ),
                }
            }
            if targets.is_empty() {
                return usage("artifact number required");
            }
            if !a.has(&["--yes"]) {
                return usage("--yes required when not running interactively");
            }
            for t in targets {
                ctx.api(
                    "DELETE",
                    &format!("/repos/{o}/{r}/issues/{num}/artifacts/{t}"),
                    None,
                )?;
                eprintln!("✓ Deleted artifact {t} from {o}/{r}#{num}");
            }
            let _ = out;
            Ok(())
        }
        "download" => {
            let a = Args::parse(
                raw,
                &vals(&["-D", "--dir", "-O", "--output", "--version"]),
                &["--clobber", "--skip-existing"],
            )?;
            let pos = a.positionals();
            let first = pos.first().copied().ok_or_else(|| Fail::Usage("usage: arks issue artifact download {<issue> [<number>...] | <artifact-url>...}".into()))?;
            let ((o, r), num, art) = art_issue(ctx, &a, first)?;
            let base = format!("/repos/{o}/{r}/issues/{num}/artifacts");
            let mut targets: Vec<u64> = art.into_iter().collect();
            targets.extend(pos[1..].iter().filter_map(|p| p.parse::<u64>().ok()));
            if targets.is_empty() {
                targets = arr(&ctx.get(&base)?)
                    .iter()
                    .map(|m| n(m, "number"))
                    .collect();
            }
            let output = a.get(&["-O", "--output"]);
            if output.is_some() && targets.len() != 1 {
                return usage("--output requires exactly one artifact");
            }
            let version = a.get(&["--version"]).filter(|v| *v != "0");
            if version.is_some() && targets.len() != 1 {
                return usage("--version requires exactly one artifact");
            }
            let dir = a.get(&["-D", "--dir"]).unwrap_or(".");
            for t in targets {
                let mut path = format!("{base}/{t}");
                if let Some(v) = version {
                    path.push_str(&format!("?version={v}"));
                }
                let m = ctx.get(&path)?;
                let (ext, content) = match s(&m, "type") {
                    "link" => (
                        ".url",
                        format!("[InternetShortcut]\nURL={}\n", s(&m, "body").trim()),
                    ),
                    "embedded_html" => (".html", s(&m, "body").to_string()),
                    _ => (".md", s(&m, "body").to_string()),
                };
                if output == Some("-") {
                    write!(out, "{content}")?;
                    continue;
                }
                let file = match output {
                    Some(p) => std::path::PathBuf::from(p),
                    None => {
                        let mut name = format!("{t}-{}", s(&m, "name").replace(['/', '\\'], "-"));
                        if !name.ends_with(ext) {
                            name.push_str(ext);
                        }
                        Path::new(dir).join(name)
                    }
                };
                if file.exists() && !a.has(&["--clobber"]) {
                    if a.has(&["--skip-existing"]) {
                        continue;
                    }
                    return Err(Fail::Api(format!(
                        "{} already exists (use --clobber to overwrite or --skip-existing to skip)",
                        file.display()
                    )));
                }
                if let Some(p) = file.parent() {
                    std::fs::create_dir_all(p)?;
                }
                std::fs::write(&file, content)?;
                writeln!(out, "{}", file.display())?;
            }
            Ok(())
        }
        "" | "help" | "-R" | "--repo" => Err(Fail::Help),
        other => usage(format!(
            "unknown command {other:?} for \"arks issue artifact\""
        )),
    }
}
