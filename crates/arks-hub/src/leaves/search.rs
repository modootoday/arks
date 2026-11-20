//! `arks search {issues|prs|repos|code|commits}` — gh-symmetric search client.
//! Flags become gh's search qualifiers (`--label x` → `label:x`), evaluated by
//! the arks forge search API.

use super::common::enc;
use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, n, row, s, usage, web};
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const HELP: &str = "\
arks search — search the arks forge (gh-symmetric argv; host-tested Partial)

USAGE:
  arks search {issues|prs|repos|code|commits} [<query>] [flags]

Flags map to search qualifiers (`--label bug` → `label:bug`, `--owner o` → `user:o`).";

const ISSUE_FIELDS: &[&str] = &[
    "assignees",
    "author",
    "authorAssociation",
    "body",
    "closedAt",
    "commentsCount",
    "createdAt",
    "id",
    "isLocked",
    "isPullRequest",
    "labels",
    "number",
    "repository",
    "state",
    "title",
    "updatedAt",
    "url",
];
const PR_FIELDS: &[&str] = &[
    "assignees",
    "author",
    "authorAssociation",
    "body",
    "closedAt",
    "commentsCount",
    "createdAt",
    "id",
    "isDraft",
    "isLocked",
    "isPullRequest",
    "labels",
    "number",
    "repository",
    "state",
    "title",
    "updatedAt",
    "url",
];
const REPO_FIELDS: &[&str] = &[
    "createdAt",
    "defaultBranch",
    "description",
    "forksCount",
    "fullName",
    "hasDownloads",
    "hasIssues",
    "hasPages",
    "hasProjects",
    "hasWiki",
    "homepage",
    "id",
    "isArchived",
    "isDisabled",
    "isFork",
    "isPrivate",
    "language",
    "license",
    "name",
    "openIssuesCount",
    "owner",
    "pushedAt",
    "size",
    "stargazersCount",
    "updatedAt",
    "url",
    "visibility",
    "watchersCount",
];
const CODE_FIELDS: &[&str] = &["path", "repository", "sha", "textMatches", "url"];
const COMMIT_FIELDS: &[&str] = &[
    "author",
    "commit",
    "committer",
    "id",
    "parents",
    "repository",
    "sha",
    "url",
];

/// Value flags → qualifier key, repeatable.
type Map2 = &'static [(&'static [&'static str], &'static str)];

const ISSUE_VALUES: Map2 = &[
    (&["--assignee"], "assignee"),
    (&["--author"], "author"),
    (&["--closed"], "closed"),
    (&["--commenter"], "commenter"),
    (&["--comments"], "comments"),
    (&["--created"], "created"),
    (&["--interactions"], "interactions"),
    (&["--involves"], "involves"),
    (&["--label"], "label"),
    (&["--language"], "language"),
    (&["--match"], "in"),
    (&["--mentions"], "mentions"),
    (&["--milestone"], "milestone"),
    (&["--owner"], "user"),
    (&["--project"], "project"),
    (&["--reactions"], "reactions"),
    (&["-R", "--repo"], "repo"),
    (&["--state"], "state"),
    (&["--team-mentions"], "team"),
    (&["--updated"], "updated"),
];
const PR_EXTRA_VALUES: Map2 = &[
    (&["-B", "--base"], "base"),
    (&["--checks"], "status"),
    (&["-H", "--head"], "head"),
    (&["--merged-at"], "merged"),
    (&["--review"], "review"),
    (&["--review-requested"], "review-requested"),
    (&["--reviewed-by"], "reviewed-by"),
];
const REPO_VALUES: Map2 = &[
    (&["--created"], "created"),
    (&["--followers"], "followers"),
    (&["--forks"], "forks"),
    (&["--good-first-issues"], "good-first-issues"),
    (&["--help-wanted-issues"], "help-wanted-issues"),
    (&["--include-forks"], "fork"),
    (&["--language"], "language"),
    (&["--license"], "license"),
    (&["--match"], "in"),
    (&["--number-topics"], "topics"),
    (&["--owner"], "user"),
    (&["--size"], "size"),
    (&["--stars"], "stars"),
    (&["--topic"], "topic"),
    (&["--updated"], "pushed"),
];
const CODE_VALUES: Map2 = &[
    (&["--extension"], "extension"),
    (&["--filename"], "filename"),
    (&["--language"], "language"),
    (&["--match"], "in"),
    (&["--owner"], "user"),
    (&["-R", "--repo"], "repo"),
    (&["--size"], "size"),
];
const COMMIT_VALUES: Map2 = &[
    (&["--author"], "author"),
    (&["--author-date"], "author-date"),
    (&["--author-email"], "author-email"),
    (&["--author-name"], "author-name"),
    (&["--committer"], "committer"),
    (&["--committer-date"], "committer-date"),
    (&["--committer-email"], "committer-email"),
    (&["--committer-name"], "committer-name"),
    (&["--hash"], "hash"),
    (&["--owner"], "user"),
    (&["--parent"], "parent"),
    (&["-R", "--repo"], "repo"),
    (&["--tree"], "tree"),
];

