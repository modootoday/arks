//! `arks repo …` — gh-symmetric repository client against the arks forge.
//!
//! `clone`, `create --clone|--push|--source`, `fork --clone|--remote` and local
//! `sync` print the `arks` commands to run (the same convention as
//! `pr checkout`); arks never spawns git or gh. Remote `sync` fast-forwards a fork
//! on the forge. `read-dir` / `read-file` read the forge contents API;
//! `gitignore` / `license` read the forge template catalogs.

use super::common::{Local, enc, parse_slug, seg};
use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, row, s, usage, web};
use base64::Engine as _;
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const HELP: &str = "\
arks repo — repositories on the arks forge (gh-symmetric argv; host-tested Partial)

USAGE:
  arks repo <command> [flags]

COMMANDS:
  create, list, view, edit, delete, fork, rename, archive, unarchive, set-default,
  clone, sync, read-dir, read-file, gitignore {list|view}, license {list|view},
  autolink …, deploy-key …

`clone` / `fork --clone` / `create --clone` print the `arks clone` commands to run
(arks never spawns git or gh). `--web` prints the URL instead of opening a browser.";

pub(crate) const JSON_FIELDS: &[&str] = &[
    "archivedAt",
    "assignableUsers",
    "codeOfConduct",
    "contactLinks",
    "createdAt",
    "defaultBranchRef",
    "deleteBranchOnMerge",
    "description",
    "diskUsage",
    "forkCount",
    "fundingLinks",
    "hasDiscussionsEnabled",
    "hasIssuesEnabled",
    "hasProjectsEnabled",
    "hasWikiEnabled",
    "homepageUrl",
    "id",
    "isArchived",
    "isBlankIssuesEnabled",
    "isEmpty",
    "isFork",
    "isInOrganization",
    "isMirror",
    "isPrivate",
    "isSecurityPolicyEnabled",
    "isTemplate",
    "isUserConfigurationRepository",
    "issueTemplates",
    "issues",
    "labels",
    "languages",
    "latestRelease",
    "licenseInfo",
    "mentionableUsers",
    "mergeCommitAllowed",
    "milestones",
    "mirrorUrl",
    "name",
    "nameWithOwner",
    "openGraphImageUrl",
    "owner",
    "parent",
    "primaryLanguage",
    "projects",
    "projectsV2",
    "pullRequestTemplates",
    "pullRequests",
    "pushedAt",
    "rebaseMergeAllowed",
    "repositoryTopics",
    "securityPolicyUrl",
    "squashMergeAllowed",
    "sshUrl",
    "stargazerCount",
    "templateRepository",
    "updatedAt",
    "url",
    "usesCustomOpenGraphImage",
    "viewerCanAdminister",
    "viewerDefaultCommitEmail",
    "viewerDefaultMergeMethod",
    "viewerHasStarred",
    "viewerPermission",
    "viewerPossibleCommitEmails",
    "viewerSubscription",
    "visibility",
    "watchers",
];

const DIR_FIELDS: &[&str] = &[
    "gitSHA",
    "gitType",
    "mode",
    "modeOctal",
    "name",
    "nameRaw",
    "path",
    "pathRaw",
    "size",
    "submodule",
    "type",
];
const FILE_FIELDS: &[&str] = &[
    "content",
    "downloadUrl",
    "encoding",
    "gitSHA",
    "gitUrl",
    "htmlUrl",
    "name",
    "path",
    "size",
    "type",
    "url",
];

const R: &[&str] = &["-R", "--repo"];

