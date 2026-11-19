//! `arks gist …` — gh-symmetric gist client (`create|list|view|edit|delete|rename|clone`).

use super::{Args, Ctx, Fail, Res, arr, read_input, row, s, usage};
use serde_json::{Map, Value, json};
use std::io::Write;
use std::path::Path;

pub(crate) const HELP: &str = "\
arks gist — gists on the arks forge (gh-symmetric argv; arks-owned; host-tested Partial)

USAGE:
  arks gist create [<filename>... | -] [-d <desc>] [-f <name>] [-p]
  arks gist list [-L <n>] [--public | --secret] [--filter <text>] [--include-content]
  arks gist view {<id> | <url>} [-f <name>] [--files] [-r]
  arks gist edit {<id> | <url>} [<content-file> | -] [-f <name>] [-a <file>] [-r <name>] [-d <desc>]
  arks gist delete {<id> | <url>} [--yes]
  arks gist rename {<id> | <url>} <old-filename> <new-filename>
  arks gist clone <gist> [<directory>]

A forge gist is a file map without git history; `gist clone` writes its files into
a directory and never spawns git. `edit` needs a content file (no editor is spawned).";

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "create" | "new" => create(ctx, rest, out),
        "list" | "ls" => list(ctx, rest, out),
        "view" => view(ctx, rest, out),
        "edit" => edit(ctx, rest, out),
        "delete" => delete(ctx, rest, out),
        "rename" => rename(ctx, rest, out),
        "clone" => clone(ctx, rest, out),
        other => usage(format!("unknown gist command `{other}`\n{HELP}")),
    }
}

/// Accept a bare id or a URL whose last path segment is the id.
fn gist_id(arg: Option<&str>) -> Result<String, Fail> {
    let a = arg.ok_or_else(|| Fail::Usage("gist id or URL required".into()))?;
    let id = a.trim_end_matches('/').rsplit('/').next().unwrap_or(a);
    if id.is_empty() {
        return usage("gist id or URL required");
    }
    Ok(id.to_string())
}

fn base_name(p: &str) -> Result<String, Fail> {
    Path::new(p)
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_string)
        .ok_or_else(|| Fail::Usage(format!("cannot take a file name from {p}")))
}

fn create(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &["-d", "--desc", "-f", "--filename"],
        &["-p", "--public", "-w", "--web"],
    )?;
    let mut files = Map::new();
    let pos = a.positionals();
    if pos.is_empty() || pos == ["-"] {
        let name = a.get(&["-f", "--filename"]).unwrap_or("gistfile0.txt");
        files.insert(name.into(), json!({"content": read_input(None)?}));
    } else {
        for (i, p) in pos.iter().enumerate() {
            let (name, content) = if *p == "-" {
                let name = a
                    .get(&["-f", "--filename"])
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("gistfile{i}.txt"));
                (name, read_input(None)?)
            } else {
                (base_name(p)?, read_input(Some(p))?)
            };
            files.insert(name, json!({"content": content}));
        }
    }
    let public = a.has(&["-p", "--public"]);
    let body = json!({
        "description": a.get(&["-d", "--desc"]).unwrap_or(""),
        "public": public,
        "files": files,
    });
    eprintln!(
        "- Creating gist with {} file(s) ({})",
        files.len(),
        if public { "public" } else { "secret" }
    );
    let g = ctx.api("POST", "/gists", Some(&body))?;
    writeln!(out, "{}", ctx.url(&format!("/gists/{}", s(&g, "id"))))?;
    if a.has(&["-w", "--web"]) {
        eprintln!("note: arks never opens a browser; use the URL above");
    }
    Ok(())
}

fn list(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &["-L", "--limit", "--filter"],
        &["--public", "--secret", "--include-content", "-w", "--web"],
    )?;
    let limit = a.limit(10)?;
    let (only_public, only_secret) = (a.has(&["--public"]), a.has(&["--secret"]));
    if only_public && only_secret {
        return usage("specify only one of --public or --secret");
    }
    let filter = a.get(&["--filter"]).map(str::to_ascii_lowercase);
    let include_content = a.has(&["--include-content"]);
    let all = ctx.api("GET", "/gists", None)?;
    let mut shown = 0;
    for g in arr(&all) {
        let public = g.get("public").and_then(Value::as_bool).unwrap_or(false);
        if (only_public && !public) || (only_secret && public) {
            continue;
        }
        let files = g.get("files").and_then(Value::as_object);
        if let Some(f) = &filter {
            let mut hay = s(g, "description").to_ascii_lowercase();
            for (name, file) in files.into_iter().flatten() {
                hay.push(' ');
                hay.push_str(&name.to_ascii_lowercase());
                if include_content {
                    hay.push(' ');
                    hay.push_str(&s(file, "content").to_ascii_lowercase());
                }
            }
            if !hay.contains(f.as_str()) {
                continue;
            }
        }
        let count = files.map_or(0, Map::len);
        let count_s = format!("{count} file{}", if count == 1 { "" } else { "s" });
        let vis = if public { "public" } else { "secret" };
        row(
            out,
            &[
                s(g, "id"),
                s(g, "description"),
                &count_s,
                vis,
                s(g, "updated_at"),
            ],
        )?;
        shown += 1;
        if shown >= limit {
            break;
        }
    }
    Ok(())
}

fn fetch(ctx: &Ctx, id: &str) -> Result<Value, Fail> {
    ctx.api("GET", &format!("/gists/{id}"), None)
}

