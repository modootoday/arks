//! `arks release …` — gh-symmetric release client against the arks forge
//! (create with assets, list, view, upload, download incl. source archives,
//! edit, delete, delete-asset). `--web` prints the URL instead of a browser.

use super::common::{enc, seg};
use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, n, read_input, row, s, usage, web};
use serde_json::{Map, Value, json};
use std::io::Write;
use std::path::PathBuf;

pub(crate) const HELP: &str = "\
arks release — releases on the arks forge (gh-symmetric argv; host-tested Partial)

USAGE:
  arks release <command> [flags]

COMMANDS:
  create, list, view, upload, download, edit, delete, delete-asset
  (verify / verify-asset are GitHub attestations: out of scope)

Flags are gh's; `-R/--repo [HOST/]OWNER/REPO` selects the repository. Tags are
created on the forge at --target when missing (Git content); --generate-notes lists
the merged pull requests / commits since the previous release.";

const LIST_FIELDS: &[&str] = &[
    "createdAt",
    "isDraft",
    "isImmutable",
    "isLatest",
    "isPrerelease",
    "name",
    "publishedAt",
    "tagName",
];
const VIEW_FIELDS: &[&str] = &[
    "apiUrl",
    "assets",
    "author",
    "body",
    "createdAt",
    "databaseId",
    "id",
    "isDraft",
    "isImmutable",
    "isPrerelease",
    "name",
    "publishedAt",
    "tagName",
    "tarballUrl",
    "targetCommitish",
    "uploadUrl",
    "url",
    "zipballUrl",
];

const R: &[&str] = &["-R", "--repo"];

