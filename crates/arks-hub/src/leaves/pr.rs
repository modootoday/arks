//! `arks pr …` / `arks co` — gh-symmetric pull-request client against the arks forge.
//!
//! Every gh 2.x flag of each leaf is parsed. Behaviour follows gh where the arks
//! forge has the primitive (reviews, merge methods, auto-merge, commit statuses
//! for `checks`, update-branch, revert, diff). Local Git work (`checkout`,
//! `--delete-branch` locally, `create --fill` from local commits) is read
//! in-process or printed as the `arks …` commands to run; arks never spawns git.

use super::common::{
    ListEdits, Local, body_of, edit_text, load_recover, names, parse_number, save_recover,
    title_body, upper, url_repo,
};
use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, n, row, s, usage, web};
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const HELP: &str = "\
arks pr — pull requests on the arks forge (gh-symmetric argv; host-tested Partial)

USAGE:
  arks pr <command> [flags]

COMMANDS:
  create, list, status, view, diff, edit, close, reopen, comment, merge, ready,
  review, checks, lock, unlock, update-branch, revert, checkout (alias: arks co)

Flags are gh's (`arks pr <command> --help` lists them; `-R/--repo [HOST/]OWNER/REPO`
selects the repository). `checkout` resolves the head and prints the `arks` commands
to run; arks never spawns git or gh. `--web` prints the URL instead of opening a
browser. `checks` reads arks commit statuses (exit 8 = pending, 1 = failed).";

pub(crate) const JSON_FIELDS: &[&str] = &[
    "additions",
    "assignees",
    "author",
    "autoMergeRequest",
    "baseRefName",
    "baseRefOid",
    "body",
    "changedFiles",
    "closed",
    "closedAt",
    "closingIssuesReferences",
    "comments",
    "commits",
    "createdAt",
    "deletions",
    "files",
    "fullDatabaseId",
    "headRefName",
    "headRefOid",
    "headRepository",
    "headRepositoryOwner",
    "id",
    "isCrossRepository",
    "isDraft",
    "labels",
    "latestReviews",
    "maintainerCanModify",
    "mergeCommit",
    "mergeStateStatus",
    "mergeable",
    "mergedAt",
    "mergedBy",
    "milestone",
    "number",
    "potentialMergeCommit",
    "projectCards",
    "projectItems",
    "reactionGroups",
    "reviewDecision",
    "reviewRequests",
    "reviews",
    "state",
    "statusCheckRollup",
    "title",
    "updatedAt",
    "url",
];

const CHECK_FIELDS: &[&str] = &[
    "bucket",
    "completedAt",
    "description",
    "event",
    "link",
    "name",
    "startedAt",
    "state",
    "workflow",
];

const R: &[&str] = &["-R", "--repo"];