fn flags_of(m: Map2) -> Vec<&'static str> {
    m.iter().flat_map(|(f, _)| f.iter().copied()).collect()
}

fn quote(v: &str) -> String {
    if v.contains(' ') {
        format!("\"{v}\"")
    } else {
        v.to_string()
    }
}

fn build(ctx: &Ctx, a: &Args<'_>, m: Map2, q: &mut Vec<String>) -> Result<(), Fail> {
    for (flags, key) in m {
        for v in a.get_all(flags) {
            let vals: Vec<String> =
                if matches!(*key, "label" | "user" | "repo" | "in" | "license" | "topic") {
                    v.split(',')
                        .map(str::trim)
                        .filter(|x| !x.is_empty())
                        .map(str::to_string)
                        .collect()
                } else {
                    vec![v.to_string()]
                };
            for x in vals {
                let x = if matches!(
                    *key,
                    "assignee"
                        | "author"
                        | "commenter"
                        | "involves"
                        | "mentions"
                        | "review-requested"
                        | "reviewed-by"
                ) {
                    ctx.who(&x)?
                } else {
                    x
                };
                q.push(format!("{key}:{}", quote(&x)));
            }
        }
    }
    Ok(())
}

fn visibility(a: &Args<'_>, q: &mut Vec<String>) -> Res {
    for v in a.list(&["--visibility"]) {
        if !matches!(v.as_str(), "public" | "private" | "internal") {
            return usage(format!(
                "invalid argument {v:?} for \"--visibility\" flag: valid values are {{public|private|internal}}"
            ));
        }
        q.push(format!("is:{v}"));
    }
    Ok(())
}

fn sort_order(a: &Args<'_>, allowed: &[&str]) -> Result<String, Fail> {
    let mut extra = String::new();
    if let Some(srt) = a.get(&["--sort"]) {
        if !allowed.contains(&srt) {
            return usage(format!(
                "invalid argument {srt:?} for \"--sort\" flag: valid values are {{{}}}",
                allowed.join("|")
            ));
        }
        extra.push_str(&format!("&sort={srt}"));
    }
    if let Some(o) = a.get(&["--order"]) {
        if !matches!(o, "asc" | "desc") {
            return usage(format!(
                "invalid argument {o:?} for \"--order\" flag: valid values are {{asc|desc}}"
            ));
        }
        extra.push_str(&format!("&order={o}"));
    }
    Ok(extra)
}

fn issue_row(x: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    let pr = x["is_pull_request"].as_bool().unwrap_or(false);
    for (k, v) in [
        (
            "assignees",
            Value::Array(
                arr(&x["assignees"])
                    .iter()
                    .map(|a| json!({"login": a["login"]}))
                    .collect(),
            ),
        ),
        ("author", json!({"login": x["user"]["login"]})),
        ("authorAssociation", json!("OWNER")),
        ("body", x["body"].clone()),
        ("closedAt", x["closed_at"].clone()),
        ("commentsCount", x["comments"].clone()),
        ("createdAt", x["created_at"].clone()),
        ("id", x["node_id"].clone()),
        ("isDraft", x["draft"].clone()),
        ("isLocked", x["locked"].clone()),
        ("isPullRequest", json!(pr)),
        (
            "labels",
            Value::Array(
                arr(&x["labels"])
                    .iter()
                    .map(|l| json!({"name": l["name"], "color": l["color"]}))
                    .collect(),
            ),
        ),
        ("number", x["number"].clone()),
        (
            "repository",
            json!({"name": x["repository"]["name"], "nameWithOwner": x["repository"]["full_name"]}),
        ),
        ("state", x["state"].clone()),
        ("title", x["title"].clone()),
        ("updatedAt", x["updated_at"].clone()),
        ("url", x["html_url"].clone()),
    ] {
        m.insert(k.into(), v);
    }
    m
}