pub(crate) fn gh_row(ctx: &Ctx, r: &Value) -> Map<String, Value> {
    let full = s(r, "full_name");
    let mut m = Map::new();
    let mut put = |k: &str, v: Value| {
        m.insert(k.to_string(), v);
    };
    let zero_list = json!({"totalCount": 0, "nodes": []});
    put(
        "archivedAt",
        if r["archived"].as_bool() == Some(true) {
            r["updated_at"].clone()
        } else {
            Value::Null
        },
    );
    put("assignableUsers", json!([{"login": r["owner"]["login"]}]));
    put("codeOfConduct", Value::Null);
    put("contactLinks", json!([]));
    put("createdAt", r["created_at"].clone());
    put("defaultBranchRef", json!({"name": r["default_branch"]}));
    put("deleteBranchOnMerge", r["delete_branch_on_merge"].clone());
    put("description", r["description"].clone());
    put("diskUsage", r["size"].clone());
    put("forkCount", r["forks_count"].clone());
    put("fundingLinks", json!([]));
    put("hasDiscussionsEnabled", r["has_discussions"].clone());
    put("hasIssuesEnabled", r["has_issues"].clone());
    put("hasProjectsEnabled", r["has_projects"].clone());
    put("hasWikiEnabled", r["has_wiki"].clone());
    put("homepageUrl", r["homepage"].clone());
    put("id", r["node_id"].clone());
    put("isArchived", r["archived"].clone());
    put("isBlankIssuesEnabled", json!(true));
    put("isEmpty", r["empty"].clone());
    put("isFork", r["fork"].clone());
    put(
        "isInOrganization",
        json!(s(&r["owner"], "type") == "Organization"),
    );
    put("isMirror", json!(false));
    put("isPrivate", r["private"].clone());
    put("isSecurityPolicyEnabled", json!(false));
    put("isTemplate", r["is_template"].clone());
    put("isUserConfigurationRepository", json!(false));
    put("issueTemplates", json!([]));
    put("issues", json!({"totalCount": r["open_issues_count"]}));
    put("labels", zero_list.clone());
    put("languages", json!([]));
    put("latestRelease", Value::Null);
    put(
        "licenseInfo",
        match &r["license"] {
            Value::Null => Value::Null,
            l => json!({"key": l["key"], "name": l["name"], "nickname": ""}),
        },
    );
    put("mentionableUsers", json!([{"login": r["owner"]["login"]}]));
    put("mergeCommitAllowed", r["allow_merge_commit"].clone());
    put("milestones", json!([]));
    put("mirrorUrl", Value::Null);
    put("name", r["name"].clone());
    put("nameWithOwner", json!(full));
    put("openGraphImageUrl", json!(""));
    put("owner", json!({"id": "", "login": r["owner"]["login"]}));
    put(
        "parent",
        match &r["parent"] {
            Value::Null => Value::Null,
            p => {
                json!({"id": "", "name": p["name"], "owner": {"id": "", "login": p["owner"]["login"]}})
            }
        },
    );
    put("primaryLanguage", Value::Null);
    put("projects", json!([]));
    put("projectsV2", zero_list.clone());
    put("pullRequestTemplates", json!([]));
    put("pullRequests", json!({"totalCount": 0}));
    put("pushedAt", r["pushed_at"].clone());
    put("rebaseMergeAllowed", r["allow_rebase_merge"].clone());
    put(
        "repositoryTopics",
        Value::Array(
            arr(&r["topics"])
                .iter()
                .map(|t| json!({"name": t}))
                .collect(),
        ),
    );
    put("securityPolicyUrl", json!(""));
    put("squashMergeAllowed", r["allow_squash_merge"].clone());
    put("sshUrl", r["ssh_url"].clone());
    put("stargazerCount", r["stargazers_count"].clone());
    put("templateRepository", Value::Null);
    put("updatedAt", r["updated_at"].clone());
    put("url", json!(ctx.web_url(&format!("/{full}"))));
    put("usesCustomOpenGraphImage", json!(false));
    put("viewerCanAdminister", json!(true));
    put("viewerDefaultCommitEmail", json!(""));
    put("viewerDefaultMergeMethod", json!("MERGE"));
    put("viewerHasStarred", json!(false));
    put("viewerPermission", json!("ADMIN"));
    put("viewerPossibleCommitEmails", json!([]));
    put("viewerSubscription", json!("SUBSCRIBED"));
    put("visibility", json!(s(r, "visibility").to_ascii_uppercase()));
    put("watchers", json!({"totalCount": 0}));
    m
}

/// Repository from a positional `[HOST/]OWNER/REPO` (a bare name means the
/// authenticated account), else `--repo` / the default.
fn target(ctx: &Ctx, a: &Args<'_>, arg: Option<&str>) -> Result<(String, String), Fail> {
    match arg {
        Some(x) if x.contains('/') => parse_slug(x).ok_or_else(|| {
            Fail::Usage(format!(
                "expected the \"[HOST/]OWNER/REPO\" format, got {x:?}"
            ))
        }),
        Some(x) => Ok((ctx.me()?, x.to_string())),
        None => ctx.repo(a),
    }
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "create" | "new" => create(ctx, rest, out),
        "list" | "ls" => list(ctx, rest, out),
        "view" => view(ctx, rest, out),
        "edit" => edit(ctx, rest, out),
        "delete" => delete(ctx, rest, out),
        "fork" => fork(ctx, rest, out),
        "rename" => rename(ctx, rest, out),
        "archive" | "unarchive" => archive(ctx, rest, out, sub == "archive"),
        "set-default" => set_default(ctx, rest, out),
        "clone" => clone(ctx, rest, out),
        "sync" => sync(ctx, rest, out),
        "read-dir" => read_dir(ctx, rest, out),
        "read-file" => read_file(ctx, rest, out),
        "gitignore" => gitignore(ctx, rest, out),
        "license" => license(ctx, rest, out),
        other => usage(format!(
            "unknown command {other:?} for \"arks repo\"\n{HELP}"
        )),
    }
}