fn vals(extra: &[&'static str]) -> Vec<&'static str> {
    let mut v: Vec<&str> = R.to_vec();
    v.extend_from_slice(extra);
    v
}

fn login_list(v: &Value, key: &str) -> Value {
    Value::Array(
        v.get(key)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|x| json!({"login": s(x, "login"), "name": ""}))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

/// gh-named JSON row for a forge pull.
pub(crate) fn gh_row(ctx: &Ctx, ctxrepo: (&str, &str), p: &Value) -> Map<String, Value> {
    let state = if p["merged"].as_bool() == Some(true) {
        "MERGED"
    } else if s(p, "state") == "closed" {
        "CLOSED"
    } else {
        "OPEN"
    };
    let reviews: Vec<Value> = super::arr(&p["reviews"])
        .iter()
        .map(|r| json!({"author": {"login": r["user"]["login"]}, "state": r["state"], "body": r["body"], "submittedAt": r["submitted_at"]}))
        .collect();
    let decision = {
        let states: Vec<&str> = super::arr(&p["reviews"])
            .iter()
            .filter_map(|r| r["state"].as_str())
            .collect();
        if states.contains(&"CHANGES_REQUESTED") && states.last() == Some(&"CHANGES_REQUESTED") {
            "CHANGES_REQUESTED"
        } else if states.contains(&"APPROVED") {
            "APPROVED"
        } else if !super::arr(&p["requested_reviewers"]).is_empty() {
            "REVIEW_REQUIRED"
        } else {
            ""
        }
    };
    let head_repo = p["head"]["repo"].clone();
    let cross = head_repo["full_name"].as_str() != p["base"]["repo"]["full_name"].as_str();
    let mut m = Map::new();
    let mut put = |k: &str, v: Value| {
        m.insert(k.to_string(), v);
    };
    put("additions", p["additions"].clone());
    put("assignees", login_list(p, "assignees"));
    put(
        "author",
        json!({"login": p["user"]["login"], "is_bot": false}),
    );
    put(
        "autoMergeRequest",
        match &p["auto_merge"] {
            Value::Null => Value::Null,
            a => {
                json!({"mergeMethod": upper(a["merge_method"].as_str().unwrap_or("merge")), "enabledBy": a["enabled_by"]})
            }
        },
    );
    put("baseRefName", p["base"]["ref"].clone());
    put("baseRefOid", p["base"]["sha"].clone());
    put("body", p["body"].clone());
    put("changedFiles", p["changed_files"].clone());
    put("closed", json!(s(p, "state") == "closed"));
    put("closedAt", p["closed_at"].clone());
    put("closingIssuesReferences", json!([]));
    put("comments", json!([]));
    put("commits", json!([]));
    put("createdAt", p["created_at"].clone());
    put("deletions", p["deletions"].clone());
    put("files", json!([]));
    put("fullDatabaseId", json!(n(p, "id").to_string()));
    put("headRefName", p["head"]["ref"].clone());
    put("headRefOid", p["head"]["sha"].clone());
    put(
        "headRepository",
        json!({"name": head_repo["name"], "id": ""}),
    );
    put(
        "headRepositoryOwner",
        json!({"login": head_repo["owner"]["login"]}),
    );
    put("id", p["node_id"].clone());
    put("isCrossRepository", json!(cross));
    put("isDraft", p["draft"].clone());
    put("labels", Value::Array(arr(&p["labels"]).iter().map(|l| json!({"name": l["name"], "color": l["color"], "description": l["description"]})).collect()));
    put("latestReviews", Value::Array(reviews.clone()));
    put("maintainerCanModify", p["maintainer_can_modify"].clone());
    put(
        "mergeCommit",
        match p["merge_commit_sha"].as_str() {
            Some(x) => json!({"oid": x}),
            None => Value::Null,
        },
    );
    put(
        "mergeStateStatus",
        json!(upper(p["mergeable_state"].as_str().unwrap_or("unknown"))),
    );
    put(
        "mergeable",
        json!(match p["mergeable"].as_bool() {
            Some(true) => "MERGEABLE",
            Some(false) => "CONFLICTING",
            None => "UNKNOWN",
        }),
    );
    put("mergedAt", p["merged_at"].clone());
    put(
        "mergedBy",
        match &p["merged_by"] {
            Value::Null => Value::Null,
            m => json!({"login": m["login"]}),
        },
    );
    put(
        "milestone",
        match &p["milestone"] {
            Value::Null => Value::Null,
            m => {
                json!({"number": m["number"], "title": m["title"], "description": m["description"]})
            }
        },
    );
    put("number", p["number"].clone());
    put("potentialMergeCommit", Value::Null);
    put("projectCards", json!([]));
    put(
        "projectItems",
        Value::Array(
            arr(&p["projects"])
                .iter()
                .map(|t| json!({"title": t}))
                .collect(),
        ),
    );
    put("reactionGroups", json!([]));
    put("reviewDecision", json!(decision));
    put(
        "reviewRequests",
        Value::Array(
            arr(&p["requested_reviewers"])
                .iter()
                .map(|r| json!({"login": r["login"]}))
                .collect(),
        ),
    );
    put("reviews", Value::Array(reviews));
    put("state", json!(state));
    put("statusCheckRollup", json!([]));
    put("title", p["title"].clone());
    put("updatedAt", p["updated_at"].clone());
    put(
        "url",
        json!(ctx.web_url(&format!(
            "/{}/{}/pull/{}",
            ctxrepo.0,
            ctxrepo.1,
            n(p, "number")
        ))),
    );
    m
}

/// Resolve a pull argument (number, URL, `#n`, or head branch; none = current branch).
fn resolve(ctx: &Ctx, a: &Args<'_>, arg: Option<&str>) -> Result<((String, String), Value), Fail> {
    let (o, r) = match arg.and_then(url_repo) {
        Some(x) => x,
        None => ctx.repo(a)?,
    };
    if let Some(num) = arg.and_then(parse_number) {
        let p = ctx.get(&format!("/repos/{o}/{r}/pulls/{num}"))?;
        return Ok(((o, r), p));
    }
    let branch = match arg {
        Some(b) => b.to_string(),
        None => Local::discover().and_then(|l| l.branch()).ok_or_else(|| {
            Fail::Usage(
                "could not determine the current branch; pass a pull request number, URL or branch"
                    .into(),
            )
        })?,
    };
    let list = ctx.get(&format!(
        "/repos/{o}/{r}/pulls?state=open&head={}",
        super::common::enc(&branch)
    ))?;
    let p = arr(&list)
        .first()
        .cloned()
        .ok_or_else(|| Fail::Api(format!("no pull requests found for branch {branch:?}")))?;
    Ok(((o, r), p))
}

/// gh-named commit row (`pr view --json commits`) from a forge commit.
fn gh_commit(c: &Value) -> Value {
    let msg = c["commit"]["message"].as_str().unwrap_or("");
    let (head, body) = msg.split_once("\n\n").unwrap_or((msg, ""));
    json!({
        "oid": c["sha"],
        "messageHeadline": head.lines().next().unwrap_or(""),
        "messageBody": body,
        "authoredDate": c["commit"]["author"]["date"],
        "committedDate": c["commit"]["committer"]["date"],
        "authors": [{
            "name": c["commit"]["author"]["name"],
            "email": c["commit"]["author"]["email"],
            "login": c["author"]["login"],
            "id": "",
        }],
    })
}

fn pr_path(o: &str, r: &str, p: &Value) -> String {
    format!("/repos/{o}/{r}/pulls/{}", n(p, "number"))
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "create" | "new" => create(ctx, rest, out),
        "list" | "ls" => list(ctx, rest, out),
        "status" => status(ctx, rest, out),
        "view" => view(ctx, rest, out),
        "diff" => diff(ctx, rest, out),
        "edit" => edit(ctx, rest, out),
        "close" => close(ctx, rest, out),
        "reopen" => reopen(ctx, rest, out),
        "comment" => comment(ctx, rest, out),
        "merge" => merge(ctx, rest, out),
        "ready" => ready(ctx, rest, out),
        "review" => review(ctx, rest, out),
        "checks" => checks(ctx, rest, out),
        "lock" | "unlock" => lock(ctx, rest, out, sub == "lock"),
        "update-branch" => update_branch(ctx, rest, out),
        "revert" => revert(ctx, rest, out),
        "checkout" => checkout(ctx, rest, out),
        "-R" | "--repo" => Err(Fail::Help),
        other => usage(format!("unknown command {other:?} for \"arks pr\"\n{HELP}")),
    }
}

/// Title/body from local commits (`--fill*`), read in-process.
fn fill(base: &str, head: &str, mode: &str) -> Result<(String, String), Fail> {
    let local =
        Local::discover().ok_or_else(|| Fail::Usage("--fill needs a local repository".into()))?;
    let g = &local.repo;
    let (Some(b), Some(h)) = (
        g.resolve(base)
            .or_else(|| g.resolve(&format!("refs/remotes/origin/{base}"))),
        g.resolve(head),
    ) else {
        return usage(format!(
            "could not find local commits between {base} and {head}"
        ));
    };
    let commits = g.range(&b, &h);
    if commits.is_empty() {
        return usage(format!("no commits between {base} and {head}"));
    }
    let msg = |c: &arks_object::Commit| {
        let m = c.message.trim();
        match m.split_once('\n') {
            Some((t, b)) => (t.trim().to_string(), b.trim().to_string()),
            None => (m.to_string(), String::new()),
        }
    };
    Ok(match mode {
        "first" => msg(&commits[0].1),
        "verbose" => {
            let title = if commits.len() == 1 {
                msg(&commits[0].1).0
            } else {
                head.replace(['-', '_'], " ")
            };
            let body = commits
                .iter()
                .map(|(_, c)| format!("* {}", c.message.trim()))
                .collect::<Vec<_>>()
                .join("\n\n");
            (title, body)
        }
        _ if commits.len() == 1 => msg(&commits[0].1),
        _ => (
            head.replace(['-', '_'], " "),
            commits
                .iter()
                .map(|(_, c)| format!("- {}", msg(c).0))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    })
}

fn create(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&[
            "-a",
            "--assignee",
            "-B",
            "--base",
            "-b",
            "--body",
            "-F",
            "--body-file",
            "-H",
            "--head",
            "-l",
            "--label",
            "-m",
            "--milestone",
            "-p",
            "--project",
            "--recover",
            "-r",
            "--reviewer",
            "-T",
            "--template",
            "-t",
            "--title",
        ]),
        &[
            "-d",
            "--draft",
            "--dry-run",
            "-e",
            "--editor",
            "-f",
            "--fill",
            "--fill-first",
            "--fill-verbose",
            "--no-maintainer-edit",
            "-w",
            "--web",
        ],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let recovered = match a.get(&["--recover"]) {
        Some(p) => load_recover(p)?,
        None => json!({}),
    };
    let head = match a.get(&["-H", "--head"]) {
        Some(h) => h.to_string(),
        None => recovered["head"]
            .as_str()
            .map(str::to_string)
            .or_else(|| Local::discover().and_then(|l| l.branch()))
            .ok_or_else(|| {
                Fail::Usage("could not determine the current branch; pass --head".into())
            })?,
    };
    let base = match a.get(&["-B", "--base"]) {
        Some(b) => b.to_string(),
        None => {
            let repo = ctx.get(&format!("/repos/{o}/{r}"))?;
            s(&repo, "default_branch").to_string()
        }
    };
    let mut title = a
        .get(&["-t", "--title"])
        .map(str::to_string)
        .or(recovered["title"].as_str().map(str::to_string));
    let mut body = body_of(&a, &["-b", "--body"], &["-F", "--body-file"])?
        .or(recovered["body"].as_str().map(str::to_string));
    let fill_mode = if a.has(&["--fill-first"]) {
        Some("first")
    } else if a.has(&["--fill-verbose"]) {
        Some("verbose")
    } else if a.has(&["-f", "--fill"]) {
        Some("fill")
    } else {
        None
    };
    if let Some(mode) = fill_mode {
        let (t, b) = fill(&base, &head, mode)?;
        title.get_or_insert(t);
        body.get_or_insert(b);
    }
    if body.is_none() {
        if let Some(t) = a.get(&["-T", "--template"]) {
            body = Some(super::read_input(Some(t))?);
        }
    }
    if a.has(&["-e", "--editor"]) {
        let seed = format!(
            "{}\n\n{}",
            title.clone().unwrap_or_default(),
            body.clone().unwrap_or_default()
        );
        let (t, b) = title_body(&edit_text(&seed)?);
        title = Some(t);
        body = Some(b);
    }
    if a.has(&["-w", "--web"]) {
        return web(
            out,
            &ctx.web_url(&format!("/{o}/{r}/compare/{base}...{head}?expand=1")),
        );
    }
    let Some(title) = title.filter(|t| !t.trim().is_empty()) else {
        return usage(
            "must provide `--title` and `--body` (or `--fill`) when not running interactively",
        );
    };
    let payload = json!({
        "title": title,
        "body": body.unwrap_or_default(),
        "head": head,
        "base": base,
        "draft": a.has(&["-d", "--draft"]),
        "maintainer_can_modify": !a.has(&["--no-maintainer-edit"]),
        "labels": a.list(&["-l", "--label"]),
        "assignees": ctx.whos(&a.list(&["-a", "--assignee"]))?,
        "reviewers": a.list(&["-r", "--reviewer"]),
        "projects": a.list(&["-p", "--project"]),
        "milestone": a.get(&["-m", "--milestone"]).map(Value::from).unwrap_or(Value::Null),
    });
    if a.has(&["--dry-run"]) {
        writeln!(out, "Would have created a Pull Request with:")?;
        writeln!(out, "title:\t{}", payload["title"].as_str().unwrap_or(""))?;
        writeln!(out, "draft:\t{}", payload["draft"])?;
        writeln!(out, "base:\t{}", payload["base"].as_str().unwrap_or(""))?;
        writeln!(out, "head:\t{}", payload["head"].as_str().unwrap_or(""))?;
        for (k, key) in [
            ("labels", "labels"),
            ("reviewers", "reviewers"),
            ("assignees", "assignees"),
            ("projects", "projects"),
        ] {
            let list: Vec<&str> = arr(&payload[key])
                .iter()
                .filter_map(Value::as_str)
                .collect();
            writeln!(out, "{k}:\t{}", list.join(", "))?;
        }
        writeln!(
            out,
            "milestones:\t{}",
            payload["milestone"].as_str().unwrap_or("")
        )?;
        writeln!(
            out,
            "maintainerCanModify:\t{}",
            payload["maintainer_can_modify"]
        )?;
        writeln!(out, "body:\n{}", payload["body"].as_str().unwrap_or(""))?;
        return Ok(());
    }
    let mut payload = payload;
    if payload["milestone"].is_null() {
        payload.as_object_mut().map(|m| m.remove("milestone"));
    }
    match ctx.api("POST", &format!("/repos/{o}/{r}/pulls"), Some(&payload)) {
        Ok(p) => {
            writeln!(
                out,
                "{}",
                ctx.web_url(&format!("/{o}/{r}/pull/{}", n(&p, "number")))
            )?;
            Ok(())
        }
        Err(e) => {
            if let Some(path) = save_recover("pr", &payload) {
                eprintln!(
                    "Failed to create pull request. Input saved to: {}",
                    path.display()
                );
                eprintln!(
                    "To recover, run: arks pr create --recover {}",
                    path.display()
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
                "-B",
                "--base",
                "-H",
                "--head",
                "-l",
                "--label",
                "-L",
                "--limit",
                "-S",
                "--search",
                "-s",
                "--state",
            ]),
            EXPORT.to_vec(),
        ]
        .concat(),
        &["-d", "--draft", "-w", "--web"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let state = a.get(&["-s", "--state"]).unwrap_or("open");
    if !matches!(state, "open" | "closed" | "merged" | "all") {
        return usage(format!(
            "invalid argument {state:?} for \"--state\" flag: valid values are {{open|closed|merged|all}}"
        ));
    }
    let limit = a.limit(30)?;
    if a.has(&["-w", "--web"]) {
        return web(out, &ctx.web_url(&format!("/{o}/{r}/pulls")));
    }
    let api_state = if state == "merged" { "closed" } else { state };
    let mut path = format!("/repos/{o}/{r}/pulls?state={api_state}");
    if let Some(b) = a.get(&["-B", "--base"]) {
        path.push_str(&format!("&base={}", super::common::enc(b)));
    }
    if let Some(h) = a.get(&["-H", "--head"]) {
        path.push_str(&format!("&head={}", super::common::enc(h)));
    }
    let mut pulls: Vec<Value> = if let Some(q) = a.get(&["-S", "--search"]) {
        let query = format!("{q} repo:{o}/{r} is:pr");
        let res = ctx.get(&format!("/search/issues?q={}", super::common::enc(&query)))?;
        res["items"].as_array().cloned().unwrap_or_default()
    } else {
        ctx.get(&path)?.as_array().cloned().unwrap_or_default()
    };
    let author = a.get(&["-A", "--author"]).map(|x| ctx.who(x)).transpose()?;
    let assignee = a
        .get(&["-a", "--assignee"])
        .map(|x| ctx.who(x))
        .transpose()?;
    let labels = a.list(&["-l", "--label"]);
    let app = a.get(&["--app"]);
    pulls.retain(|p| {
        (state != "merged" || p["merged"].as_bool() == Some(true))
            && author
                .as_deref()
                .is_none_or(|w| p["user"]["login"].as_str() == Some(w))
            && assignee.as_deref().is_none_or(|w| {
                arr(&p["assignees"])
                    .iter()
                    .any(|x| x["login"].as_str() == Some(w))
            })
            && labels.iter().all(|l| {
                arr(&p["labels"]).iter().any(|x| {
                    x["name"]
                        .as_str()
                        .is_some_and(|n| n.eq_ignore_ascii_case(l))
                })
            })
            && (!a.has(&["-d", "--draft"]) || p["draft"].as_bool() == Some(true))
            && app.is_none_or(|ap| p["user"]["login"].as_str() == Some(&format!("{ap}[bot]")))
    });
    pulls.truncate(limit);
    let rows: Vec<Map<String, Value>> = pulls.iter().map(|p| gh_row(ctx, (&o, &r), p)).collect();
    if json_out(out, &a, &rows, JSON_FIELDS, false)? {
        return Ok(());
    }
    if rows.is_empty() {
        eprintln!("no pull requests match your search in {o}/{r}");
        return Ok(());
    }
    for (p, m) in pulls.iter().zip(&rows) {
        let st = if p["draft"].as_bool() == Some(true) && s(p, "state") == "open" {
            "DRAFT".to_string()
        } else {
            m["state"].as_str().unwrap_or("").to_string()
        };
        row(
            out,
            &[
                &n(p, "number").to_string(),
                s(p, "title"),
                s(&p["head"], "ref"),
                &st,
                s(p, "created_at"),
            ],
        )?;
    }
    Ok(())
}

fn status(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[R.to_vec(), EXPORT.to_vec()].concat(),
        &["-c", "--conflict-status"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let me = ctx.me()?;
    let pulls = ctx.get(&format!("/repos/{o}/{r}/pulls?state=open"))?;
    let all = arr(&pulls).to_vec();
    let branch = Local::discover().and_then(|l| l.branch());
    let current: Vec<&Value> = all
        .iter()
        .filter(|p| branch.as_deref() == p["head"]["ref"].as_str())
        .collect();
    let mine: Vec<&Value> = all
        .iter()
        .filter(|p| p["user"]["login"].as_str() == Some(&me))
        .collect();
    let asked: Vec<&Value> = all
        .iter()
        .filter(|p| {
            arr(&p["requested_reviewers"])
                .iter()
                .any(|x| x["login"].as_str() == Some(&me))
        })
        .collect();
    if Args::parse(
        raw,
        &[R.to_vec(), EXPORT.to_vec()].concat(),
        &["-c", "--conflict-status"],
    )?
    .get(&["--json"])
    .is_some()
    {
        let section = |v: &[&Value]| {
            Value::Array(
                v.iter()
                    .map(|p| Value::Object(gh_row(ctx, (&o, &r), p)))
                    .collect(),
            )
        };
        let mut m = Map::new();
        m.insert(
            "currentBranch".into(),
            current
                .first()
                .map(|p| Value::Object(gh_row(ctx, (&o, &r), p)))
                .unwrap_or(Value::Null),
        );
        m.insert("createdBy".into(), section(&mine));
        m.insert("needsReview".into(), section(&asked));
        return super::emit(
            out,
            &Value::Object(m),
            a.get(&["-q", "--jq"]),
            a.get(&["-t", "--template"]),
        );
    }
    let line = |out: &mut dyn Write, p: &Value| -> Res {
        let mut extra = String::new();
        if a.has(&["-c", "--conflict-status"]) {
            extra = match p["mergeable"].as_bool() {
                Some(true) => " - Mergeable".into(),
                Some(false) => " - Merge conflicts".into(),
                None => " - Not mergeable (unknown)".into(),
            };
        }
        writeln!(
            out,
            "  #{}  {} [{}]{extra}",
            n(p, "number"),
            s(p, "title"),
            s(&p["head"], "ref")
        )?;
        Ok(())
    };
    writeln!(out, "\nRelevant pull requests in {o}/{r}\n")?;
    writeln!(out, "Current branch")?;
    match (branch.as_deref(), current.first()) {
        (_, Some(p)) => line(out, p)?,
        (Some(b), None) => writeln!(out, "  There is no pull request associated with [{b}]")?,
        (None, None) => writeln!(out, "  There is no current branch")?,
    }
    writeln!(out, "\nCreated by you")?;
    if mine.is_empty() {
        writeln!(out, "  You have no open pull requests")?;
    }
    for p in &mine {
        line(out, p)?;
    }
    writeln!(out, "\nRequesting a code review from you")?;
    if asked.is_empty() {
        writeln!(out, "  You have no pull requests to review")?;
    }
    for p in &asked {
        line(out, p)?;
    }
    writeln!(out)?;
    Ok(())
}

fn view(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[R.to_vec(), EXPORT.to_vec()].concat(),
        &["-c", "--comments", "-w", "--web"],
    )?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let m = gh_row(ctx, (&o, &r), &p);
    if a.has(&["-w", "--web"]) {
        return web(out, m["url"].as_str().unwrap_or(""));
    }
    let comments = || ctx.get(&format!("{}/comments", pr_path(&o, &r, &p)));
    if a.get(&["--json"]).is_some() {
        let mut m = m.clone();
        if a.get(&["--json"]).is_some_and(|j| j.contains("comments")) {
            let c = comments()?;
            m.insert(
                "comments".into(),
                Value::Array(arr(&c).iter().map(super::common::gh_comment).collect()),
            );
        }
        let wants = |f: &str| {
            a.get(&["--json"])
                .is_some_and(|j| j.split(',').any(|x| x.trim() == f))
        };
        if wants("files") {
            let f = ctx.get(&format!("{}/files", pr_path(&o, &r, &p)))?;
            m.insert("files".into(), Value::Array(arr(&f).iter().map(|f| json!({"path": f["filename"], "additions": f["additions"], "deletions": f["deletions"]})).collect()));
        }
        if wants("commits") {
            let c = ctx.get(&format!("{}/commits", pr_path(&o, &r, &p)))?;
            m.insert(
                "commits".into(),
                Value::Array(arr(&c).iter().map(gh_commit).collect()),
            );
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
    let st = if p["draft"].as_bool() == Some(true) && s(&p, "state") == "open" {
        "DRAFT"
    } else {
        m["state"].as_str().unwrap_or("")
    };
    writeln!(out, "title:\t{}", s(&p, "title"))?;
    writeln!(out, "state:\t{st}")?;
    writeln!(out, "author:\t{}", s(&p["user"], "login"))?;
    writeln!(out, "labels:\t{}", names(&p, "labels"))?;
    writeln!(out, "assignees:\t{}", names(&p, "assignees"))?;
    let reviewers: Vec<String> = arr(&p["requested_reviewers"])
        .iter()
        .map(|x| format!("{} (Requested)", s(x, "login")))
        .chain(
            arr(&p["reviews"])
                .iter()
                .map(|x| format!("{} ({})", s(&x["user"], "login"), s(x, "state"))),
        )
        .collect();
    writeln!(out, "reviewers:\t{}", reviewers.join(", "))?;
    writeln!(
        out,
        "projects:\t{}",
        arr(&p["projects"])
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    )?;
    writeln!(
        out,
        "milestone:\t{}",
        p["milestone"]["title"].as_str().unwrap_or("")
    )?;
    writeln!(out, "number:\t{}", n(&p, "number"))?;
    writeln!(out, "url:\t{}", m["url"].as_str().unwrap_or(""))?;
    writeln!(out, "additions:\t{}", n(&p, "additions"))?;
    writeln!(out, "deletions:\t{}", n(&p, "deletions"))?;
    let auto = match &p["auto_merge"] {
        Value::Null => "disabled".to_string(),
        am => format!(
            "enabled\t{}\t{}",
            s(&am["enabled_by"], "login"),
            s(am, "merge_method")
        ),
    };
    writeln!(out, "auto-merge:\t{auto}")?;
    writeln!(out, "--")?;
    writeln!(out, "{}", s(&p, "body"))?;
    Ok(())
}

fn diff(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&["--color", "-e", "--exclude"]),
        &["--name-only", "--patch", "-w", "--web"],
    )?;
    let color = a.get(&["--color"]).unwrap_or("auto");
    if !matches!(color, "always" | "never" | "auto") {
        return usage(format!(
            "invalid argument {color:?} for \"--color\" flag: valid values are {{always|never|auto}}"
        ));
    }
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    if a.has(&["-w", "--web"]) {
        return web(
            out,
            &ctx.web_url(&format!("/{o}/{r}/pull/{}/files", n(&p, "number"))),
        );
    }
    let excludes = a.list(&["-e", "--exclude"]);
    let skip = |path: &str| {
        excludes.iter().any(|g| {
            crate::forge::rules::glob(g, path)
                || crate::forge::rules::glob(g, path.rsplit('/').next().unwrap_or(path))
        })
    };
    if a.has(&["--name-only"]) {
        let files = ctx.get(&format!("{}/files", pr_path(&o, &r, &p)))?;
        for f in arr(&files) {
            if !skip(s(f, "filename")) {
                writeln!(out, "{}", s(f, "filename"))?;
            }
        }
        return Ok(());
    }
    let accept = if a.has(&["--patch"]) {
        "application/vnd.github.patch"
    } else {
        "application/vnd.github.diff"
    };
    let bytes = ctx.raw(
        "GET",
        &pr_path(&o, &r, &p),
        None,
        "application/json",
        accept,
    )?;
    let text = String::from_utf8_lossy(&bytes);
    let mut keep = true;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            keep = !skip(rest.split(" b/").next().unwrap_or(rest));
        }
        if !keep {
            continue;
        }
        if color == "always" {
            let c = if line.starts_with("+++")
                || line.starts_with("---")
                || line.starts_with("diff ")
                || line.starts_with("index ")
            {
                "\x1b[1;38m"
            } else if line.starts_with('+') {
                "\x1b[32m"
            } else if line.starts_with('-') {
                "\x1b[31m"
            } else if line.starts_with("@@") {
                "\x1b[36m"
            } else {
                ""
            };
            if c.is_empty() {
                writeln!(out, "{line}")?;
            } else {
                writeln!(out, "{c}{line}\x1b[m")?;
            }
        } else {
            writeln!(out, "{line}")?;
        }
    }
    Ok(())
}

fn edit(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&[
            "--add-assignee",
            "--add-label",
            "--add-project",
            "--add-reviewer",
            "-B",
            "--base",
            "-b",
            "--body",
            "-F",
            "--body-file",
            "-m",
            "--milestone",
            "--remove-assignee",
            "--remove-label",
            "--remove-project",
            "--remove-reviewer",
            "-t",
            "--title",
        ]),
        &["--remove-milestone"],
    )?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let mut patch = Map::new();
    if let Some(t) = a.get(&["-t", "--title"]) {
        patch.insert("title".into(), json!(t));
    }
    if let Some(b) = body_of(&a, &["-b", "--body"], &["-F", "--body-file"])? {
        patch.insert("body".into(), json!(b));
    }
    if let Some(b) = a.get(&["-B", "--base"]) {
        patch.insert("base".into(), json!(b));
    }
    let edits = ListEdits {
        labels: (a.list(&["--add-label"]), a.list(&["--remove-label"])),
        assignees: (
            ctx.whos(&a.list(&["--add-assignee"]))?,
            ctx.whos(&a.list(&["--remove-assignee"]))?,
        ),
        reviewers: (a.list(&["--add-reviewer"]), a.list(&["--remove-reviewer"])),
    };
    let (add, rm) = (a.list(&["--add-project"]), a.list(&["--remove-project"]));
    if !add.is_empty() || !rm.is_empty() {
        let mut v: Vec<String> = arr(&p["projects"])
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
        ctx.api("PATCH", &pr_path(&o, &r, &p), Some(&Value::Object(patch)))?;
    }
    edits.apply(ctx, &pr_path(&o, &r, &p))?;
    writeln!(
        out,
        "{}",
        ctx.web_url(&format!("/{o}/{r}/pull/{}", n(&p, "number")))
    )?;
    Ok(())
}

