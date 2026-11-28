//! Embedded single-binary SSR Web UI for `arks serve` (E8).
//!
//! Compile-time embedded stylesheet (`include_str!`), strict HTML entity escaping
//! on every dynamic value, and `Content-Security-Policy: default-src 'self'; script-src 'none'`.
//! Never interferes with Git smart/dumb HTTP (`info/refs`, `git-upload-pack`,
//! `git-receive-pack`, `objects/`, `HEAD`) or Git LFS (`info/lfs/`).
//!
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.8 (E8).

use crate::http::{HttpRequest, HttpResponse};
use crate::serve::PageHandler;
use arks_object::{Commit, ObjectKind, Tree, TreeMode};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

pub const WEB_UI_CSP: &str = "default-src 'self'; script-src 'none'";
const STYLESHEET: &str = include_str!("web_ui.css");

/// Strictly escape HTML special characters to prevent XSS.
pub fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            other => out.push(other),
        }
    }
    out
}

fn html_response(status: u16, title: &str, body_html: &str) -> HttpResponse {
    let doc = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n\
         <title>{}</title>\n<link rel=\"stylesheet\" href=\"/style.css\">\n</head>\n<body>\n\
         <header>\n  <a href=\"/\">arks</a>\n  <a href=\"/commits\">Commits</a>\n  <a href=\"/refs\">Refs</a>\n</header>\n\
         <main>\n{body_html}\n</main>\n</body>\n</html>\n",
        html_escape(title)
    );
    HttpResponse {
        status,
        content_type: "text/html; charset=utf-8".into(),
        body: doc.into_bytes(),
        www_authenticate: None,
        extra_headers: vec![
            ("Content-Security-Policy".into(), WEB_UI_CSP.into()),
            ("X-Content-Type-Options".into(), "nosniff".into()),
        ],
    }
}

fn css_response() -> HttpResponse {
    HttpResponse {
        status: 200,
        content_type: "text/css; charset=utf-8".into(),
        body: STYLESHEET.as_bytes().to_vec(),
        www_authenticate: None,
        extra_headers: vec![
            ("Content-Security-Policy".into(), WEB_UI_CSP.into()),
            ("X-Content-Type-Options".into(), "nosniff".into()),
        ],
    }
}

/// Build a [`PageHandler`] serving the embedded SSR Web UI for `git_dir`.
pub fn web_ui_page_handler(git_dir: impl Into<PathBuf>) -> PageHandler {
    let dir = git_dir.into();
    Arc::new(move |req: &HttpRequest| handle_web_ui(&dir, req))
}

/// Route browser `GET` requests to the embedded SSR Web UI; returns `None` for
/// Git smart/dumb HTTP and LFS paths so protocol traffic passes through untouched.
pub fn handle_web_ui(git_dir: &std::path::Path, req: &HttpRequest) -> Option<HttpResponse> {
    if req.method != "GET" {
        return None;
    }
    let path = req.path.as_str();
    if path == "/style.css" {
        return Some(css_response());
    }
    // Never intercept Git smart/dumb HTTP or LFS routes.
    if path.contains("/info/refs")
        || path.contains("/info/lfs/")
        || path.contains("/git-upload-pack")
        || path.contains("/git-receive-pack")
        || path.starts_with("/objects/")
        || path == "/HEAD"
    {
        return None;
    }

    let Ok(repo) = Repository::open(git_dir) else {
        return Some(html_response(
            500,
            "arks — Error",
            "<h1>Repository unavailable</h1>",
        ));
    };

    if path == "/" || path.is_empty() {
        return Some(render_overview(&repo));
    }
    if path == "/refs" {
        return Some(render_refs(&repo));
    }
    if path == "/commits" {
        return Some(render_commits(&repo, "HEAD"));
    }
    if let Some(rev) = path.strip_prefix("/commits/") {
        return Some(render_commits(&repo, rev));
    }
    if let Some(oid_hex) = path.strip_prefix("/commit/") {
        return Some(render_commit_detail(&repo, oid_hex));
    }
    if let Some(rest) = path.strip_prefix("/tree/") {
        let (rev, subpath) = match rest.split_once('/') {
            Some((r, p)) => (r, p),
            None => (rest, ""),
        };
        return Some(render_tree(&repo, rev, subpath));
    }
    if let Some(rest) = path.strip_prefix("/blob/") {
        if let Some((rev, subpath)) = rest.split_once('/') {
            return Some(render_blob(&repo, rev, subpath));
        }
        return Some(html_response(
            404,
            "Not Found",
            "<h1>Blob path missing</h1>",
        ));
    }

    None
}