fn create(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[
            "-d",
            "--description",
            "-g",
            "--gitignore",
            "-h",
            "--homepage",
            "-l",
            "--license",
            "-r",
            "--remote",
            "-s",
            "--source",
            "-t",
            "--team",
            "-p",
            "--template",
        ],
        &[
            "--add-readme",
            "-c",
            "--clone",
            "--disable-issues",
            "--disable-wiki",
            "--include-all-branches",
            "--internal",
            "--private",
            "--public",
            "--push",
        ],
    )?;
    let vis: Vec<&str> = [
        ("public", "--public"),
        ("private", "--private"),
        ("internal", "--internal"),
    ]
    .iter()
    .filter(|(_, f)| a.has(&[f]))
    .map(|(v, _)| *v)
    .collect();
    if vis.len() > 1 {
        return usage("expected exactly one of `--public`, `--private`, or `--internal`");
    }
    let source = a.get(&["-s", "--source"]);
    let arg = a.positionals().first().copied();
    let name_arg = match (arg, source) {
        (Some(n), _) => n.to_string(),
        (None, Some(src)) => std::fs::canonicalize(src)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .ok_or_else(|| Fail::Usage(format!("cannot read source directory {src:?}")))?,
        (None, None) => return usage("name argument required when not running interactively"),
    };
    if vis.is_empty() {
        return usage(
            "`--public`, `--private`, or `--internal` required when not running interactively",
        );
    }
    if (a.has(&["--push"]) || a.get(&["-r", "--remote"]).is_some()) && source.is_none() {
        return usage("`--push` and `--remote` are only supported with `--source`");
    }
    if a.get(&["-p", "--template"]).is_some()
        && (a.has(&["--add-readme"])
            || a.get(&["-g", "--gitignore"]).is_some()
            || a.get(&["-l", "--license"]).is_some())
    {
        return usage(
            "the `--add-readme`, `--gitignore` and `--license` flags are not supported with `--template`",
        );
    }
    let (owner, name) = match name_arg.split_once('/') {
        Some((o, n)) => (Some(o.to_string()), n.to_string()),
        None => (None, name_arg.clone()),
    };
    let mut body = json!({
        "name": name,
        "description": a.get(&["-d", "--description"]).unwrap_or(""),
        "homepage": a.get(&["-h", "--homepage"]).unwrap_or(""),
        "visibility": vis[0],
        "has_issues": !a.has(&["--disable-issues"]),
        "has_wiki": !a.has(&["--disable-wiki"]),
        "auto_init": a.has(&["--add-readme"]),
    });
    if let Some(g) = a.get(&["-g", "--gitignore"]) {
        body["gitignore_template"] = json!(g);
    }
    if let Some(l) = a.get(&["-l", "--license"]) {
        body["license_template"] = json!(l);
    }
    if let Some(t) = a.get(&["-t", "--team"]) {
        eprintln!("note: the arks forge has no teams; --team {t:?} grants nothing");
    }
    let me = ctx.me()?;
    let r = if let Some(tpl) = a.get(&["-p", "--template"]) {
        let (to, tn) = target(ctx, &a, Some(tpl))?;
        body["owner"] = json!(owner.clone().unwrap_or(me.clone()));
        body["include_all_branches"] = json!(a.has(&["--include-all-branches"]));
        ctx.api("POST", &format!("/repos/{to}/{tn}/generate"), Some(&body))?
    } else {
        let path = match &owner {
            Some(o) if *o != me => format!("/orgs/{o}/repos"),
            _ => "/user/repos".into(),
        };
        ctx.api("POST", &path, Some(&body))?
    };
    let full = s(&r, "full_name").to_string();
    writeln!(out, "{}", ctx.web_url(&format!("/{full}")))?;
    let url = r["clone_url"].as_str().unwrap_or("").to_string();
    if let Some(src) = source {
        let remote = a.get(&["-r", "--remote"]).unwrap_or("origin");
        writeln!(out, "recipe=arks -C {src} remote add {remote} {url}")?;
        if a.has(&["--push"]) {
            writeln!(out, "recipe=arks -C {src} push -u {remote} HEAD")?;
        }
    } else if a.has(&["-c", "--clone"]) {
        writeln!(out, "recipe=arks clone {url} {}", s(&r, "name"))?;
    }
    Ok(())
}