fn view(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &["-f", "--filename"],
        &["--files", "-r", "--raw", "-w", "--web"],
    )?;
    let id = gist_id(a.positionals().first().copied())?;
    if a.has(&["-w", "--web"]) {
        writeln!(out, "{}", ctx.url(&format!("/gists/{id}")))?;
        return Ok(());
    }
    let g = fetch(ctx, &id)?;
    let files = g
        .get("files")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if a.has(&["--files"]) {
        for name in files.keys() {
            writeln!(out, "{name}")?;
        }
        return Ok(());
    }
    if let Some(name) = a.get(&["-f", "--filename"]) {
        let f = files
            .get(name)
            .ok_or_else(|| Fail::Api(format!("gist has no file {name:?}")))?;
        write_content(out, s(f, "content"))?;
        return Ok(());
    }
    let raw_mode = a.has(&["-r", "--raw"]);
    if !raw_mode && !s(&g, "description").is_empty() {
        writeln!(out, "{}\n", s(&g, "description"))?;
    }
    let many = files.len() > 1;
    for (i, (name, f)) in files.iter().enumerate() {
        if !raw_mode && many {
            writeln!(out, "{name}\n")?;
        }
        write_content(out, s(f, "content"))?;
        if i + 1 < files.len() {
            writeln!(out)?;
        }
    }
    Ok(())
}

fn write_content(out: &mut dyn Write, c: &str) -> Res {
    out.write_all(c.as_bytes())?;
    if !c.ends_with('\n') {
        writeln!(out)?;
    }
    Ok(())
}

fn edit(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(
        raw,
        &[
            "-a",
            "--add",
            "-d",
            "--desc",
            "-f",
            "--filename",
            "-r",
            "--remove",
        ],
        &[],
    )?;
    let pos = a.positionals();
    let id = gist_id(pos.first().copied())?;
    let source = pos.get(1).copied();
    let g = fetch(ctx, &id)?;
    let files = g
        .get("files")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut patch = Map::new();
    if let Some(add) = a.get(&["-a", "--add"]) {
        let name = base_name(add)?;
        if files.contains_key(&name) {
            return usage(format!(
                "gist already has a file named {name:?}; use -f to edit it"
            ));
        }
        patch.insert(name, json!({"content": read_input(Some(add))?}));
    }
    if let Some(rm) = a.get(&["-r", "--remove"]) {
        if !files.contains_key(rm) {
            return usage(format!("gist has no file {rm:?}"));
        }
        patch.insert(rm.to_string(), Value::Null);
    }
    if let Some(src) = source {
        let target = match a.get(&["-f", "--filename"]) {
            Some(n) => n.to_string(),
            None if files.len() == 1 => files.keys().next().cloned().unwrap_or_default(),
            None if src != "-" => base_name(src)?,
            None => return usage("gist has several files; pick one with -f <name>"),
        };
        patch.insert(target, json!({"content": read_input(Some(src))?}));
    } else if a.get(&["-f", "--filename"]).is_some() {
        return usage("arks does not spawn an editor; pass a content file (or - for stdin)");
    }
    let mut body = Map::new();
    if let Some(d) = a.get(&["-d", "--desc"]) {
        body.insert("description".into(), json!(d));
    }
    if !patch.is_empty() {
        body.insert("files".into(), Value::Object(patch));
    }
    if body.is_empty() {
        return usage(
            "nothing to edit: pass a content file, --add, --remove or --desc (no editor is spawned)",
        );
    }
    ctx.api("PATCH", &format!("/gists/{id}"), Some(&Value::Object(body)))?;
    writeln!(out, "{}", ctx.url(&format!("/gists/{id}")))?;
    Ok(())
}

fn delete(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &[], &["--yes", "-y"])?;
    let id = gist_id(a.positionals().first().copied())?;
    ctx.api("DELETE", &format!("/gists/{id}"), None)?;
    writeln!(out, "Gist \"{id}\" deleted")?;
    Ok(())
}

fn rename(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &[], &[])?;
    let pos = a.positionals();
    let [id, old, new] = pos.as_slice() else {
        return usage("usage: arks gist rename {<id> | <url>} <old-filename> <new-filename>");
    };
    let id = gist_id(Some(id))?;
    let body = json!({"files": {*old: {"filename": *new}}});
    ctx.api("PATCH", &format!("/gists/{id}"), Some(&body))?;
    writeln!(out, "Renamed {old} to {new} in gist {id}")?;
    Ok(())
}

fn clone(ctx: &Ctx, raw: &[String], out: &mut dyn Write) -> Res {
    let a = Args::parse(raw, &[], &[])?;
    let pos = a.positionals();
    let id = gist_id(pos.first().copied())?;
    if !a.passthrough().is_empty() {
        eprintln!("note: git flags after `--` are ignored; arks does not spawn git");
    }
    let dir = pos
        .get(1)
        .map(|d| d.to_string())
        .unwrap_or_else(|| id.clone());
    let path = Path::new(&dir);
    if path.exists()
        && std::fs::read_dir(path)
            .map(|mut d| d.next().is_some())
            .unwrap_or(true)
    {
        return usage(format!(
            "destination path {dir:?} already exists and is not empty"
        ));
    }
    let g = fetch(ctx, &id)?;
    std::fs::create_dir_all(path)?;
    let files = g
        .get("files")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (name, f) in &files {
        // Server refuses path separators; re-check before touching the disk.
        if name.contains('/') || name.contains('\\') || name == ".." || name == "." {
            return Err(Fail::Api(format!(
                "refusing unsafe gist file name {name:?}"
            )));
        }
        std::fs::write(path.join(name), s(f, "content"))?;
    }
    writeln!(out, "Cloned gist {id} into {dir} ({} files)", files.len())?;
    writeln!(
        out,
        "recipe=cd {dir} && arks init && arks add . && arks commit -m \"gist {id}\""
    )?;
    eprintln!("note: forge gists carry no git history; arks never spawns git");
    Ok(())
}