fn vals(extra: &[&'static str]) -> Vec<&'static str> {
    let mut v: Vec<&str> = R.to_vec();
    v.extend_from_slice(extra);
    v
}

fn gh_row(ctx: &Ctx, o: &str, r: &str, x: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    let mut put = |k: &str, v: Value| {
        m.insert(k.to_string(), v);
    };
    put("apiUrl", x["url"].clone());
    put("assets", Value::Array(arr(&x["assets"]).iter().map(|a| json!({"name": a["name"], "label": a["label"], "size": a["size"], "contentType": a["content_type"], "createdAt": a["created_at"], "downloadCount": a["download_count"], "state": "uploaded", "url": a["browser_download_url"], "apiUrl": a["url"]})).collect()));
    put("author", json!({"login": x["author"]["login"]}));
    put("body", x["body"].clone());
    put("createdAt", x["created_at"].clone());
    put("databaseId", x["id"].clone());
    put("id", x["node_id"].clone());
    put("isDraft", x["draft"].clone());
    put("isImmutable", x["immutable"].clone());
    put("isLatest", x["is_latest"].clone());
    put("isPrerelease", x["prerelease"].clone());
    put("name", x["name"].clone());
    put("publishedAt", x["published_at"].clone());
    put("tagName", x["tag_name"].clone());
    put("tarballUrl", x["tarball_url"].clone());
    put("targetCommitish", x["target_commitish"].clone());
    put("uploadUrl", x["upload_url"].clone());
    put(
        "url",
        json!(ctx.web_url(&format!("/{o}/{r}/releases/tag/{}", s(x, "tag_name")))),
    );
    put("zipballUrl", x["zipball_url"].clone());
    m
}

fn by_tag(ctx: &Ctx, o: &str, r: &str, tag: &str) -> Result<Value, Fail> {
    ctx.get(&format!("/repos/{o}/{r}/releases/tags/{}", seg(tag)))
        .map_err(|e| match e {
            Fail::Api(m) if m.contains("404") => Fail::Api("release not found".into()),
            other => other,
        })
}

/// `path[#label]` asset arguments → (path, display name, label).
fn asset_args(items: &[&str]) -> Result<Vec<(PathBuf, String, String)>, Fail> {
    items
        .iter()
        .map(|spec| {
            let (p, label) = match spec.split_once('#') {
                Some((p, l)) => (p, l.to_string()),
                None => (*spec, String::new()),
            };
            let path = PathBuf::from(p);
            if !path.is_file() {
                return usage(format!("open {p}: no such file"));
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(p)
                .to_string();
            Ok((path, name, label))
        })
        .collect()
}

fn upload(
    ctx: &Ctx,
    o: &str,
    r: &str,
    rel: &Value,
    files: &[(PathBuf, String, String)],
    clobber: bool,
) -> Res {
    for (path, name, label) in files {
        if let Some(old) = arr(&rel["assets"]).iter().find(|a| s(a, "name") == name) {
            if !clobber {
                return Err(Fail::Api(format!(
                    "asset under the same name already exists: [{name}]"
                )));
            }
            ctx.api(
                "DELETE",
                &format!("/repos/{o}/{r}/releases/assets/{}", n(old, "id")),
                None,
            )?;
        }
        let bytes = std::fs::read(path)?;
        let mut q = format!(
            "/repos/{o}/{r}/releases/{}/assets?name={}",
            n(rel, "id"),
            enc(name)
        );
        if !label.is_empty() {
            q.push_str(&format!("&label={}", enc(label)));
        }
        ctx.raw(
            "POST",
            &q,
            Some(&bytes),
            "application/octet-stream",
            "application/json",
        )?;
    }
    Ok(())
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "create" | "new" => create(ctx, rest, out),
        "list" | "ls" => list(ctx, rest, out),
        "view" => view(ctx, rest, out),
        "upload" => {
            // `--tag/--file/--name` are the legacy arks spellings of gh's positionals.
            let a = Args::parse(rest, &vals(&["--tag", "--file", "--name"]), &["--clobber"])?;
            let (o, r) = ctx.repo(&a)?;
            let pos = a.positionals();
            let tag = a
                .get(&["--tag"])
                .or(pos.first().copied())
                .ok_or_else(|| Fail::Usage("usage: arks release upload <tag> <files>...".into()))?;
            let mut items: Vec<&str> = if a.get(&["--tag"]).is_some() {
                pos.clone()
            } else {
                pos.get(1..).unwrap_or(&[]).to_vec()
            };
            let legacy;
            if let Some(f) = a.get(&["--file"]) {
                legacy = match a.get(&["--name"]) {
                    Some(nm) => {
                        let dir = std::env::temp_dir()
                            .join(format!("arks-upload-{}", std::process::id()));
                        std::fs::create_dir_all(&dir)?;
                        let to = dir.join(nm);
                        std::fs::copy(f, &to)?;
                        to.display().to_string()
                    }
                    None => f.to_string(),
                };
                items.push(&legacy);
            }
            if items.is_empty() {
                return usage("usage: arks release upload <tag> <files>...");
            }
            let rel = by_tag(ctx, &o, &r, tag)?;
            upload(
                ctx,
                &o,
                &r,
                &rel,
                &asset_args(&items)?,
                a.has(&["--clobber"]),
            )?;
            let _ = out;
            Ok(())
        }
        "download" => download(ctx, rest, out),
        "edit" => edit(ctx, rest, out),
        "delete" => {
            // `--tag` is the legacy arks spelling of gh's positional tag.
            let a = Args::parse(rest, &vals(&["--tag"]), &["--cleanup-tag", "-y", "--yes"])?;
            let (o, r) = ctx.repo(&a)?;
            let tag = a
                .get(&["--tag"])
                .or(a.positionals().first().copied())
                .ok_or_else(|| Fail::Usage("usage: arks release delete <tag>".into()))?;
            let rel = by_tag(ctx, &o, &r, tag)?;
            ctx.api(
                "DELETE",
                &format!("/repos/{o}/{r}/releases/{}", n(&rel, "id")),
                None,
            )?;
            if a.has(&["--cleanup-tag"]) {
                ctx.api(
                    "DELETE",
                    &format!("/repos/{o}/{r}/git/refs/tags/{tag}"),
                    None,
                )?;
            }
            eprintln!(
                "✓ Deleted release {tag}{}",
                if a.has(&["--cleanup-tag"]) {
                    " and tag"
                } else {
                    ""
                }
            );
            let _ = out;
            Ok(())
        }
        "delete-asset" => {
            // `--asset-id` is the legacy arks spelling (gh: <tag> <asset-name>).
            let a = Args::parse(rest, &vals(&["--asset-id"]), &["-y", "--yes"])?;
            let (o, r) = ctx.repo(&a)?;
            let id = if let Some(id) = a.get(&["--asset-id"]) {
                id.to_string()
            } else {
                let pos = a.positionals();
                let (Some(tag), Some(name)) = (pos.first(), pos.get(1)) else {
                    return usage("usage: arks release delete-asset <tag> <asset-name>");
                };
                let rel = by_tag(ctx, &o, &r, tag)?;
                let asset = arr(&rel["assets"])
                    .iter()
                    .find(|x| s(x, "name") == *name)
                    .cloned()
                    .ok_or_else(|| Fail::Api(format!("asset {name} not found in release {tag}")))?;
                n(&asset, "id").to_string()
            };
            ctx.api(
                "DELETE",
                &format!("/repos/{o}/{r}/releases/assets/{id}"),
                None,
            )?;
            eprintln!("✓ Deleted asset {id}");
            let _ = out;
            Ok(())
        }
        "verify" | "verify-asset" => usage(format!(
            "release {sub}: GitHub-platform-only (attestations) — out of scope for the arks forge \
             ([[adr:root/20261119230000]]); refuse exit 2"
        )),
        other => usage(format!(
            "unknown command {other:?} for \"arks release\"\n{HELP}"
        )),
    }
}

fn create(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&[
            "--discussion-category",
            "-n",
            "--notes",
            "-F",
            "--notes-file",
            "--notes-start-tag",
            "--target",
            "-t",
            "--title",
            "--tag",
        ]),
        &[
            "-d",
            "--draft",
            "--fail-on-no-commits",
            "--generate-notes",
            "--latest",
            "--notes-from-tag",
            "-p",
            "--prerelease",
            "--verify-tag",
        ],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let pos = a.positionals();
    // `--tag` is the legacy arks spelling of gh's positional tag.
    let (tag, files) = match a.get(&["--tag"]) {
        Some(t) => (t, pos.as_slice()),
        None => match pos.split_first() {
            Some((t, f)) => (*t, f),
            None => return usage("could not create: no tag name provided"),
        },
    };
    if a.has(&["--notes-from-tag"])
        && (a.get(&["-n", "--notes", "-F", "--notes-file"]).is_some()
            || a.has(&["--generate-notes"]))
    {
        return usage(
            "using `--notes-from-tag` with `--generate-notes` or `--notes` / `--notes-file` is not supported",
        );
    }
    let assets = asset_args(files)?;
    let notes = match (a.get(&["-n", "--notes"]), a.get(&["-F", "--notes-file"])) {
        (Some(nt), _) => Some(nt.to_string()),
        (None, Some(f)) => Some(read_input(Some(f))?),
        _ => None,
    };
    if a.has(&["--fail-on-no-commits"]) {
        if let Ok(latest) = ctx.get(&format!("/repos/{o}/{r}/releases/latest")) {
            let target = a.get(&["--target"]).map(str::to_string);
            let base = s(&latest, "tag_name");
            let tip = target.unwrap_or_else(|| s(&latest, "target_commitish").to_string());
            let (Ok(b), Ok(t)) = (
                ctx.get(&format!("/repos/{o}/{r}/commits/{}", seg(base))),
                ctx.get(&format!("/repos/{o}/{r}/commits/{}", seg(&tip))),
            ) else {
                return Err(Fail::Api(
                    "cannot compare commits without Git content on the forge".into(),
                ));
            };
            if s(&b, "sha") == s(&t, "sha") {
                return Err(Fail::Api(format!(
                    "no new commits since the last release ({base})"
                )));
            }
        }
    }
    let mut body = json!({
        "tag_name": tag,
        "draft": a.has(&["-d", "--draft"]),
        "prerelease": a.has(&["-p", "--prerelease"]),
        "generate_release_notes": a.has(&["--generate-notes"]),
        "notes_from_tag": a.has(&["--notes-from-tag"]),
        "verify_tag": a.has(&["--verify-tag"]),
    });
    if let Some(t) = a.get(&["-t", "--title"]) {
        body["name"] = json!(t);
    }
    if let Some(nt) = notes {
        body["body"] = json!(nt);
    }
    if let Some(t) = a.get(&["--target"]) {
        body["target_commitish"] = json!(t);
    }
    if let Some(l) = a.bool_opt(&["--latest"]) {
        body["make_latest"] = json!(l.to_string());
    }
    if let Some(c) = a.get(&["--discussion-category"]) {
        body["discussion_category_name"] = json!(c);
    }
    if a.has(&["--generate-notes"]) {
        if let Some(start) = a.get(&["--notes-start-tag"]) {
            let notes_gen = ctx.api("POST", &format!("/repos/{o}/{r}/releases/generate-notes"), Some(&json!({"tag_name": tag, "previous_tag_name": start, "target_commitish": body["target_commitish"]})))?;
            let joined = match body["body"].as_str() {
                Some(b) => format!("{b}\n\n{}", s(&notes_gen, "body")),
                None => s(&notes_gen, "body").to_string(),
            };
            body["body"] = json!(joined);
            body["generate_release_notes"] = json!(false);
        }
    }
    // Upload assets to a draft first, then publish (gh's order).
    let publish = !assets.is_empty() && body["draft"] == json!(false);
    if publish {
        body["draft"] = json!(true);
    }
    let rel = ctx.api("POST", &format!("/repos/{o}/{r}/releases"), Some(&body))?;
    upload(ctx, &o, &r, &rel, &assets, false)?;
    if publish {
        ctx.api(
            "PATCH",
            &format!("/repos/{o}/{r}/releases/{}", n(&rel, "id")),
            Some(&json!({"draft": false})),
        )?;
    }
    writeln!(
        out,
        "{}",
        ctx.web_url(&format!("/{o}/{r}/releases/tag/{tag}"))
    )?;
    Ok(())
}

fn list(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[vals(&["-L", "--limit", "-O", "--order"]), EXPORT.to_vec()].concat(),
        &["--exclude-drafts", "--exclude-pre-releases"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let order = a.get(&["-O", "--order"]).unwrap_or("desc");
    if !matches!(order, "asc" | "desc") {
        return usage(format!(
            "invalid argument {order:?} for \"-O, --order\" flag: valid values are {{asc|desc}}"
        ));
    }
    let mut items = ctx
        .get(&format!("/repos/{o}/{r}/releases"))?
        .as_array()
        .cloned()
        .unwrap_or_default();
    items.retain(|x| {
        !(a.has(&["--exclude-drafts"]) && x["draft"].as_bool() == Some(true))
            && !(a.has(&["--exclude-pre-releases"]) && x["prerelease"].as_bool() == Some(true))
    });
    if order == "asc" {
        items.reverse();
    }
    items.truncate(a.limit(30)?);
    let rows: Vec<Map<String, Value>> = items.iter().map(|x| gh_row(ctx, &o, &r, x)).collect();
    if json_out(out, &a, &rows, LIST_FIELDS, false)? {
        return Ok(());
    }
    if rows.is_empty() {
        eprintln!("no releases found");
        return Ok(());
    }
    for x in &items {
        let kind = if x["is_latest"].as_bool() == Some(true) {
            "Latest"
        } else if x["draft"].as_bool() == Some(true) {
            "Draft"
        } else if x["prerelease"].as_bool() == Some(true) {
            "Pre-release"
        } else {
            ""
        };
        let when = x["published_at"]
            .as_str()
            .unwrap_or_else(|| s(x, "created_at"));
        row(out, &[s(x, "name"), kind, s(x, "tag_name"), when])?;
    }
    Ok(())
}

fn view(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[vals(&["--tag"]), EXPORT.to_vec()].concat(),
        &["-w", "--web"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let rel = match a.get(&["--tag"]).or(a.positionals().first().copied()) {
        Some(t) if t.chars().all(|c| c.is_ascii_digit()) => {
            ctx.get(&format!("/repos/{o}/{r}/releases/{t}"))?
        }
        Some(t) => by_tag(ctx, &o, &r, t)?,
        None => ctx.get(&format!("/repos/{o}/{r}/releases/latest"))?,
    };
    let m = gh_row(ctx, &o, &r, &rel);
    if a.has(&["-w", "--web"]) {
        return web(out, m["url"].as_str().unwrap_or(""));
    }
    if json_out(out, &a, std::slice::from_ref(&m), VIEW_FIELDS, true)? {
        return Ok(());
    }
    writeln!(out, "title:\t{}", s(&rel, "name"))?;
    writeln!(out, "tag:\t{}", s(&rel, "tag_name"))?;
    writeln!(out, "draft:\t{}", rel["draft"])?;
    writeln!(out, "prerelease:\t{}", rel["prerelease"])?;
    writeln!(out, "author:\t{}", s(&rel["author"], "login"))?;
    writeln!(out, "created:\t{}", s(&rel, "created_at"))?;
    writeln!(
        out,
        "published:\t{}",
        rel["published_at"].as_str().unwrap_or("")
    )?;
    writeln!(out, "url:\t{}", m["url"].as_str().unwrap_or(""))?;
    for x in arr(&rel["assets"]) {
        writeln!(out, "asset:\t{}", s(x, "name"))?;
    }
    writeln!(out, "--")?;
    writeln!(out, "{}", s(&rel, "body"))?;
    Ok(())
}

fn download(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    // `--asset-id` is the legacy arks spelling; `-o` is its old output flag.
    let a = Args::parse(
        raw,
        &vals(&[
            "-A",
            "--archive",
            "-D",
            "--dir",
            "-O",
            "--output",
            "-o",
            "-p",
            "--pattern",
            "--asset-id",
        ]),
        &["--clobber", "--skip-existing"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let output = a.get(&["-O", "--output", "-o"]);
    let dir = PathBuf::from(a.get(&["-D", "--dir"]).unwrap_or("."));
    let write_one = |out: &mut dyn Write, name: &str, bytes: &[u8]| -> Res {
        if output == Some("-") {
            out.write_all(bytes)?;
            return Ok(());
        }
        let path = match output {
            Some(p) => PathBuf::from(p),
            None => dir.join(name),
        };
        if path.exists() && !a.has(&["--clobber"]) {
            if a.has(&["--skip-existing"]) {
                return Ok(());
            }
            return Err(Fail::Api(format!(
                "{} already exists (use `--clobber` to overwrite file or `--skip-existing` to skip file)",
                path.display()
            )));
        }
        if let Some(p) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(&path, bytes)?;
        if a.get(&["--asset-id"]).is_some() {
            writeln!(out, "wrote={} bytes={}", path.display(), bytes.len())?;
        }
        Ok(())
    };
    if let Some(id) = a.get(&["--asset-id"]) {
        let bytes = ctx.raw(
            "GET",
            &format!("/repos/{o}/{r}/releases/assets/{id}"),
            None,
            "application/json",
            "application/octet-stream",
        )?;
        return write_one(out, &format!("asset-{id}"), &bytes);
    }
    let rel = match a.positionals().first() {
        Some(t) => by_tag(ctx, &o, &r, t)?,
        None => ctx.get(&format!("/repos/{o}/{r}/releases/latest"))?,
    };
    if let Some(fmt) = a.get(&["-A", "--archive"]) {
        let (kind, ext) = match fmt {
            "zip" => ("zipball", "zip"),
            "tar.gz" => ("tarball", "tar.gz"),
            other => {
                return usage(format!(
                    "the value for `--archive` must be one of \"zip\" or \"tar.gz\", got {other:?}"
                ));
            }
        };
        let tag = s(&rel, "tag_name");
        let bytes = ctx.raw(
            "GET",
            &format!("/repos/{o}/{r}/{kind}/{}", seg(tag)),
            None,
            "application/json",
            "application/octet-stream",
        )?;
        return write_one(
            out,
            &format!("{r}-{}.{ext}", tag.trim_start_matches('v')),
            &bytes,
        );
    }
    let pats = a.get_all(&["-p", "--pattern"]);
    let assets: Vec<&Value> = arr(&rel["assets"])
        .iter()
        .filter(|x| {
            pats.is_empty()
                || pats
                    .iter()
                    .any(|p| crate::forge::rules::glob(p, s(x, "name")))
        })
        .collect();
    if assets.is_empty() {
        return Err(Fail::Api("no assets to download".into()));
    }
    if output.is_some() && assets.len() > 1 {
        return usage(
            "multiple assets match; `--output` needs exactly one (narrow with --pattern)",
        );
    }
    for x in assets {
        let bytes = ctx.raw(
            "GET",
            &format!("/repos/{o}/{r}/releases/assets/{}", n(x, "id")),
            None,
            "application/json",
            "application/octet-stream",
        )?;
        write_one(out, s(x, "name"), &bytes)?;
    }
    Ok(())
}

fn edit(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &vals(&[
            "--discussion-category",
            "-n",
            "--notes",
            "-F",
            "--notes-file",
            "--tag",
            "--target",
            "-t",
            "--title",
        ]),
        &["--draft", "--latest", "--prerelease", "--verify-tag"],
    )?;
    let (o, r) = ctx.repo(&a)?;
    let pos = a.positionals();
    // gh: `release edit <tag> [--tag new]`; legacy arks: `release edit --tag <tag>`.
    let (cur, new_tag) = match pos.first() {
        Some(t) => (*t, a.get(&["--tag"])),
        None => (
            a.get(&["--tag"])
                .ok_or_else(|| Fail::Usage("usage: arks release edit <tag>".into()))?,
            None,
        ),
    };
    let rel = by_tag(ctx, &o, &r, cur)?;
    let mut body = Map::new();
    if let Some(t) = a.get(&["-t", "--title"]) {
        body.insert("name".into(), json!(t));
    }
    match (a.get(&["-n", "--notes"]), a.get(&["-F", "--notes-file"])) {
        (Some(nt), _) => {
            body.insert("body".into(), json!(nt));
        }
        (None, Some(f)) => {
            body.insert("body".into(), json!(read_input(Some(f))?));
        }
        _ => {}
    }
    if let Some(t) = new_tag {
        body.insert("tag_name".into(), json!(t));
    }
    if let Some(t) = a.get(&["--target"]) {
        body.insert("target_commitish".into(), json!(t));
    }
    for (flag, key) in [("--draft", "draft"), ("--prerelease", "prerelease")] {
        if let Some(b) = a.bool_opt(&[flag]) {
            body.insert(key.into(), json!(b));
        }
    }
    if let Some(l) = a.bool_opt(&["--latest"]) {
        body.insert("make_latest".into(), json!(l.to_string()));
    }
    if a.has(&["--verify-tag"]) {
        body.insert("verify_tag".into(), json!(true));
    }
    if let Some(c) = a.get(&["--discussion-category"]) {
        body.insert("discussion_category_name".into(), json!(c));
    }
    if body.is_empty() {
        return usage("nothing to edit: pass at least one flag");
    }
    let v = ctx.api(
        "PATCH",
        &format!("/repos/{o}/{r}/releases/{}", n(&rel, "id")),
        Some(&Value::Object(body)),
    )?;
    writeln!(
        out,
        "{}",
        ctx.web_url(&format!("/{o}/{r}/releases/tag/{}", s(&v, "tag_name")))
    )?;
    Ok(())
}