fn add_comment(ctx: &Ctx, o: &str, r: &str, p: &Value, body: &str) -> Result<Value, Fail> {
    ctx.api(
        "POST",
        &format!("{}/comments", pr_path(o, r, p)),
        Some(&json!({"body": body})),
    )
}

fn delete_head(ctx: &Ctx, o: &str, r: &str, p: &Value) -> Res {
    let head = s(&p["head"], "ref");
    if p["head"]["repo"]["full_name"].as_str() == Some(&format!("{o}/{r}")) && !head.is_empty() {
        match ctx.api(
            "DELETE",
            &format!("/repos/{o}/{r}/git/refs/heads/{head}"),
            None,
        ) {
            Ok(_) => eprintln!("✓ Deleted branch {head} on {o}/{r}"),
            Err(e) => eprintln!("! could not delete remote branch {head}: {e:?}"),
        }
    }
    if Local::discover().and_then(|l| l.branch()).is_some() {
        eprintln!("note: delete the local branch with: arks branch -D {head}");
    }
    Ok(())
}

fn close(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["-c", "--comment"]), &["-d", "--delete-branch"])?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    if s(&p, "state") == "closed" {
        eprintln!(
            "! Pull request {o}/{r}#{} ({}) is already closed",
            n(&p, "number"),
            s(&p, "title")
        );
        return Ok(());
    }
    if let Some(c) = a.get(&["-c", "--comment"]) {
        add_comment(ctx, &o, &r, &p, c)?;
    }
    ctx.api(
        "PATCH",
        &pr_path(&o, &r, &p),
        Some(&json!({"state": "closed"})),
    )?;
    eprintln!(
        "✓ Closed pull request {o}/{r}#{} ({})",
        n(&p, "number"),
        s(&p, "title")
    );
    if a.has(&["-d", "--delete-branch"]) {
        delete_head(ctx, &o, &r, &p)?;
    }
    let _ = out;
    Ok(())
}