fn list(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[
            &[
                "-l",
                "--language",
                "-L",
                "--limit",
                "--topic",
                "--visibility",
            ][..],
            EXPORT,
        ]
        .concat(),
        &["--archived", "--fork", "--no-archived", "--source"],
    )?;
    if a.has(&["--fork"]) && a.has(&["--source"]) {
        return usage("specify only one of `--source` or `--fork`");
    }
    if a.has(&["--archived"]) && a.has(&["--no-archived"]) {
        return usage("specify only one of `--archived` or `--no-archived`");
    }
    let vis = a.get(&["--visibility"]);
    if let Some(v) = vis {
        if !matches!(v, "public" | "private" | "internal") {
            return usage(format!(
                "invalid argument {v:?} for \"--visibility\" flag: valid values are {{public|private|internal}}"
            ));
        }
    }
    let owner = match a.positionals().first() {
        Some(o) => o.to_string(),
        None => ctx.me()?,
    };
    let limit = a.limit(30)?;
    let all = ctx.get(&format!("/users/{}/repos", seg(&owner)))?;
    let topics = a.list(&["--topic"]);
    let lang = a.get(&["-l", "--language"]);
    let mut items: Vec<Value> = arr(&all)
        .iter()
        .filter(|r| vis.is_none_or(|v| s(r, "visibility") == v))
        .filter(|r| !a.has(&["--fork"]) || r["fork"].as_bool() == Some(true))
        .filter(|r| !a.has(&["--source"]) || r["fork"].as_bool() != Some(true))
        .filter(|r| !a.has(&["--archived"]) || r["archived"].as_bool() == Some(true))
        .filter(|r| !a.has(&["--no-archived"]) || r["archived"].as_bool() != Some(true))
        .filter(|r| {
            topics
                .iter()
                .all(|t| arr(&r["topics"]).iter().any(|x| x.as_str() == Some(t)))
        })
        .filter(|r| {
            lang.is_none_or(|l| {
                r["language"]
                    .as_str()
                    .is_some_and(|x| x.eq_ignore_ascii_case(l))
            })
        })
        .cloned()
        .collect();
    items.truncate(limit);
    let rows: Vec<Map<String, Value>> = items.iter().map(|r| gh_row(ctx, r)).collect();
    if json_out(out, &a, &rows, JSON_FIELDS, false)? {
        return Ok(());
    }
    for r in &items {
        let mut info = vec![s(r, "visibility").to_string()];
        if r["fork"].as_bool() == Some(true) {
            info.push("fork".into());
        }
        if r["archived"].as_bool() == Some(true) {
            info.push("archived".into());
        }
        row(
            out,
            &[
                s(r, "full_name"),
                s(r, "description"),
                &info.join(", "),
                s(r, "updated_at"),
            ],
        )?;
    }
    Ok(())
}

fn view(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[&["-b", "--branch", "-R", "--repo"][..], EXPORT].concat(),
        &["-w", "--web"],
    )?;
    let (o, n) = target(ctx, &a, a.positionals().first().copied())?;
    let r = ctx.get(&format!("/repos/{o}/{n}"))?;
    if a.has(&["-w", "--web"]) {
        let mut url = ctx.web_url(&format!("/{o}/{n}"));
        if let Some(b) = a.get(&["-b", "--branch"]) {
            url.push_str(&format!("/tree/{b}"));
        }
        return web(out, &url);
    }
    if json_out(
        out,
        &a,
        std::slice::from_ref(&gh_row(ctx, &r)),
        JSON_FIELDS,
        true,
    )? {
        return Ok(());
    }
    let mut readme_path = format!("/repos/{o}/{n}/readme");
    if let Some(b) = a.get(&["-b", "--branch"]) {
        readme_path.push_str(&format!("?ref={}", enc(b)));
    }
    let readme = ctx
        .get(&readme_path)
        .ok()
        .and_then(|f| {
            base64::engine::general_purpose::STANDARD
                .decode(s(&f, "content"))
                .ok()
        })
        .map(|b| String::from_utf8_lossy(&b).into_owned());
    writeln!(out, "name:\t{o}/{n}")?;
    writeln!(out, "description:\t{}", s(&r, "description"))?;
    writeln!(out, "--")?;
    match readme {
        Some(t) => write!(out, "{t}")?,
        None => writeln!(out, "This repository does not have a README")?,
    }
    Ok(())
}

