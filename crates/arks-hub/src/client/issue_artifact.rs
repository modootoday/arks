//! `arks issue artifact create|delete|download|edit|list|view` (gh preview).
//!
//! Peers: <https://cli.github.com/manual/gh_issue_artifact> and its leaves.
//! REST: <https://docs.github.com/en/rest/issues/artifacts>
//! (`GET|POST /repos/{o}/{r}/issues/{n}/artifacts`,
//! `GET|PATCH|DELETE …/artifacts/{artifact_number}`).
//!
//! Partial: `--attach` (image/video upload) refuses — no documented upload
//! endpoint; `--web` refuses (no browser launch); `--jq`/`--template` refuse.
//! Artifact URLs as arguments are accepted when they share one issue.

use super::common::{
    Ctx, call, ctx, fail, has_flag, issue_ref, json_fields, positionals, print_json, select,
    str_of, usage,
};
use serde_json::{Map, Value, json};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

const VALUE_FLAGS: &[&str] = &[
    "-R",
    "--repo",
    "--json",
    "-q",
    "--jq",
    "-t",
    "--template",
    "-L",
    "--limit",
    "--type",
    "-b",
    "--body",
    "-F",
    "--body-file",
    "--name",
    "--attach",
    "--version",
    "--restore-version",
    "-D",
    "--dir",
    "-O",
    "--output",
];
const LIST_FIELDS: &[&str] = &[
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
const VIEW_FIELDS: &[&str] = &[
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
    "versions",
];

fn gh_obj(a: &Value) -> Map<String, Value> {
    let g = |k: &str| a.get(k).cloned().unwrap_or(Value::Null);
    let mut m = Map::new();
    m.insert("body".into(), g("body"));
    m.insert("bodyHtml".into(), g("body_html"));
    m.insert("createdAt".into(), g("created_at"));
    m.insert("creator".into(), g("creator"));
    m.insert("description".into(), g("description"));
    m.insert("id".into(), g("id"));
    m.insert("name".into(), g("name"));
    m.insert("number".into(), g("number"));
    m.insert("type".into(), g("type"));
    m.insert("updatedAt".into(), g("updated_at"));
    m.insert("updatedByActor".into(), g("updated_by_actor"));
    m.insert("url".into(), g("html_url"));
    m
}

/// Resolve `{<issue> <artifact-number>... | <artifact-url>...}` to (issue, numbers).
fn issue_and_numbers(pos: &[String], c: &mut Ctx) -> Result<(u64, Vec<u64>), String> {
    let first = pos.first().ok_or("issue number or artifact URL required")?;
    if first.contains("/artifacts/") {
        let mut issue = None;
        let mut nums = Vec::new();
        for u in pos {
            let (slug, n) = issue_ref(u).ok_or_else(|| format!("bad artifact URL {u}"))?;
            if let Some((o, r)) = slug {
                c.owner = o;
                c.repo = r;
            }
            if issue.is_some_and(|i| i != n) {
                return Err("all artifact URLs must belong to the same issue".into());
            }
            issue = Some(n);
            let k = u
                .rsplit_once("/artifacts/")
                .and_then(|(_, k)| k.trim_end_matches('/').parse().ok())
                .ok_or_else(|| format!("bad artifact URL {u}"))?;
            nums.push(k);
        }
        return Ok((issue.unwrap_or(0), nums));
    }
    let (slug, issue) = issue_ref(first).ok_or_else(|| format!("bad issue reference {first}"))?;
    if let Some((o, r)) = slug {
        c.owner = o;
        c.repo = r;
    }
    let mut nums = Vec::new();
    for s in &pos[1..] {
        nums.push(
            s.parse()
                .map_err(|_| format!("artifact number expected, got `{s}`"))?,
        );
    }
    Ok((issue, nums))
}

fn base_path(c: &Ctx, issue: u64) -> String {
    format!("/repos/{}/{}/issues/{issue}/artifacts", c.owner, c.repo)
}

fn read_body_file(p: &str) -> Result<String, String> {
    if p == "-" {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| e.to_string())?;
        return Ok(s);
    }
    fs::read_to_string(p).map_err(|e| format!("read {p}: {e}"))
}