fn reopen(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &vals(&["-c", "--comment"]), &[])?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    if p["merged"].as_bool() == Some(true) {
        return Err(Fail::Api(format!(
            "Pull request {o}/{r}#{} ({}) can't be reopened because it was already merged",
            n(&p, "number"),
            s(&p, "title")
        )));
    }
    if s(&p, "state") == "open" {
        eprintln!(
            "! Pull request {o}/{r}#{} ({}) is already open",
            n(&p, "number"),
            s(&p, "title")
        );
        return Ok(());
    }
    if let Some(c) = a.get(&["-c", "--comment"]) {
        add_comment(ctx, &o, &r, &p, c)?;
    }
    ctx.api(
        "PATCH",
        &pr_path(&o, &r, &p),
        Some(&json!({"state": "open"})),
    )?;
    eprintln!(
        "✓ Reopened pull request {o}/{r}#{} ({})",
        n(&p, "number"),
        s(&p, "title")
    );
    let _ = out;
    Ok(())
}

/// Shared `pr comment` / `issue comment` flow over a comments collection path.
pub(crate) fn comment_flow(
    ctx: &Ctx,
    a: &Args<'_>,
    list_path: &str,
    web_url: &str,
    out: &mut dyn Write,
) -> Res {
    if a.has(&["-w", "--web"]) {
        return web(out, web_url);
    }
    let (o_r, _) = list_path
        .trim_start_matches("/repos/")
        .split_once("/issues/")
        .or_else(|| {
            list_path
                .trim_start_matches("/repos/")
                .split_once("/pulls/")
        })
        .unwrap_or(("", ""));
    let me = ctx.me()?;
    let last = || -> Result<Option<Value>, Fail> {
        let all = ctx.get(list_path)?;
        Ok(arr(&all)
            .iter()
            .rev()
            .find(|c| c["user"]["login"].as_str() == Some(&me))
            .cloned())
    };
    if a.has(&["--delete-last"]) {
        let Some(c) = last()? else {
            return Err(Fail::Api("no comments found for current user".into()));
        };
        if !a.has(&["--yes"]) {
            return usage("--yes required when not running interactively");
        }
        ctx.api(
            "DELETE",
            &format!("/repos/{o_r}/issues/comments/{}", n(&c, "id")),
            None,
        )?;
        eprintln!("Comment deleted");
        return Ok(());
    }
    let mut body = body_of(a, &["-b", "--body"], &["-F", "--body-file"])?;
    if a.has(&["-e", "--editor"]) {
        body = Some(edit_text(body.as_deref().unwrap_or(""))?.trim().to_string());
    }
    let Some(body) = body.filter(|b| !b.trim().is_empty()) else {
        return usage(
            "`--body`, `--body-file` or `--editor` required when not running interactively",
        );
    };
    if a.has(&["--create-if-none"]) && !a.has(&["--edit-last"]) {
        return usage("`--create-if-none` can only be used with `--edit-last`");
    }
    let c = if a.has(&["--edit-last"]) {
        match last()? {
            Some(c) => ctx.api(
                "PATCH",
                &format!("/repos/{o_r}/issues/comments/{}", n(&c, "id")),
                Some(&json!({"body": body})),
            )?,
            None if a.has(&["--create-if-none"]) => {
                ctx.api("POST", list_path, Some(&json!({"body": body})))?
            }
            None => return Err(Fail::Api("no comments found for current user".into())),
        }
    } else {
        ctx.api("POST", list_path, Some(&json!({"body": body})))?
    };
    writeln!(out, "{}", s(&c, "html_url"))?;
    Ok(())
}