fn edit(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let bools = [
        "--accept-visibility-change-consequences",
        "--allow-forking",
        "--allow-update-branch",
        "--delete-branch-on-merge",
        "--enable-advanced-security",
        "--enable-auto-merge",
        "--enable-discussions",
        "--enable-issues",
        "--enable-merge-commit",
        "--enable-projects",
        "--enable-rebase-merge",
        "--enable-secret-scanning",
        "--enable-secret-scanning-push-protection",
        "--enable-squash-merge",
        "--enable-wiki",
        "--template",
    ];
    let a = Args::parse(
        raw,
        &[
            "--add-topic",
            "--default-branch",
            "-d",
            "--description",
            "-h",
            "--homepage",
            "--remove-topic",
            "--squash-merge-commit-message",
            "--visibility",
        ],
        &bools,
    )?;
    let (o, n) = target(ctx, &a, a.positionals().first().copied())?;
    let mut body = Map::new();
    if let Some(d) = a.get(&["-d", "--description"]) {
        body.insert("description".into(), json!(d));
    }
    if let Some(h) = a.get(&["-h", "--homepage"]) {
        body.insert("homepage".into(), json!(h));
    }
    if let Some(b) = a.get(&["--default-branch"]) {
        body.insert("default_branch".into(), json!(b));
    }
    if let Some(v) = a.get(&["--visibility"]) {
        if !matches!(v, "public" | "private" | "internal") {
            return usage(format!(
                "invalid argument {v:?} for \"--visibility\" flag: valid values are {{public,private,internal}}"
            ));
        }
        if !a.has(&["--accept-visibility-change-consequences"]) {
            return usage(
                "use of --visibility flag requires --accept-visibility-change-consequences flag",
            );
        }
        body.insert("visibility".into(), json!(v));
    }
    if let Some(m) = a.get(&["--squash-merge-commit-message"]) {
        let ok = [
            "default",
            "pr-title",
            "pr-title-commits",
            "pr-title-description",
        ];
        if !ok.contains(&m) {
            return usage(format!(
                "invalid argument {m:?} for \"--squash-merge-commit-message\" flag: valid values are {{{}}}",
                ok.join("|")
            ));
        }
        body.insert("squash_merge_commit_message".into(), json!(m));
    }
    for (flag, key) in [
        ("--allow-forking", "allow_forking"),
        ("--allow-update-branch", "allow_update_branch"),
        ("--delete-branch-on-merge", "delete_branch_on_merge"),
        ("--enable-auto-merge", "allow_auto_merge"),
        ("--enable-discussions", "has_discussions"),
        ("--enable-issues", "has_issues"),
        ("--enable-merge-commit", "allow_merge_commit"),
        ("--enable-projects", "has_projects"),
        ("--enable-rebase-merge", "allow_rebase_merge"),
        ("--enable-squash-merge", "allow_squash_merge"),
        ("--enable-wiki", "has_wiki"),
        ("--template", "is_template"),
    ] {
        if let Some(b) = a.bool_opt(&[flag]) {
            body.insert(key.into(), json!(b));
        }
    }
    for flag in [
        "--enable-advanced-security",
        "--enable-secret-scanning",
        "--enable-secret-scanning-push-protection",
    ] {
        if a.bool_opt(&[flag]).is_some() {
            eprintln!(
                "note: {flag} is a GitHub security product; the arks forge has no equivalent setting (nothing changed)"
            );
        }
    }
    let (add, rm) = (a.list(&["--add-topic"]), a.list(&["--remove-topic"]));
    if !add.is_empty() || !rm.is_empty() {
        let cur = ctx.get(&format!("/repos/{o}/{n}/topics"))?;
        let mut names: Vec<String> = arr(&cur["names"])
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .filter(|t| !rm.contains(t))
            .collect();
        for t in add {
            if !names.contains(&t) {
                names.push(t);
            }
        }
        ctx.api(
            "PUT",
            &format!("/repos/{o}/{n}/topics"),
            Some(&json!({"names": names})),
        )?;
    } else if body.is_empty() {
        return usage("specify at least one flag to edit (see --help)");
    }
    if !body.is_empty() {
        ctx.api(
            "PATCH",
            &format!("/repos/{o}/{n}"),
            Some(&Value::Object(body)),
        )?;
    }
    writeln!(out, "{}", ctx.web_url(&format!("/{o}/{n}")))?;
    Ok(())
}

fn delete(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, R, &["--yes", "-y"])?;
    let (o, n) = target(ctx, &a, a.positionals().first().copied())?;
    ctx.get(&format!("/repos/{o}/{n}"))?;
    ctx.api("DELETE", &format!("/repos/{o}/{n}"), None)?;
    eprintln!("✓ Deleted repository {o}/{n}");
    writeln!(out, "deleted={o}/{n}")?;
    Ok(())
}