/// Strip characters the manual says artifact names cannot hold (parentheses).
fn clean_name(n: &str) -> String {
    n.chars()
        .filter(|c| *c != '(' && *c != ')')
        .collect::<String>()
}

/// Parse an Internet Shortcut (`.url`) file: `[InternetShortcut]` + `URL=`.
fn shortcut_url(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.trim().strip_prefix("URL=").map(|u| u.trim().to_string()))
}

fn is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// One artifact to create: (type, name, body).
fn plan_from_source(
    src: &str,
    type_override: Option<&str>,
) -> Result<(String, String, String), String> {
    if is_http_url(src) {
        let host = src
            .split("://")
            .nth(1)
            .and_then(|r| r.split('/').next())
            .unwrap_or(src);
        return Ok(("link".into(), host.to_string(), src.to_string()));
    }
    let (file, name) = match src.split_once('#') {
        Some((f, n)) => (f, Some(n.to_string())),
        None => (src, None),
    };
    let text = fs::read_to_string(file).map_err(|e| format!("read {file}: {e}"))?;
    let fname = Path::new(file)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.to_string());
    let lower = fname.to_ascii_lowercase();
    if lower.ends_with(".url") {
        let url = shortcut_url(&text).ok_or_else(|| format!("{file}: no URL= line"))?;
        let host = url
            .split("://")
            .nth(1)
            .and_then(|r| r.split('/').next())
            .unwrap_or(&url)
            .to_string();
        return Ok(("link".into(), name.unwrap_or(host), url));
    }
    let kind = if lower.ends_with(".html") || lower.ends_with(".htm") {
        "embedded_html"
    } else {
        "generic"
    };
    let kind = type_override.unwrap_or(kind).to_string();
    Ok((kind, clean_name(&name.unwrap_or(fname)), text))
}

pub fn run(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "" | "-h" | "--help" => {
            eprintln!(
                "arks issue artifact — create|delete|download|edit|list|view \
                 (/repos/{{o}}/{{r}}/issues/{{n}}/artifacts; preview)"
            );
            ExitCode::SUCCESS
        }
        "list" | "ls" => list(rest),
        "create" | "new" => create(rest),
        "view" => view(rest),
        "edit" => edit(rest),
        "delete" => delete(rest),
        "download" => download(rest),
        other => usage(
            "issue artifact",
            &format!("unknown subcommand `{other}` (create|delete|download|edit|list|view)"),
        ),
    }
}

fn setup(args: &[String], label: &str) -> Result<(Ctx, u64, Vec<u64>), ExitCode> {
    let mut c = ctx(args, label)?;
    let pos = positionals(args, VALUE_FLAGS);
    let (issue, nums) = issue_and_numbers(&pos, &mut c).map_err(|e| usage(label, &e))?;
    Ok((c, issue, nums))
}