pub(crate) const COMMENT_VALUES: &[&str] = &["-R", "--repo", "-b", "--body", "-F", "--body-file"];
pub(crate) const COMMENT_BOOLS: &[&str] = &[
    "--create-if-none",
    "--delete-last",
    "--edit-last",
    "-e",
    "--editor",
    "-w",
    "--web",
    "--yes",
];

fn comment(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, COMMENT_VALUES, COMMENT_BOOLS)?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let url = ctx.web_url(&format!("/{o}/{r}/pull/{}", n(&p, "number")));
    comment_flow(
        ctx,
        &a,
        &format!("{}/comments", pr_path(&o, &r, &p)),
        &url,
        out,
    )
}

fn merge(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&[
            "-A",
            "--author-email",
            "-b",
            "--body",
            "-F",
            "--body-file",
            "--match-head-commit",
            "-t",
            "--subject",
        ]),
        &[
            "--admin",
            "--auto",
            "-d",
            "--delete-branch",
            "--disable-auto",
            "-m",
            "--merge",
            "-r",
            "--rebase",
            "-s",
            "--squash",
        ],
    )?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let methods: Vec<&str> = [
        ("merge", &["-m", "--merge"][..]),
        ("rebase", &["-r", "--rebase"]),
        ("squash", &["-s", "--squash"]),
    ]
    .iter()
    .filter(|(_, f)| a.has(f))
    .map(|(m, _)| *m)
    .collect();
    if methods.len() > 1 {
        return usage("only one of --merge, --rebase, or --squash can be enabled");
    }
    let num = n(&p, "number");
    if a.has(&["--disable-auto"]) {
        ctx.api(
            "DELETE",
            &format!("{}/auto-merge", pr_path(&o, &r, &p)),
            None,
        )?;
        eprintln!("✓ Auto-merge disabled for pull request {o}/{r}#{num}");
        return Ok(());
    }
    let Some(method) = methods.first().copied() else {
        return usage("--merge, --rebase, or --squash required when not running interactively");
    };
    if a.has(&["--auto"]) {
        let v = ctx.api(
            "PUT",
            &format!("{}/auto-merge", pr_path(&o, &r, &p)),
            Some(&json!({"merge_method": method})),
        )?;
        if v["merged"].as_bool() == Some(true) {
            eprintln!("✓ Merged pull request {o}/{r}#{num} ({})", s(&p, "title"));
        } else {
            eprintln!(
                "✓ Pull request {o}/{r}#{num} will be automatically merged via {method} when all requirements are met"
            );
        }
        let _ = out;
        return Ok(());
    }
    let mut body = json!({"merge_method": method, "admin": a.has(&["--admin"])});
    if let Some(t) = a.get(&["-t", "--subject"]) {
        body["commit_title"] = json!(t);
    }
    if let Some(b) = body_of(&a, &["-b", "--body"], &["-F", "--body-file"])? {
        body["commit_message"] = json!(b);
    }
    if let Some(sha) = a.get(&["--match-head-commit"]) {
        body["sha"] = json!(sha);
    }
    if let Some(e) = a.get(&["-A", "--author-email"]) {
        body["author_email"] = json!(e);
    }
    let res = ctx.api(
        "PUT",
        &format!("{}/merge", pr_path(&o, &r, &p)),
        Some(&body),
    )?;
    let verb = match method {
        "squash" => "Squashed and merged",
        "rebase" => "Rebased and merged",
        _ => "Merged",
    };
    eprintln!("✓ {verb} pull request {o}/{r}#{num} ({})", s(&p, "title"));
    writeln!(out, "sha={}", s(&res, "sha"))?;
    if a.has(&["-d", "--delete-branch"]) {
        delete_head(ctx, &o, &r, &p)?;
    }
    Ok(())
}