fn resolve_rev_simple(repo: &Repository, rev: &str) -> Option<Oid> {
    if rev == "HEAD" {
        return repo.refs().resolve("HEAD").ok();
    }
    if let Ok(oid) = Oid::from_hex(rev) {
        return Some(oid);
    }
    for candidate in [
        rev.to_string(),
        format!("refs/heads/{rev}"),
        format!("refs/tags/{rev}"),
    ] {
        if let Ok(oid) = repo.refs().resolve(&candidate) {
            return Some(oid);
        }
    }
    None
}

fn load_commit(repo: &Repository, oid: &Oid) -> Option<Commit> {
    let obj = repo.get_object(oid).ok()?;
    if obj.kind != ObjectKind::Commit {
        return None;
    }
    Commit::decode(&obj.payload).ok()
}

fn head_branch_label(repo: &Repository) -> String {
    match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => s.strip_prefix("refs/heads/").unwrap_or(&s).to_string(),
        Ok(RefTarget::Direct(o)) => o.to_hex()[..7.min(o.to_hex().len())].to_string(),
        Err(_) => "HEAD".into(),
    }
}

fn render_overview(repo: &Repository) -> HttpResponse {
    let branch = head_branch_label(repo);
    let refs_count = repo.refs().list_refs("refs/").map(|v| v.len()).unwrap_or(0);
    let mut html = format!(
        "<h1>Repository <span class=\"badge\">{}</span></h1>\n\
         <p class=\"meta\">{} refs &middot; <a href=\"/commits/{}\">Commit history</a> &middot; <a href=\"/refs\">All refs</a></p>\n",
        html_escape(&branch),
        refs_count,
        html_escape(&branch),
    );

    if let Some(head_oid) = resolve_rev_simple(repo, "HEAD") {
        if let Some(c) = load_commit(repo, &head_oid) {
            let short = &head_oid.to_hex()[..7.min(head_oid.to_hex().len())];
            let subj = c.message.lines().next().unwrap_or("");
            html.push_str(&format!(
                "<p>Latest commit <a href=\"/commit/{}\"><code>{}</code></a> — {} <span class=\"meta\">({})</span></p>\n",
                html_escape(&head_oid.to_hex()),
                html_escape(short),
                html_escape(subj),
                html_escape(&c.author.name),
            ));
            html.push_str(&render_tree_table(repo, &branch, &c.tree, ""));
        }
    } else {
        html.push_str("<p class=\"meta\">Empty repository (no commits yet).</p>\n");
    }

    html_response(200, "arks — Repository", &html)
}

fn render_tree_table(repo: &Repository, rev: &str, tree_oid: &Oid, subpath: &str) -> String {
    let Ok(obj) = repo.get_object(tree_oid) else {
        return "<p>Unable to read tree.</p>".into();
    };
    let Ok(tree) = Tree::decode(&obj.payload, repo.oid_kind()) else {
        return "<p>Corrupt tree.</p>".into();
    };
    let mut out = String::from(
        "<table><thead><tr><th>Name</th><th>Type</th><th>OID</th></tr></thead><tbody>\n",
    );
    for entry in tree.entries {
        let full = if subpath.is_empty() {
            entry.name.clone()
        } else {
            format!("{subpath}/{}", entry.name)
        };
        let is_dir = entry.mode == TreeMode::Tree;
        let href = if is_dir {
            format!("/tree/{}/{}", html_escape(rev), html_escape(&full))
        } else {
            format!("/blob/{}/{}", html_escape(rev), html_escape(&full))
        };
        let kind = if is_dir { "tree" } else { "blob" };
        let short = &entry.oid.to_hex()[..7.min(entry.oid.to_hex().len())];
        out.push_str(&format!(
            "<tr><td><a href=\"{href}\">{}{}</a></td><td class=\"meta\">{kind}</td><td><code>{}</code></td></tr>\n",
            html_escape(&entry.name),
            if is_dir { "/" } else { "" },
            html_escape(short)
        ));
    }
    out.push_str("</tbody></table>\n");
    out
}