fn list(args: &[String]) -> ExitCode {
    let label = "issue artifact list";
    let fields = match json_fields(args, label, LIST_FIELDS) {
        Ok(f) => f,
        Err(c) => return c,
    };
    let (c, issue, _) = match setup(args, label) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let mut path = base_path(&c, issue);
    if let Some(t) = crate::flag_value(args, &["--type"]) {
        path.push_str(&format!("?type={t}"));
    }
    let limit: usize = crate::flag_value(args, &["--limit", "-L"])
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    match call(&c.base, &c.tok, "GET", &path, None, label) {
        Ok(Value::Array(mut items)) => {
            if limit > 0 {
                items.truncate(limit);
            }
            if let Some(f) = fields {
                let out: Vec<Value> = items.iter().map(|a| select(&gh_obj(a), &f)).collect();
                print_json(&Value::Array(out));
            } else {
                for a in &items {
                    println!(
                        "{}\t{}\t{}\t{}",
                        a.get("number").cloned().unwrap_or(Value::Null),
                        str_of(a, "type"),
                        str_of(a, "name"),
                        str_of(a, "updated_at")
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Ok(_) => usage(label, "unexpected response shape"),
        Err(code) => code,
    }
}

fn create(args: &[String]) -> ExitCode {
    let label = "issue artifact create";
    if crate::flag_value(args, &["--attach"]).is_some() {
        return usage(
            label,
            "--attach is not implemented (no documented image/video upload endpoint; Partial)",
        );
    }
    let type_override = crate::flag_value(args, &["--type"]);
    if let Some(t) = type_override
        && !["generic", "plan", "embedded_html"].contains(&t)
    {
        return usage(label, "--type must be generic, plan or embedded_html");
    }
    let mut c = match ctx(args, label) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let pos = positionals(args, VALUE_FLAGS);
    let Some(first) = pos.first() else {
        return usage(label, "<issue-number> required");
    };
    let Some((slug, issue)) = issue_ref(first) else {
        return usage(label, &format!("bad issue reference {first}"));
    };
    if let Some((o, r)) = slug {
        c.owner = o;
        c.repo = r;
    }
    let name_flag = crate::flag_value(args, &["--name"]);
    let mut plans = Vec::new();
    if let Some(b) = crate::flag_value(args, &["--body", "-b"]) {
        let Some(n) = name_flag else {
            return usage(label, "--name is required with --body");
        };
        plans.push((
            type_override.unwrap_or("generic").to_string(),
            clean_name(n),
            b.to_string(),
        ));
    } else if let Some(f) = crate::flag_value(args, &["--body-file", "-F"]) {
        let text = match read_body_file(f) {
            Ok(t) => t,
            Err(e) => return usage(label, &e),
        };
        let n = name_flag.map(str::to_string).unwrap_or_else(|| {
            Path::new(f)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "artifact".into())
        });
        plans.push((
            type_override.unwrap_or("generic").to_string(),
            clean_name(&n),
            text,
        ));
    } else {
        for src in &pos[1..] {
            match plan_from_source(src, type_override) {
                Ok(mut p) => {
                    if pos.len() == 2
                        && let Some(n) = name_flag
                    {
                        p.1 = clean_name(n);
                    }
                    plans.push(p);
                }
                Err(e) => return usage(label, &e),
            }
        }
    }
    if plans.is_empty() {
        return usage(
            label,
            "nothing to create: give files/URLs, --body or --body-file",
        );
    }
    let path = base_path(&c, issue);
    for (kind, name, body) in plans {
        let payload = json!({"name": name, "body": body, "type": kind});
        match call(&c.base, &c.tok, "POST", &path, Some(&payload), label) {
            Ok(v) => println!(
                "{}\t{}\t{}",
                v.get("number").cloned().unwrap_or(Value::Null),
                str_of(&v, "type"),
                str_of(&v, "name")
            ),
            Err(code) => return code,
        }
    }
    ExitCode::SUCCESS
}

/// GET one artifact; returns (artifact object, versions newest-first).
fn fetch_one(c: &Ctx, issue: u64, num: u64, label: &str) -> Result<(Value, Vec<Value>), ExitCode> {
    let v = call(
        &c.base,
        &c.tok,
        "GET",
        &format!("{}/{num}", base_path(c, issue)),
        None,
        label,
    )?;
    if let Some(a) = v.get("artifact") {
        let versions = v
            .get("versions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok((a.clone(), versions))
    } else {
        Ok((v, Vec::new()))
    }
}

/// Apply `--version N` (0 = current) to an artifact object.
fn at_version(a: &Value, versions: &[Value], version: u64, label: &str) -> Result<Value, ExitCode> {
    if version == 0 {
        return Ok(a.clone());
    }
    let ver = versions
        .iter()
        .find(|v| v.get("version").and_then(Value::as_u64) == Some(version))
        .ok_or_else(|| fail(label, &format!("version {version} not found")))?;
    let mut out = a.clone();
    out["name"] = ver.get("name").cloned().unwrap_or(Value::Null);
    out["body"] = ver.get("body").cloned().unwrap_or(Value::Null);
    Ok(out)
}

fn version_flag(args: &[String], names: &[&str], label: &str) -> Result<u64, ExitCode> {
    match crate::flag_value(args, names) {
        None => Ok(0),
        Some(s) => s
            .parse()
            .map_err(|_| usage(label, &format!("{} expects a number", names[0]))),
    }
}

fn view(args: &[String]) -> ExitCode {
    let label = "issue artifact view";
    if has_flag(args, &["--web", "-w"]) {
        return usage(label, "--web is not implemented (no browser launch)");
    }
    let fields = match json_fields(args, label, VIEW_FIELDS) {
        Ok(f) => f,
        Err(c) => return c,
    };
    let version = match version_flag(args, &["--version"], label) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (c, issue, nums) = match setup(args, label) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let [num] = nums[..] else {
        return usage(label, "exactly one artifact required");
    };
    let (a, versions) = match fetch_one(&c, issue, num, label) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let shown = match at_version(&a, &versions, version, label) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if let Some(f) = fields {
        let mut m = gh_obj(&shown);
        m.insert("versions".into(), Value::Array(versions));
        print_json(&select(&m, &f));
        return ExitCode::SUCCESS;
    }
    println!("name:\t{}", str_of(&shown, "name"));
    println!("type:\t{}", str_of(&shown, "type"));
    println!(
        "creator:\t{}",
        shown
            .get("creator")
            .and_then(|c| c.get("login"))
            .and_then(Value::as_str)
            .unwrap_or("")
    );
    println!("number:\t{num}");
    println!("versions:\t{}", versions.len());
    println!("--");
    print!("{}", str_of(&shown, "body"));
    if !str_of(&shown, "body").ends_with('\n') {
        println!();
    }
    ExitCode::SUCCESS
}

fn edit(args: &[String]) -> ExitCode {
    let label = "issue artifact edit";
    if crate::flag_value(args, &["--attach"]).is_some() {
        return usage(
            label,
            "--attach is not implemented (no documented image/video upload endpoint; Partial)",
        );
    }
    let restore = match version_flag(args, &["--restore-version"], label) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (c, issue, nums) = match setup(args, label) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let [num] = nums[..] else {
        return usage(label, "exactly one artifact required");
    };
    let (a, versions) = match fetch_one(&c, issue, num, label) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let mut patch = Map::new();
    if restore > 0 {
        let old = match at_version(&a, &versions, restore, label) {
            Ok(v) => v,
            Err(code) => return code,
        };
        patch.insert(
            "name".into(),
            old.get("name").cloned().unwrap_or(Value::Null),
        );
        patch.insert(
            "body".into(),
            old.get("body").cloned().unwrap_or(Value::Null),
        );
    }
    if let Some(n) = crate::flag_value(args, &["--name"]) {
        patch.insert("name".into(), json!(clean_name(n)));
    }
    let new_body = if let Some(b) = crate::flag_value(args, &["--body", "-b"]) {
        Some(b.to_string())
    } else if let Some(f) = crate::flag_value(args, &["--body-file", "-F"]) {
        match read_body_file(f) {
            Ok(t) if str_of(&a, "type") == "link" && f.to_ascii_lowercase().ends_with(".url") => {
                shortcut_url(&t)
            }
            Ok(t) => Some(t),
            Err(e) => return usage(label, &e),
        }
    } else {
        None
    };
    if let Some(b) = new_body {
        if str_of(&a, "type") == "link" && !is_http_url(b.trim()) {
            return usage(
                label,
                "a link's new content must be an http(s) URL or a .url file",
            );
        }
        patch.insert("body".into(), json!(b));
    }
    if patch.is_empty() {
        return usage(
            label,
            "nothing to edit: give --name, --body, --body-file or --restore-version",
        );
    }
    let path = format!("{}/{num}", base_path(&c, issue));
    match call(
        &c.base,
        &c.tok,
        "PATCH",
        &path,
        Some(&Value::Object(patch)),
        label,
    ) {
        Ok(v) => {
            println!(
                "{}\t{}\t{}",
                v.get("number").cloned().unwrap_or(Value::Null),
                str_of(&v, "type"),
                str_of(&v, "name")
            );
            ExitCode::SUCCESS
        }
        Err(code) => code,
    }
}

fn delete(args: &[String]) -> ExitCode {
    let label = "issue artifact delete";
    if !has_flag(args, &["--yes"]) {
        return usage(label, "--yes required when not running interactively");
    }
    let (c, issue, nums) = match setup(args, label) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if nums.is_empty() {
        return usage(label, "at least one artifact number required");
    }
    for num in nums {
        let path = format!("{}/{num}", base_path(&c, issue));
        if let Err(code) = call(&c.base, &c.tok, "DELETE", &path, None, label) {
            return code;
        }
        println!("deleted artifact {num} from issue {issue}");
    }
    ExitCode::SUCCESS
}

/// File content for a downloaded artifact (`.url` shortcut for links).
fn file_payload(a: &Value) -> (String, String) {
    let body = str_of(a, "body").to_string();
    match str_of(a, "type") {
        "link" => (
            "url".into(),
            format!("[InternetShortcut]\r\nURL={}\r\n", body.trim()),
        ),
        "embedded_html" => ("html".into(), body),
        _ => ("md".into(), body),
    }
}

fn file_name_for(a: &Value) -> String {
    let (ext, _) = file_payload(a);
    let name: String = str_of(a, "name")
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':') {
                '-'
            } else {
                c
            }
        })
        .collect();
    let num = a.get("number").cloned().unwrap_or(Value::Null);
    let base = format!("{num}-{name}");
    if base.to_ascii_lowercase().ends_with(&format!(".{ext}")) {
        base
    } else {
        format!("{base}.{ext}")
    }
}

fn download(args: &[String]) -> ExitCode {
    let label = "issue artifact download";
    let version = match version_flag(args, &["--version"], label) {
        Ok(v) => v,
        Err(c) => return c,
    };
    let (c, issue, mut nums) = match setup(args, label) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if nums.is_empty() {
        match call(&c.base, &c.tok, "GET", &base_path(&c, issue), None, label) {
            Ok(Value::Array(items)) => {
                nums = items
                    .iter()
                    .filter_map(|a| a.get("number").and_then(Value::as_u64))
                    .collect();
            }
            Ok(_) => return usage(label, "unexpected response shape"),
            Err(code) => return code,
        }
    }
    let output = crate::flag_value(args, &["--output", "-O"]);
    if output.is_some() && nums.len() != 1 {
        return usage(label, "--output needs exactly one artifact");
    }
    let dir = crate::flag_value(args, &["--dir", "-D"]).unwrap_or(".");
    let clobber = has_flag(args, &["--clobber"]);
    let skip = has_flag(args, &["--skip-existing"]);
    let mut failed = false;
    for num in nums {
        let (a, versions) = match fetch_one(&c, issue, num, label) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let a = match at_version(&a, &versions, version, label) {
            Ok(v) => v,
            Err(code) => return code,
        };
        let (_, content) = file_payload(&a);
        if output == Some("-") {
            print!("{content}");
            continue;
        }
        let target = match output {
            Some(o) => Path::new(o).to_path_buf(),
            None => Path::new(dir).join(file_name_for(&a)),
        };
        if target.exists() {
            if skip {
                continue;
            }
            if !clobber {
                eprintln!(
                    "arks hub {label}: {} already exists (use --clobber or --skip-existing)",
                    target.display()
                );
                failed = true;
                continue;
            }
        }
        if let Some(parent) = target.parent().filter(|d| !d.as_os_str().is_empty()) {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&target, content.as_bytes()) {
            eprintln!("arks hub {label}: write {}: {e}", target.display());
            failed = true;
            continue;
        }
        println!("{}", target.display());
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_payloads() {
        assert_eq!(clean_name("plan (1).md"), "plan 1.md");
        let link = json!({"number": 3, "type": "link", "name": "example.com", "body": "https://example.com/x"});
        assert_eq!(file_name_for(&link), "3-example.com.url");
        assert!(file_payload(&link).1.contains("URL=https://example.com/x"));
        let doc = json!({"number": 2, "type": "generic", "name": "notes.md", "body": "# hi"});
        assert_eq!(file_name_for(&doc), "2-notes.md");
        assert_eq!(
            shortcut_url("[InternetShortcut]\r\nURL=https://a/b\r\n").as_deref(),
            Some("https://a/b")
        );
    }
}