fn ready(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, R, &["--undo"])?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let undo = a.has(&["--undo"]);
    let v = ctx.api("PATCH", &pr_path(&o, &r, &p), Some(&json!({"draft": undo})))?;
    if undo {
        eprintln!(
            "✓ Pull request {o}/{r}#{} is converted to \"draft\"",
            n(&p, "number")
        );
    } else {
        eprintln!(
            "✓ Pull request {o}/{r}#{} is marked as \"ready for review\"",
            n(&p, "number")
        );
    }
    writeln!(
        out,
        "number={}\ndraft={}\nready={}",
        n(&v, "number"),
        v["draft"],
        v["draft"].as_bool() == Some(false)
    )?;
    Ok(())
}

fn review(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&["-b", "--body", "-F", "--body-file", "-e", "--event"]),
        &[
            "-a",
            "--approve",
            "-c",
            "--comment",
            "-r",
            "--request-changes",
        ],
    )?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let picked: Vec<&str> = [
        ("APPROVE", &["-a", "--approve"][..]),
        ("COMMENT", &["-c", "--comment"]),
        ("REQUEST_CHANGES", &["-r", "--request-changes"]),
    ]
    .iter()
    .filter(|(_, f)| a.has(f))
    .map(|(e, _)| *e)
    .collect();
    if picked.len() > 1 {
        return usage("need exactly one of --approve, --request-changes, or --comment");
    }
    // `--event` is the legacy arks spelling; gh selects the event by flag.
    let event = picked.first().copied().or(a.get(&["-e", "--event"])).ok_or_else(|| Fail::Usage("--approve, --request-changes, or --comment required when not running interactively".into()))?;
    let body = body_of(&a, &["-b", "--body"], &["-F", "--body-file"])?.unwrap_or_default();
    let v = ctx.api(
        "POST",
        &format!("{}/reviews", pr_path(&o, &r, &p)),
        Some(&json!({"event": event, "body": body})),
    )?;
    let num = n(&p, "number");
    match event {
        "APPROVE" => eprintln!("✓ Approved pull request {o}/{r}#{num}"),
        "REQUEST_CHANGES" => eprintln!("+ Requested changes to pull request {o}/{r}#{num}"),
        _ => eprintln!("- Reviewed pull request {o}/{r}#{num}"),
    }
    writeln!(
        out,
        "id={}\nstate={}\nevent={event}",
        n(&v, "id"),
        s(&v, "state")
    )?;
    Ok(())
}