fn fork(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &["--fork-name", "--org", "--remote-name"],
        &["--clone", "--default-branch-only", "--remote"],
    )?;
    let (o, n) = target(ctx, &a, a.positionals().first().copied())?;
    let mut body = json!({"default_branch_only": a.has(&["--default-branch-only"])});
    if let Some(org) = a.get(&["--org"]) {
        body["organization"] = json!(org);
    }
    if let Some(nm) = a.get(&["--fork-name"]) {
        body["name"] = json!(nm);
    }
    let f = ctx.api("POST", &format!("/repos/{o}/{n}/forks"), Some(&body))?;
    let full = s(&f, "full_name");
    eprintln!("✓ Created fork {full}");
    writeln!(out, "{}", ctx.web_url(&format!("/{full}")))?;
    let url = s(&f, "clone_url");
    let remote_name = a.get(&["--remote-name"]).unwrap_or("origin");
    if a.has(&["--clone"]) {
        let src = ctx.get(&format!("/repos/{o}/{n}"))?;
        writeln!(
            out,
            "recipe=arks clone {url} {} && arks -C {} remote add upstream {}",
            s(&f, "name"),
            s(&f, "name"),
            s(&src, "clone_url")
        )?;
    } else if a.has(&["--remote"]) || a.get(&["--remote-name"]).is_some() {
        writeln!(
            out,
            "recipe=arks remote rename origin upstream && arks remote add {remote_name} {url}"
        )?;
    }
    if !a.passthrough().is_empty() {
        eprintln!(
            "note: arguments after `--` are for gh's git clone; add them to the arks clone command yourself"
        );
    }
    Ok(())
}

fn rename(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    // `-n/--name` is the legacy arks spelling; gh takes the new name positionally.
    let a = Args::parse(raw, &["-R", "--repo", "-n", "--name"], &["-y", "--yes"])?;
    let (o, n) = ctx.repo(&a)?;
    let new = a
        .positionals()
        .first()
        .copied()
        .or(a.get(&["-n", "--name"]))
        .ok_or_else(|| {
            Fail::Usage("new name argument required when not running interactively".into())
        })?;
    let new = new.rsplit('/').next().unwrap_or(new);
    let r = ctx.api(
        "PATCH",
        &format!("/repos/{o}/{n}"),
        Some(&json!({"name": new})),
    )?;
    eprintln!("✓ Renamed repository {}", s(&r, "full_name"));
    writeln!(
        out,
        "full_name={}\nname={}",
        s(&r, "full_name"),
        s(&r, "name")
    )?;
    Ok(())
}

fn archive(ctx: &Ctx, raw: &[String], out: &mut dyn Write, on: bool) -> Res {
    let a = Args::parse(raw, R, &["-y", "--yes"])?;
    let (o, n) = target(ctx, &a, a.positionals().first().copied())?;
    let cur = ctx.get(&format!("/repos/{o}/{n}"))?;
    if cur["archived"].as_bool() == Some(on) {
        eprintln!(
            "! Repository {o}/{n} is already {}archived",
            if on { "" } else { "un" }
        );
        writeln!(out, "full_name={o}/{n}\narchived={on}")?;
        return Ok(());
    }
    let r = ctx.api(
        "PATCH",
        &format!("/repos/{o}/{n}"),
        Some(&json!({"archived": on})),
    )?;
    eprintln!(
        "✓ {} repository {o}/{n}",
        if on { "Archived" } else { "Unarchived" }
    );
    writeln!(
        out,
        "full_name={}\narchived={}",
        s(&r, "full_name"),
        r["archived"]
    )?;
    Ok(())
}

fn set_default(_ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, R, &["-u", "--unset", "-v", "--view"])?;
    let mut st = crate::load_hub_client_state();
    if a.has(&["-v", "--view"]) {
        match &st.default_repo {
            Some(r) => writeln!(out, "{r}")?,
            None => eprintln!(
                "no default repository has been set; use `arks repo set-default` to select one"
            ),
        }
        return Ok(());
    }
    if a.has(&["-u", "--unset"]) {
        st.default_repo = None;
        crate::save_hub_client_state(&st).map_err(Fail::Api)?;
        eprintln!("✓ Unset the default repository");
        return Ok(());
    }
    let arg = a
        .positionals()
        .first()
        .map(|s| s.to_string())
        .or_else(|| a.get(R).map(str::to_string))
        .or_else(|| std::env::var("ARKS_HUB_REPO").ok());
    let Some(arg) = arg else {
        return usage("repository argument required when not running interactively");
    };
    let (o, n) = parse_slug(&arg).ok_or_else(|| {
        Fail::Usage(format!(
            "expected the \"[HOST/]OWNER/REPO\" format, got {arg:?}"
        ))
    })?;
    st.default_repo = Some(format!("{o}/{n}"));
    crate::save_hub_client_state(&st).map_err(Fail::Api)?;
    writeln!(out, "default_repo={o}/{n}")?;
    Ok(())
}