fn find_tree_entry(repo: &Repository, root_tree: &Oid, subpath: &str) -> Option<(TreeMode, Oid)> {
    let mut cur_oid = *root_tree;
    let mut cur_mode = TreeMode::Tree;
    for part in subpath.split('/').filter(|p| !p.is_empty()) {
        let obj = repo.get_object(&cur_oid).ok()?;
        if obj.kind != ObjectKind::Tree {
            return None;
        }
        let tree = Tree::decode(&obj.payload, repo.oid_kind()).ok()?;
        let found = tree.entries.into_iter().find(|e| e.name == part)?;
        cur_mode = found.mode;
        cur_oid = found.oid;
    }
    Some((cur_mode, cur_oid))
}

fn render_tree(repo: &Repository, rev: &str, subpath: &str) -> HttpResponse {
    let Some(commit_oid) = resolve_rev_simple(repo, rev) else {
        return html_response(404, "Not Found", "<h1>Revision not found</h1>");
    };
    let Some(c) = load_commit(repo, &commit_oid) else {
        return html_response(404, "Not Found", "<h1>Commit not found</h1>");
    };
    let Some((mode, oid)) = find_tree_entry(repo, &c.tree, subpath) else {
        return html_response(404, "Not Found", "<h1>Path not found</h1>");
    };
    if mode != TreeMode::Tree {
        return render_blob(repo, rev, subpath);
    }
    let heading = format!(
        "<h1>Tree: <code>{}</code> @ <span class=\"badge\">{}</span></h1>\n",
        html_escape(if subpath.is_empty() { "/" } else { subpath }),
        html_escape(rev)
    );
    let table = render_tree_table(repo, rev, &oid, subpath);
    html_response(
        200,
        &format!("arks — {subpath}"),
        &format!("{heading}{table}"),
    )
}

fn render_blob(repo: &Repository, rev: &str, subpath: &str) -> HttpResponse {
    let Some(commit_oid) = resolve_rev_simple(repo, rev) else {
        return html_response(404, "Not Found", "<h1>Revision not found</h1>");
    };
    let Some(c) = load_commit(repo, &commit_oid) else {
        return html_response(404, "Not Found", "<h1>Commit not found</h1>");
    };
    let Some((mode, oid)) = find_tree_entry(repo, &c.tree, subpath) else {
        return html_response(404, "Not Found", "<h1>File not found</h1>");
    };
    if mode == TreeMode::Tree {
        return render_tree(repo, rev, subpath);
    }
    let Ok(obj) = repo.get_object(&oid) else {
        return html_response(404, "Not Found", "<h1>Blob missing</h1>");
    };
    let mut html = format!(
        "<h1>{} <span class=\"badge\">{}</span></h1>\n<p class=\"meta\">{} bytes &middot; <code>{}</code></p>\n",
        html_escape(subpath),
        html_escape(rev),
        obj.payload.len(),
        html_escape(&oid.to_hex())
    );
    if obj.payload.contains(&0) {
        html.push_str("<p class=\"meta\">Binary file not displayed.</p>\n");
    } else {
        let text = String::from_utf8_lossy(&obj.payload);
        html.push_str("<pre><code>");
        for (idx, line) in text.lines().enumerate() {
            html.push_str(&format!(
                "<span class=\"ln\">{}</span>{}\n",
                idx + 1,
                html_escape(line)
            ));
        }
        html.push_str("</code></pre>\n");
    }
    html_response(200, &format!("arks — {subpath}"), &html)
}