fn bucket(state: &str) -> &'static str {
    match state {
        "success" => "pass",
        "failure" | "error" => "fail",
        _ => "pending",
    }
}

fn checks(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[vals(&["-i", "--interval"]), EXPORT.to_vec()].concat(),
        &["--fail-fast", "--required", "--watch", "-w", "--web"],
    )?;
    let interval: u64 = match a.get(&["-i", "--interval"]) {
        Some(i) => i.parse().map_err(|_| {
            Fail::Usage(format!(
                "invalid argument {i:?} for \"-i, --interval\" flag"
            ))
        })?,
        None => 10,
    };
    if a.has(&["--fail-fast"]) && !a.has(&["--watch"]) {
        return usage("cannot use `--fail-fast` flag without `--watch` flag");
    }
    if a.get(&["-i", "--interval"]).is_some() && !a.has(&["--watch"]) {
        return usage("cannot use `--interval` flag without `--watch` flag");
    }
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let num = n(&p, "number");
    if a.has(&["-w", "--web"]) {
        return web(out, &ctx.web_url(&format!("/{o}/{r}/pull/{num}/checks")));
    }
    let required: Vec<String> = if a.has(&["--required"]) {
        let rules = ctx.get(&format!(
            "/repos/{o}/{r}/rules/branches/{}",
            s(&p["base"], "ref")
        ))?;
        arr(&rules)
            .iter()
            .filter(|x| s(x, "type") == "required_status_checks")
            .flat_map(|x| {
                arr(&x["parameters"]["required_status_checks"])
                    .iter()
                    .map(|c| s(c, "context").to_string())
                    .collect::<Vec<_>>()
            })
            .collect()
    } else {
        Vec::new()
    };
    let sha = p["head"]["sha"]
        .as_str()
        .unwrap_or_else(|| s(&p["head"], "ref"))
        .to_string();
    loop {
        let combined = ctx.get(&format!(
            "/repos/{o}/{r}/commits/{}/status",
            super::common::seg(&sha)
        ))?;
        let mut list: Vec<Value> = arr(&combined["statuses"]).to_vec();
        if a.has(&["--required"]) {
            list.retain(|c| required.iter().any(|q| q == s(c, "context")));
            for q in &required {
                if !list.iter().any(|c| s(c, "context") == q) {
                    list.push(json!({"context": q, "state": "pending", "description": "Expected — waiting for status to be reported", "target_url": ""}));
                }
            }
        }
        if list.is_empty() {
            if a.has(&["--required"]) {
                return Err(Fail::Api(format!(
                    "no required checks reported on the '{}' branch",
                    s(&p["head"], "ref")
                )));
            }
            return Err(Fail::Api(format!(
                "no checks reported on the '{}' branch",
                s(&p["head"], "ref")
            )));
        }
        let rows: Vec<Map<String, Value>> = list
            .iter()
            .map(|c| {
                let mut m = Map::new();
                m.insert("bucket".into(), json!(bucket(s(c, "state"))));
                m.insert(
                    "completedAt".into(),
                    if s(c, "state") == "pending" {
                        Value::Null
                    } else {
                        c["updated_at"].clone()
                    },
                );
                m.insert("description".into(), c["description"].clone());
                m.insert("event".into(), json!(""));
                m.insert("link".into(), c["target_url"].clone());
                m.insert("name".into(), c["context"].clone());
                m.insert("startedAt".into(), c["created_at"].clone());
                m.insert("state".into(), json!(upper(s(c, "state"))));
                m.insert("workflow".into(), json!(""));
                m
            })
            .collect();
        let failed = rows.iter().filter(|m| m["bucket"] == "fail").count();
        let pending = rows.iter().filter(|m| m["bucket"] == "pending").count();
        let watching =
            a.has(&["--watch"]) && pending > 0 && !(a.has(&["--fail-fast"]) && failed > 0);
        if !watching {
            if json_out(out, &a, &rows, CHECK_FIELDS, false)? {
                return Ok(());
            }
            for (c, m) in list.iter().zip(&rows) {
                let elapsed = match (
                    super::common::iso_secs(s(c, "created_at")),
                    super::common::iso_secs(s(c, "updated_at")),
                ) {
                    (Some(a), Some(b)) if b >= a && s(c, "state") != "pending" => {
                        format!("{}s", b - a)
                    }
                    _ => "0".into(),
                };
                row(
                    out,
                    &[
                        s(c, "context"),
                        m["bucket"].as_str().unwrap_or(""),
                        &elapsed,
                        s(c, "target_url"),
                        s(c, "description"),
                    ],
                )?;
            }
            return if failed > 0 {
                Err(Fail::Exit(1))
            } else if pending > 0 {
                Err(Fail::Exit(8))
            } else {
                Ok(())
            };
        }
        std::thread::sleep(std::time::Duration::from_secs(interval.max(1)));
    }
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
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let path = format!("{}/lock", pr_path(&o, &r, &p));
    if on {
        ctx.api("PUT", &path, Some(&json!({"lock_reason": reason})))?;
        eprintln!(
            "✓ Locked as {} conversation on pull request {o}/{r}#{}",
            reason.unwrap_or("").replace('_', " "),
            n(&p, "number")
        );
    } else {
        ctx.api("DELETE", &path, None)?;
        eprintln!(
            "✓ Unlocked conversation on pull request {o}/{r}#{}",
            n(&p, "number")
        );
    }
    writeln!(out, "number={}\nlocked={on}", n(&p, "number"))?;
    Ok(())
}