fn clone(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &["-u", "--upstream-remote-name"], &["--no-upstream"])?;
    let pos = a.positionals();
    let arg = pos
        .first()
        .copied()
        .ok_or_else(|| Fail::Usage("cannot clone: repository argument required".into()))?;
    let (o, n) = target(ctx, &a, Some(arg))?;
    let r = ctx.get(&format!("/repos/{o}/{n}"))?;
    let url = r["clone_url"].as_str().ok_or_else(|| {
        Fail::Api(format!(
            "{o}/{n} has no clone URL on this forge (no Git content)"
        ))
    })?;
    let dir = pos.get(1).copied().unwrap_or(&n);
    let mut extra = String::new();
    for p in a.passthrough() {
        extra.push(' ');
        extra.push_str(p);
    }
    let mut recipe = format!("arks clone{extra} {url} {dir}");
    if !a.has(&["--no-upstream"]) && !r["parent"].is_null() {
        let p = &r["parent"];
        let pr = ctx.get(&format!("/repos/{}", s(p, "full_name")))?;
        let up = a
            .get(&["-u", "--upstream-remote-name"])
            .unwrap_or("upstream");
        recipe.push_str(&format!(
            " && arks -C {dir} remote add {up} {}",
            s(&pr, "clone_url")
        ));
    }
    writeln!(out, "clone_url={url}")?;
    writeln!(out, "recipe={recipe}")?;
    eprintln!("note: repo clone prints the arks commands to run; it never spawns git or gh");
    Ok(())
}

fn sync(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &["-b", "--branch", "-s", "--source"], &["--force"])?;
    let dest = a.positionals().first().copied();
    let branch = a.get(&["-b", "--branch"]);
    let Some(dest) = dest else {
        // Local sync: fast-forward the current checkout from its remote.
        let local = Local::discover().ok_or_else(|| {
            Fail::Usage("not in a repository; pass a destination repository".into())
        })?;
        let b = branch
            .map(str::to_string)
            .or_else(|| local.branch())
            .unwrap_or_else(|| "main".into());
        let remote = a.get(&["-s", "--source"]).unwrap_or("origin");
        let reset = if a.has(&["--force"]) {
            format!("arks reset --hard {remote}/{b}")
        } else {
            format!("arks merge --ff-only {remote}/{b}")
        };
        writeln!(
            out,
            "recipe=arks fetch {remote} {b} && arks checkout {b} && {reset}"
        )?;
        eprintln!("note: local sync prints the arks commands to run; arks never spawns git");
        return Ok(());
    };
    let (o, n) = target(ctx, &a, Some(dest))?;
    let mut body = json!({"force": a.has(&["--force"])});
    if let Some(b) = branch {
        body["branch"] = json!(b);
    }
    if let Some(src) = a.get(&["-s", "--source"]) {
        let (so, sn) =
            parse_slug(src).ok_or_else(|| Fail::Usage(format!("invalid --source {src:?}")))?;
        body["source"] = json!(format!("{so}/{sn}"));
    }
    let v = ctx.api(
        "POST",
        &format!("/repos/{o}/{n}/merge-upstream"),
        Some(&body),
    )?;
    let from = s(&v, "base_branch");
    eprintln!(
        "✓ Synced the \"{o}/{n}:{}\" branch from \"{from}\"",
        from.rsplit(':').next().unwrap_or("")
    );
    writeln!(
        out,
        "merge_type={}\nsha={}",
        s(&v, "merge_type"),
        s(&v, "sha")
    )?;
    Ok(())
}

fn read_dir(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &[&["--ref", "-R", "--repo"][..], EXPORT].concat(), &[])?;
    let (o, n) = ctx.repo(&a)?;
    let path = a
        .positionals()
        .first()
        .copied()
        .unwrap_or("")
        .trim_matches('/');
    let mut url = format!("/repos/{o}/{n}/contents/{path}");
    if let Some(r) = a.get(&["--ref"]) {
        url.push_str(&format!("?ref={}", enc(r)));
    }
    let v = ctx.get(&url)?;
    let Some(items) = v.as_array() else {
        return Err(Fail::Api(format!("{path:?} is not a directory")));
    };
    let rows: Vec<Map<String, Value>> = items
        .iter()
        .map(|e| {
            let mode = s(e, "mode");
            let gtype = match s(e, "type") {
                "dir" => "tree",
                "submodule" => "commit",
                _ => "blob",
            };
            let mut m = Map::new();
            m.insert("gitSHA".into(), e["sha"].clone());
            m.insert("gitType".into(), json!(gtype));
            m.insert("mode".into(), json!(mode));
            m.insert("modeOctal".into(), json!(format!("{:0>6}", mode)));
            m.insert("name".into(), e["name"].clone());
            m.insert("nameRaw".into(), e["name"].clone());
            m.insert("path".into(), e["path"].clone());
            m.insert("pathRaw".into(), e["path"].clone());
            m.insert("size".into(), e["size"].clone());
            m.insert(
                "submodule".into(),
                if s(e, "type") == "submodule" {
                    json!({"gitUrl": ""})
                } else {
                    Value::Null
                },
            );
            m.insert("type".into(), e["type"].clone());
            m
        })
        .collect();
    if json_out(out, &a, &rows, DIR_FIELDS, false)? {
        return Ok(());
    }
    for m in &rows {
        let name = m["name"].as_str().unwrap_or("");
        let suffix = if m["type"] == "dir" { "/" } else { "" };
        writeln!(out, "{name}{suffix}")?;
    }
    Ok(())
}

