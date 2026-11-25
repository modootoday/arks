//! Tree and file pages: `tree`, `blob` and the raw `blob_plain`.
//!
//! The object is `h` when given, else the path `f` in the commit `hb`
//! (default HEAD).

use super::html::{esc, href, link, table};
use super::model::Ctx;
use super::params::Params;
use super::{error, not_found, page, text};
use arks_object::{ObjectKind, TreeMode};
use arks_oid::Oid;
use arks_server::HttpResponse;

struct Located {
    /// The base revision as given (for links), when one was given.
    base: Option<String>,
    path: String,
    oid: Oid,
}

fn locate(ctx: &Ctx, p: &Params) -> Result<Located, HttpResponse> {
    let path = p.file.clone().unwrap_or_default();
    let base = p.hash_base.clone();
    if let Some(h) = &p.hash {
        let oid = ctx
            .resolve(h)
            .ok_or_else(|| not_found(ctx, "Cannot find file"))?;
        return Ok(Located { base, path, oid });
    }
    let rev = base.as_deref().unwrap_or("HEAD");
    let (_, c) = ctx
        .resolve_commit(rev)
        .ok_or_else(|| not_found(ctx, "Unknown commit object"))?;
    let oid = ctx
        .at_path(&c, &path)
        .ok_or_else(|| not_found(ctx, "Cannot find file"))?;
    Ok(Located { base, path, oid })
}

/// The `ls -l` style mode column.
fn mode_str(mode: TreeMode) -> &'static str {
    match mode {
        TreeMode::Tree => "drwxr-xr-x",
        TreeMode::Executable => "-rwxr-xr-x",
        TreeMode::Symlink => "lrwxrwxrwx",
        TreeMode::Gitlink => "m---------",
        _ => "-rw-r--r--",
    }
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// `hb` (when given) and `f` link pairs.
fn pairs<'a>(action: &'a str, base: Option<&'a str>, path: &'a str) -> Vec<(&'a str, &'a str)> {
    let mut v = vec![("a", action)];
    if let Some(b) = base {
        v.push(("hb", b));
    }
    if !path.is_empty() {
        v.push(("f", path));
    }
    v
}

pub fn tree(ctx: &Ctx, p: &Params) -> HttpResponse {
    let loc = match locate(ctx, p) {
        Ok(l) => l,
        Err(e) => return e,
    };
    let oid = match ctx.kind(&loc.oid) {
        Some(ObjectKind::Tree) => loc.oid,
        Some(ObjectKind::Commit) => match ctx.commit(&loc.oid) {
            Some(c) => c.tree,
            None => return not_found(ctx, "Cannot find file"),
        },
        Some(_) => return error(Some(ctx), 400, "Not a tree"),
        None => return not_found(ctx, "Cannot find file"),
    };
    let Some(entries) = ctx.tree(&oid) else {
        return not_found(ctx, "Cannot find file");
    };
    let pn = Some(ctx.name.as_str());
    let base = loc.base.as_deref();
    let mut rows = Vec::new();
    if !loc.path.is_empty() {
        let up = loc.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        rows.push(vec![
            "<span class=\"mode\">drwxr-xr-x</span>".into(),
            link(&href(pn, &pairs("tree", base, up)), ".."),
            String::new(),
        ]);
    }
    for e in &entries {
        let full = join(&loc.path, &e.name);
        let (name, links) = match e.mode {
            TreeMode::Tree => (
                link(&href(pn, &pairs("tree", base, &full)), &e.name),
                link(&href(pn, &pairs("tree", base, &full)), "tree"),
            ),
            TreeMode::Gitlink => (
                format!("{} @ <code>{}</code>", esc(&e.name), ctx.short(&e.oid)),
                String::new(),
            ),
            _ => (
                link(&href(pn, &pairs("blob", base, &full)), &e.name),
                format!(
                    "{} | {}",
                    link(&href(pn, &pairs("blob", base, &full)), "blob"),
                    link(&href(pn, &pairs("blob_plain", base, &full)), "raw")
                ),
            ),
        };
        rows.push(vec![
            format!("<span class=\"mode\">{}</span>", mode_str(e.mode)),
            name,
            links,
        ]);
    }
    let title = if loc.path.is_empty() { "/" } else { &loc.path };
    let body = format!(
        "<h2 class=\"path\">{}</h2>\n<p>tree <code>{}</code></p>\n{}",
        esc(title),
        oid.to_hex(),
        table("tree", &["Mode", "Name", ""], &rows)
    );
    page(ctx, "tree", base, &body)
}

/// Whether bytes look binary (a NUL in the first 8000 bytes, as git's
/// buffer_is_binary heuristic).
fn is_binary(b: &[u8]) -> bool {
    b[..b.len().min(8000)].contains(&0)
}

fn blob_of(ctx: &Ctx, p: &Params) -> Result<(Located, Vec<u8>), HttpResponse> {
    let loc = locate(ctx, p)?;
    match ctx.blob(&loc.oid) {
        Some(b) => Ok((loc, b)),
        None => Err(error(Some(ctx), 400, "Not a blob")),
    }
}

pub fn blob(ctx: &Ctx, p: &Params) -> HttpResponse {
    let (loc, bytes) = match blob_of(ctx, p) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let pn = Some(ctx.name.as_str());
    let base = loc.base.as_deref();
    let raw = if loc.path.is_empty() {
        href(pn, &[("a", "blob_plain"), ("h", &loc.oid.to_hex())])
    } else {
        href(pn, &pairs("blob_plain", base, &loc.path))
    };
    let mut body = format!(
        "<h2 class=\"path\">{}</h2>\n<p>blob <code>{}</code> | {}</p>\n",
        esc(&loc.path),
        loc.oid.to_hex(),
        link(&raw, "raw")
    );
    if is_binary(&bytes) {
        body.push_str(&format!(
            "<p class=\"binary\">Binary file ({} bytes); see {}.</p>\n",
            bytes.len(),
            link(&raw, "raw")
        ));
    } else {
        body.push_str("<table class=\"blob\">\n");
        let text = String::from_utf8_lossy(&bytes);
        for (i, line) in text.lines().enumerate() {
            let n = i + 1;
            body.push_str(&format!(
                "<tr id=\"l{n}\"><td class=\"lineno\"><a href=\"#l{n}\">{n}</a></td><td><pre>{}</pre></td></tr>\n",
                esc(line)
            ));
        }
        body.push_str("</table>\n");
    }
    page(ctx, "blob", base, &body)
}

pub fn blob_plain(ctx: &Ctx, p: &Params) -> HttpResponse {
    let (loc, bytes) = match blob_of(ctx, p) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let ty = if is_binary(&bytes) {
        "application/octet-stream"
    } else {
        "text/plain; charset=utf-8"
    };
    let mut resp = text(200, ty, bytes);
    let name = loc.path.rsplit('/').next().unwrap_or("");
    let name = if name.is_empty() {
        loc.oid.to_hex()
    } else {
        name.replace(['"', '\\', '\r', '\n'], "_")
    };
    resp.extra_headers.push((
        "Content-Disposition".into(),
        format!("inline; filename=\"{name}\""),
    ));
    resp
}