fn update_branch(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, R, &["--rebase"])?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let method = if a.has(&["--rebase"]) {
        "rebase"
    } else {
        "merge"
    };
    let body = json!({"update_method": method, "expected_head_sha": p["head"]["sha"]});
    match ctx.api(
        "PUT",
        &format!("{}/update-branch", pr_path(&o, &r, &p)),
        Some(&body),
    ) {
        Ok(_) => {
            eprintln!("✓ PR branch updated");
            Ok(())
        }
        Err(Fail::Api(m)) if m.contains("no new commits") => {
            eprintln!("✓ PR branch already up-to-date");
            let _ = out;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn revert(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&["-b", "--body", "-F", "--body-file", "-t", "--title"]),
        &["-d", "--draft"],
    )?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let mut body = json!({"draft": a.has(&["-d", "--draft"])});
    if let Some(t) = a.get(&["-t", "--title"]) {
        body["title"] = json!(t);
    }
    if let Some(b) = body_of(&a, &["-b", "--body"], &["-F", "--body-file"])? {
        body["body"] = json!(b);
    }
    let v = ctx.api(
        "POST",
        &format!("{}/revert", pr_path(&o, &r, &p)),
        Some(&body),
    )?;
    writeln!(
        out,
        "{}",
        ctx.web_url(&format!("/{o}/{r}/pull/{}", n(&v, "number")))
    )?;
    Ok(())
}

/// Resolve the PR head and print the checkout recipe (never spawns git/gh).
fn checkout(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&["-b", "--branch"]),
        &["--detach", "-f", "--force", "--recurse-submodules"],
    )?;
    let ((o, r), p) = resolve(ctx, &a, a.positionals().first().copied())?;
    let number = n(&p, "number");
    let head = s(&p["head"], "ref").to_string();
    let local = a.get(&["-b", "--branch"]).unwrap_or(&head).to_string();
    writeln!(out, "pr={number}")?;
    writeln!(out, "head_ref={head}")?;
    if let Some(sha) = p["head"]["sha"].as_str() {
        writeln!(out, "head_sha={sha}")?;
    }
    let fetch = format!("arks fetch origin pull/{number}/head:{local}");
    let mut recipe = if a.has(&["--detach"]) {
        format!("arks fetch origin pull/{number}/head && arks checkout --detach FETCH_HEAD")
    } else if a.has(&["-f", "--force"]) {
        format!("arks fetch origin +pull/{number}/head:{local} && arks checkout {local}")
    } else {
        format!("{fetch} && arks checkout {local}")
    };
    if a.has(&["--recurse-submodules"]) {
        recipe.push_str(" && arks submodule update --init --recursive");
    }
    if !a.passthrough().is_empty() {
        eprintln!("note: arguments after `--` are for gh's git; arks prints its own commands");
    }
    writeln!(out, "recipe={recipe}")?;
    eprintln!(
        "note: pr checkout resolves the head and prints the arks commands to run; it never spawns git or gh ({o}/{r})"
    );
    Ok(())
}