fn read_file(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[&["--ref", "-R", "--repo", "-o", "--output"][..], EXPORT].concat(),
        &["--allow-escape-sequences", "--clobber"],
    )?;
    let (o, n) = ctx.repo(&a)?;
    let path = a
        .positionals()
        .first()
        .copied()
        .ok_or_else(|| Fail::Usage("path argument required".into()))?
        .trim_matches('/');
    let mut url = format!("/repos/{o}/{n}/contents/{path}");
    if let Some(r) = a.get(&["--ref"]) {
        url.push_str(&format!("?ref={}", enc(r)));
    }
    let v = ctx.get(&url)?;
    if v.is_array() {
        return Err(Fail::Api(format!(
            "{path:?} is a directory; use `arks repo read-dir`"
        )));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(s(&v, "content"))
        .unwrap_or_default();
    let mut m = Map::new();
    m.insert("content".into(), json!(String::from_utf8_lossy(&bytes)));
    m.insert("downloadUrl".into(), v["download_url"].clone());
    m.insert("encoding".into(), json!("utf-8"));
    m.insert("gitSHA".into(), v["sha"].clone());
    m.insert("gitUrl".into(), v["git_url"].clone());
    m.insert("htmlUrl".into(), v["html_url"].clone());
    m.insert("name".into(), v["name"].clone());
    m.insert("path".into(), v["path"].clone());
    m.insert("size".into(), v["size"].clone());
    m.insert("type".into(), v["type"].clone());
    m.insert("url".into(), v["url"].clone());
    if json_out(out, &a, std::slice::from_ref(&m), FILE_FIELDS, true)? {
        return Ok(());
    }
    if let Some(dest) = a.get(&["-o", "--output"]) {
        let p = std::path::Path::new(dest);
        if p.exists() && !a.has(&["--clobber"]) {
            return Err(Fail::Api(format!(
                "{dest} already exists; use --clobber to overwrite it"
            )));
        }
        if let Some(parent) = p.parent().filter(|x| !x.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, &bytes)?;
        return Ok(());
    }
    if !a.has(&["--allow-escape-sequences"]) && bytes.contains(&0x1b) {
        return Err(Fail::Api(format!(
            "{path} contains terminal escape sequences; pass --allow-escape-sequences to print it anyway"
        )));
    }
    out.write_all(&bytes)?;
    Ok(())
}

fn gitignore(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let a = Args::parse(args.get(1..).unwrap_or(&[]), &[], &[])?;
    match sub {
        "list" | "ls" => {
            for name in arr(&ctx.get("/gitignore/templates")?) {
                writeln!(out, "{}", name.as_str().unwrap_or(""))?;
            }
            Ok(())
        }
        "view" => {
            let name = a.positionals().first().copied().ok_or_else(|| {
                Fail::Usage("template argument required when not running interactively".into())
            })?;
            let t = ctx.get(&format!("/gitignore/templates/{}", seg(name)))?;
            write!(out, "{}", s(&t, "source"))?;
            Ok(())
        }
        "" | "help" => Err(Fail::Help),
        other => usage(format!(
            "unknown command {other:?} for \"arks repo gitignore\""
        )),
    }
}

fn license(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "list" | "ls" => {
            Args::parse(args.get(1..).unwrap_or(&[]), &[], &[])?;
            for l in arr(&ctx.get("/licenses")?) {
                row(out, &[s(l, "key"), s(l, "name")])?;
            }
            Ok(())
        }
        "view" => {
            let a = Args::parse(args.get(1..).unwrap_or(&[]), &[], &["-w", "--web"])?;
            let key = a.positionals().first().copied().ok_or_else(|| {
                Fail::Usage("license argument required when not running interactively".into())
            })?;
            let l = ctx.get(&format!("/licenses/{}", seg(key)))?;
            if a.has(&["-w", "--web"]) {
                return web(out, s(&l, "html_url"));
            }
            writeln!(out, "{}", s(&l, "description"))?;
            writeln!(out)?;
            writeln!(out, "To implement: {}", s(&l, "implementation"))?;
            writeln!(out)?;
            writeln!(out, "For more information, see: {}", s(&l, "html_url"))?;
            writeln!(out)?;
            write!(out, "{}", s(&l, "body"))?;
            Ok(())
        }
        "" | "help" => Err(Fail::Help),
        other => usage(format!(
            "unknown command {other:?} for \"arks repo license\""
        )),
    }
}