fn repo_row(x: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in [
        ("createdAt", x["created_at"].clone()),
        ("defaultBranch", x["default_branch"].clone()),
        ("description", x["description"].clone()),
        ("forksCount", x["forks_count"].clone()),
        ("fullName", x["full_name"].clone()),
        ("hasDownloads", json!(true)),
        ("hasIssues", x["has_issues"].clone()),
        ("hasPages", json!(false)),
        ("hasProjects", x["has_projects"].clone()),
        ("hasWiki", x["has_wiki"].clone()),
        ("homepage", x["homepage"].clone()),
        ("id", x["node_id"].clone()),
        ("isArchived", x["archived"].clone()),
        ("isDisabled", json!(false)),
        ("isFork", x["fork"].clone()),
        ("isPrivate", x["private"].clone()),
        ("language", x["language"].clone()),
        ("license", x["license"].clone()),
        ("name", x["name"].clone()),
        ("openIssuesCount", x["open_issues_count"].clone()),
        ("owner", json!({"login": x["owner"]["login"]})),
        ("pushedAt", x["pushed_at"].clone()),
        ("size", x["size"].clone()),
        ("stargazersCount", x["stargazers_count"].clone()),
        ("updatedAt", x["updated_at"].clone()),
        ("url", x["html_url"].clone()),
        ("visibility", x["visibility"].clone()),
        ("watchersCount", x["watchers_count"].clone()),
    ] {
        m.insert(k.into(), v);
    }
    m
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    let (kind, values, bools, fields): (&str, Vec<&str>, Vec<&str>, &[&str]) = match sub {
        "issues" => (
            "issues",
            [
                flags_of(ISSUE_VALUES),
                vec![
                    "--app",
                    "--visibility",
                    "-L",
                    "--limit",
                    "--order",
                    "--sort",
                ],
            ]
            .concat(),
            vec![
                "--archived",
                "--include-prs",
                "--locked",
                "--no-assignee",
                "--no-label",
                "--no-milestone",
                "--no-project",
                "-w",
                "--web",
            ],
            ISSUE_FIELDS,
        ),
        "prs" | "pulls" => (
            "prs",
            [
                flags_of(ISSUE_VALUES),
                flags_of(PR_EXTRA_VALUES),
                vec![
                    "--app",
                    "--visibility",
                    "-L",
                    "--limit",
                    "--order",
                    "--sort",
                ],
            ]
            .concat(),
            vec![
                "--archived",
                "--draft",
                "--locked",
                "--merged",
                "--no-assignee",
                "--no-label",
                "--no-milestone",
                "--no-project",
                "-w",
                "--web",
            ],
            PR_FIELDS,
        ),
        "repos" => (
            "repos",
            [
                flags_of(REPO_VALUES),
                vec!["--visibility", "-L", "--limit", "--order", "--sort"],
            ]
            .concat(),
            vec!["--archived", "-w", "--web"],
            REPO_FIELDS,
        ),
        "code" => (
            "code",
            [flags_of(CODE_VALUES), vec!["-L", "--limit"]].concat(),
            vec!["-w", "--web"],
            CODE_FIELDS,
        ),
        "commits" => (
            "commits",
            [
                flags_of(COMMIT_VALUES),
                vec!["--visibility", "-L", "--limit", "--order", "--sort"],
            ]
            .concat(),
            vec!["--merge", "-w", "--web"],
            COMMIT_FIELDS,
        ),
        other => {
            return usage(format!(
                "unknown command {other:?} for \"arks search\"\n{HELP}"
            ));
        }
    };
    let a = Args::parse(raw, &[values, EXPORT.to_vec()].concat(), &bools)?;
    let mut q: Vec<String> = a.positionals().iter().map(|t| t.to_string()).collect();
    match kind {
        "issues" | "prs" => {
            build(ctx, &a, ISSUE_VALUES, &mut q)?;
            if kind == "prs" {
                build(ctx, &a, PR_EXTRA_VALUES, &mut q)?;
                q.push("type:pr".into());
                if a.has(&["--draft"]) {
                    q.push("draft:true".into());
                }
                if a.has(&["--merged"]) {
                    q.push("is:merged".into());
                }
            } else if !a.has(&["--include-prs"]) {
                q.push("type:issue".into());
            }
            if let Some(app) = a.get(&["--app"]) {
                q.push(format!("author:app/{app}"));
            }
            if let Some(b) = a.bool_opt(&["--archived"]) {
                q.push(format!("archived:{b}"));
            }
            for (flag, qual) in [
                ("--locked", "is:locked"),
                ("--no-assignee", "no:assignee"),
                ("--no-label", "no:label"),
                ("--no-milestone", "no:milestone"),
                ("--no-project", "no:project"),
            ] {
                if a.has(&[flag]) {
                    q.push(qual.into());
                }
            }
            visibility(&a, &mut q)?;
        }
        "repos" => {
            build(ctx, &a, REPO_VALUES, &mut q)?;
            if let Some(b) = a.bool_opt(&["--archived"]) {
                q.push(format!("archived:{b}"));
            }
            visibility(&a, &mut q)?;
        }
        "code" => build(ctx, &a, CODE_VALUES, &mut q)?,
        _ => {
            build(ctx, &a, COMMIT_VALUES, &mut q)?;
            if let Some(b) = a.bool_opt(&["--merge"]) {
                q.push(format!("merge:{b}"));
            }
            visibility(&a, &mut q)?;
        }
    }
    let query = q.join(" ");
    if a.has(&["-w", "--web"]) {
        let t = match kind {
            "prs" => "pullrequests",
            "repos" => "repositories",
            k => k,
        };
        return web(
            out,
            &ctx.web_url(&format!("/search?q={}&type={t}", enc(&query))),
        );
    }
    if q.is_empty() {
        return usage("specify search keywords or flags");
    }
    let sorts: &[&str] = match kind {
        "issues" | "prs" => &[
            "comments",
            "created",
            "interactions",
            "reactions",
            "reactions-+1",
            "reactions--1",
            "reactions-heart",
            "reactions-smile",
            "reactions-tada",
            "reactions-thinking_face",
            "updated",
        ],
        "repos" => &["forks", "help-wanted-issues", "stars", "updated"],
        "commits" => &["author-date", "committer-date"],
        _ => &[],
    };
    let extra = if sorts.is_empty() {
        String::new()
    } else {
        sort_order(&a, sorts)?
    };
    let limit = a.limit(30)?;
    let api_kind = match kind {
        "prs" | "issues" => "issues",
        "repos" => "repositories",
        k => k,
    };
    let res = ctx.get(&format!(
        "/search/{api_kind}?q={}&per_page={limit}{extra}",
        enc(&query)
    ))?;
    let items = arr(&res["items"]);
    let rows: Vec<Map<String, Value>> = match kind {
        "issues" | "prs" => items.iter().map(issue_row).collect(),
        "repos" => items.iter().map(repo_row).collect(),
        "code" => items
            .iter()
            .map(|x| {
                let mut m = Map::new();
                m.insert("path".into(), x["path"].clone());
                m.insert("repository".into(), json!({"nameWithOwner": x["repository"]["full_name"], "isPrivate": x["repository"]["private"]}));
                m.insert("sha".into(), x["sha"].clone());
                m.insert("textMatches".into(), x["text_matches"].clone());
                m.insert("url".into(), x["html_url"].clone());
                m
            })
            .collect(),
        _ => items
            .iter()
            .map(|x| {
                let mut m = Map::new();
                m.insert("author".into(), x["author"].clone());
                m.insert("commit".into(), x["commit"].clone());
                m.insert("committer".into(), x["committer"].clone());
                m.insert("id".into(), x["node_id"].clone());
                m.insert("parents".into(), x["parents"].clone());
                m.insert("repository".into(), json!({"fullName": x["repository"]["full_name"], "name": x["repository"]["name"]}));
                m.insert("sha".into(), x["sha"].clone());
                m.insert("url".into(), x["html_url"].clone());
                m
            })
            .collect(),
    };
    if json_out(out, &a, &rows, fields, false)? {
        return Ok(());
    }
    if items.is_empty() {
        let what = match kind {
            "prs" => "pull requests",
            "repos" => "repositories",
            "code" => "code results",
            k => k,
        };
        return Err(Fail::Api(format!("no {what} matched your search")));
    }
    for x in items {
        match kind {
            "issues" | "prs" => {
                let labels = arr(&x["labels"])
                    .iter()
                    .filter_map(|l| l["name"].as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                row(
                    out,
                    &[
                        s(&x["repository"], "full_name"),
                        &n(x, "number").to_string(),
                        s(x, "state"),
                        s(x, "title"),
                        &labels,
                        s(x, "updated_at"),
                    ],
                )?;
            }
            "repos" => {
                let mut info = vec![s(x, "visibility").to_string()];
                if x["fork"].as_bool() == Some(true) {
                    info.push("fork".into());
                }
                if x["archived"].as_bool() == Some(true) {
                    info.push("archived".into());
                }
                row(
                    out,
                    &[
                        s(x, "full_name"),
                        s(x, "description"),
                        &info.join(", "),
                        s(x, "updated_at"),
                    ],
                )?;
            }
            "code" => {
                let repo = s(&x["repository"], "full_name");
                let frags = arr(&x["text_matches"]);
                if frags.is_empty() {
                    writeln!(out, "{repo}:{}", s(x, "path"))?;
                }
                for f in frags {
                    writeln!(out, "{repo}:{}:{}", s(x, "path"), s(f, "fragment").trim())?;
                }
            }
            _ => {
                let msg = s(&x["commit"], "message").lines().next().unwrap_or("");
                row(
                    out,
                    &[
                        s(&x["repository"], "full_name"),
                        s(x, "sha"),
                        msg,
                        s(&x["commit"]["author"], "name"),
                        s(&x["commit"]["author"], "date"),
                    ],
                )?;
            }
        }
    }
    Ok(())
}