fn render_commits(repo: &Repository, rev: &str) -> HttpResponse {
    let Some(mut cur) = resolve_rev_simple(repo, rev) else {
        return html_response(404, "Not Found", "<h1>Revision not found</h1>");
    };
    let mut html = format!(
        "<h1>Commits <span class=\"badge\">{}</span></h1>\n\
         <table><thead><tr><th>Commit</th><th>Subject</th><th>Author</th></tr></thead><tbody>\n",
        html_escape(rev)
    );
    for _ in 0..50 {
        let Some(c) = load_commit(repo, &cur) else {
            break;
        };
        let hex = cur.to_hex();
        let short = &hex[..7.min(hex.len())];
        let subj = c.message.lines().next().unwrap_or("");
        html.push_str(&format!(
            "<tr><td><a href=\"/commit/{}\"><code>{}</code></a></td><td>{}</td><td class=\"meta\">{}</td></tr>\n",
            html_escape(&hex),
            html_escape(short),
            html_escape(subj),
            html_escape(&c.author.name)
        ));
        let Some(&parent) = c.parents.first() else {
            break;
        };
        cur = parent;
    }
    html.push_str("</tbody></table>\n");
    html_response(200, &format!("arks — Commits ({rev})"), &html)
}

fn render_commit_detail(repo: &Repository, oid_hex: &str) -> HttpResponse {
    let Some(oid) = resolve_rev_simple(repo, oid_hex) else {
        return html_response(404, "Not Found", "<h1>Commit not found</h1>");
    };
    let Some(c) = load_commit(repo, &oid) else {
        return html_response(404, "Not Found", "<h1>Commit not found</h1>");
    };
    let mut html = format!(
        "<h1>Commit <code>{}</code></h1>\n\
         <p class=\"meta\">Author: {} &lt;{}&gt; &middot; Committer: {}</p>\n\
         <pre>{}</pre>\n",
        html_escape(&oid.to_hex()),
        html_escape(&c.author.name),
        html_escape(&c.author.email),
        html_escape(&c.committer.name),
        html_escape(&c.message)
    );

    // Compute changed files vs first parent (or empty tree for root commit).
    let cur_files: BTreeMap<String, Oid> = arks_repo::tree_blob_map(repo, &oid)
        .unwrap_or_default()
        .into_iter()
        .collect();
    let parent_files: BTreeMap<String, Oid> = c
        .parents
        .first()
        .and_then(|p| arks_repo::tree_blob_map(repo, p).ok())
        .unwrap_or_default()
        .into_iter()
        .collect();

    html.push_str("<h2>Changed files</h2>\n<table><thead><tr><th>Status</th><th>Path</th></tr></thead><tbody>\n");
    for (p, o) in &cur_files {
        match parent_files.get(p) {
            None => html.push_str(&format!(
                "<tr><td>A</td><td><a href=\"/blob/{}/{}\">{}</a></td></tr>\n",
                html_escape(&oid.to_hex()),
                html_escape(p),
                html_escape(p)
            )),
            Some(po) if po != o => html.push_str(&format!(
                "<tr><td>M</td><td><a href=\"/blob/{}/{}\">{}</a></td></tr>\n",
                html_escape(&oid.to_hex()),
                html_escape(p),
                html_escape(p)
            )),
            _ => {}
        }
    }
    for p in parent_files.keys() {
        if !cur_files.contains_key(p) {
            html.push_str(&format!("<tr><td>D</td><td>{}</td></tr>\n", html_escape(p)));
        }
    }
    html.push_str("</tbody></table>\n");
    html_response(200, &format!("arks — Commit {oid_hex}"), &html)
}

fn render_refs(repo: &Repository) -> HttpResponse {
    let refs = repo.refs().list_refs("refs/").unwrap_or_default();
    let mut html = String::from(
        "<h1>References</h1>\n<table><thead><tr><th>Ref</th><th>OID</th></tr></thead><tbody>\n",
    );
    for (name, oid) in refs {
        let short = &oid.to_hex()[..7.min(oid.to_hex().len())];
        let link_rev = name
            .strip_prefix("refs/heads/")
            .or_else(|| name.strip_prefix("refs/tags/"))
            .unwrap_or(&name);
        html.push_str(&format!(
            "<tr><td><a href=\"/tree/{}\">{}</a></td><td><a href=\"/commit/{}\"><code>{}</code></a></td></tr>\n",
            html_escape(link_rev),
            html_escape(&name),
            html_escape(&oid.to_hex()),
            html_escape(short)
        ));
    }
    html.push_str("</tbody></table>\n");
    html_response(200, "arks — References", &html)
}
