//! Embedded single-binary SSR Web UI for `arks serve` (E8 — GitHub/GitLab common interface).
//!
//! Compile-time embedded stylesheet (`include_str!`), strict HTML entity escaping
//! on every dynamic value, and `Content-Security-Policy: default-src 'self'; script-src 'none'`.
//! Never interferes with Git smart/dumb HTTP (`info/refs`, `git-upload-pack`,
//! `git-receive-pack`, `objects/`, `HEAD`) or Git LFS (`info/lfs/`).
//!
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.8 (E8).

use crate::http::{HttpRequest, HttpResponse};
use crate::serve::PageHandler;
use arks_object::{Commit, ObjectKind, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub const WEB_UI_CSP: &str = "default-src 'self'; script-src 'self'";
const STYLESHEET: &str = include_str!("web_ui.css");
const HTMX_JS: &str = include_str!("assets/htmx.min.js");
const FAVICON_SVG: &str = include_str!("assets/favicon.svg");

const ICON_DIR: &str = "<svg class=\"icon icon-dir\" viewBox=\"0 0 16 16\" aria-hidden=\"true\"><path d=\"M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5a.25.25 0 0 1-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75Z\"/></svg>";
const ICON_FILE: &str = "<svg class=\"icon icon-file\" viewBox=\"0 0 16 16\" aria-hidden=\"true\"><path d=\"M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25Zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h9.5a.25.25 0 0 0 .25-.25V6h-2.75A1.75 1.75 0 0 1 9 4.25V1.5Zm6.75.062V4.25c0 .138.112.25.25.25h2.688l-.011-.013-2.914-2.914-.013-.011Z\"/></svg>";

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActiveTab {
    Code,
    Commits,
    Branches,
    Tags,
}

struct RepoContext {
    name: String,
    description: String,
    default_branch: String,
    branches: Vec<(String, Oid)>,
    tags: Vec<(String, Oid)>,
    clone_url: String,
    has_lfs: bool,
}

impl RepoContext {
    fn from_repo(repo: &Repository, host: Option<&str>) -> Self {
        let common = repo.common_dir();
        let name = common
            .file_name()
            .and_then(|s| s.to_str())
            .map(|n| {
                if n == ".git" {
                    common
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|s| s.to_str())
                        .unwrap_or("repository")
                        .to_string()
                } else {
                    n.trim_end_matches(".git").to_string()
                }
            })
            .unwrap_or_else(|| "repository".into());

        let description = fs::read_to_string(common.join("description"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with("Unnamed repository"))
            .unwrap_or_else(|| "Self-hosted Git repository served by arks".into());

        let default_branch = head_branch_label(repo);
        let all_refs = repo.refs().list_refs("refs/").unwrap_or_default();
        let mut branches = Vec::new();
        let mut tags = Vec::new();
        for (rname, oid) in all_refs {
            if let Some(b) = rname.strip_prefix("refs/heads/") {
                branches.push((b.to_string(), oid));
            } else if let Some(t) = rname.strip_prefix("refs/tags/") {
                tags.push((t.to_string(), oid));
            }
        }
        let clone_url = format!("http://{}/", host.unwrap_or("127.0.0.1:8080"));
        let has_lfs = common.join("lfs").join("objects").is_dir();

        Self {
            name,
            description,
            default_branch,
            branches,
            tags,
            clone_url,
            has_lfs,
        }
    }
}

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

fn render_page(
    status: u16,
    ctx: &RepoContext,
    tab: ActiveTab,
    title: &str,
    main_html: &str,
) -> HttpResponse {
    let cls = |t: ActiveTab| if t == tab { "tab active" } else { "tab" };
    let doc = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n\
         <title>{}</title>\n\
         <link rel=\"icon\" type=\"image/svg+xml\" href=\"/favicon.svg\">\n\
         <link rel=\"stylesheet\" href=\"/style.css\">\n\
         <script src=\"/assets/htmx.min.js\" defer></script>\n\
         </head>\n<body hx-boost=\"true\">\n\
         <header class=\"topbar\"><div class=\"topbar-inner\">\n\
         <div class=\"repo-head\">\n\
           <div class=\"repo-title\"><img src=\"/favicon.svg\" width=\"26\" height=\"26\" alt=\"arks\">\
           <a href=\"/\">{}</a><span class=\"badge\">arks serve</span></div>\n\
           <div class=\"clone-pill\"><span>Clone</span><code>git clone {}</code></div>\n\
         </div>\n\
         <nav class=\"tabs\">\n\
           <a class=\"{}\" href=\"/\">Code</a>\n\
           <a class=\"{}\" href=\"/commits/{}\">Commits</a>\n\
           <a class=\"{}\" href=\"/branches\">Branches <span class=\"counter\">{}</span></a>\n\
           <a class=\"{}\" href=\"/tags\">Tags <span class=\"counter\">{}</span></a>\n\
         </nav>\n\
         </div></header>\n\
         <main>\n{main_html}\n</main>\n</body>\n</html>\n",
        html_escape(title),
        html_escape(&ctx.name),
        html_escape(&ctx.clone_url),
        cls(ActiveTab::Code),
        cls(ActiveTab::Commits),
        html_escape(&ctx.default_branch),
        cls(ActiveTab::Branches),
        ctx.branches.len(),
        cls(ActiveTab::Tags),
        ctx.tags.len(),
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
            ("Cache-Control".into(), "public, max-age=86400".into()),
        ],
    }
}

fn htmx_response() -> HttpResponse {
    HttpResponse {
        status: 200,
        content_type: "application/javascript; charset=utf-8".into(),
        body: HTMX_JS.as_bytes().to_vec(),
        www_authenticate: None,
        extra_headers: vec![
            ("X-Content-Type-Options".into(), "nosniff".into()),
            (
                "Cache-Control".into(),
                "public, max-age=31536000, immutable".into(),
            ),
        ],
    }
}

fn favicon_response() -> HttpResponse {
    HttpResponse {
        status: 200,
        content_type: "image/svg+xml".into(),
        body: FAVICON_SVG.as_bytes().to_vec(),
        www_authenticate: None,
        extra_headers: vec![
            ("X-Content-Type-Options".into(), "nosniff".into()),
            ("Cache-Control".into(), "public, max-age=86400".into()),
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
    if path == "/assets/htmx.min.js" {
        return Some(htmx_response());
    }
    if path == "/favicon.svg" {
        return Some(favicon_response());
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
        return Some(HttpResponse {
            status: 500,
            content_type: "text/plain; charset=utf-8".into(),
            body: b"Repository unavailable\n".to_vec(),
            www_authenticate: None,
            extra_headers: vec![("Content-Security-Policy".into(), WEB_UI_CSP.into())],
        });
    };
    let ctx = RepoContext::from_repo(&repo, req.host.as_deref());

    if path == "/" || path.is_empty() {
        return Some(render_overview(&repo, &ctx));
    }
    if path == "/branches" {
        return Some(render_branches(&repo, &ctx));
    }
    if path == "/tags" {
        return Some(render_tags(&repo, &ctx));
    }
    if path == "/refs" {
        return Some(render_refs(&repo, &ctx));
    }
    if path == "/commits" {
        return Some(render_commits(&repo, &ctx, &ctx.default_branch));
    }
    if let Some(rev) = path.strip_prefix("/commits/") {
        return Some(render_commits(&repo, &ctx, rev));
    }
    if let Some(oid_hex) = path.strip_prefix("/commit/") {
        return Some(render_commit_detail(&repo, &ctx, oid_hex));
    }
    if let Some(rest) = path.strip_prefix("/tree/") {
        let (rev, subpath) = match rest.split_once('/') {
            Some((r, p)) => (r, p),
            None => (rest, ""),
        };
        return Some(render_tree(&repo, &ctx, rev, subpath));
    }
    if let Some(rest) = path.strip_prefix("/blob/") {
        if let Some((rev, subpath)) = rest.split_once('/') {
            return Some(render_blob(&repo, &ctx, rev, subpath));
        }
        return Some(render_page(
            404,
            &ctx,
            ActiveTab::Code,
            "Not Found",
            "<h1>Blob path missing</h1>",
        ));
    }
    if let Some(rest) = path.strip_prefix("/raw/") {
        if let Some((rev, subpath)) = rest.split_once('/') {
            return Some(render_raw(&repo, rev, subpath));
        }
        return Some(HttpResponse {
            status: 404,
            content_type: "text/plain".into(),
            body: b"not found\n".to_vec(),
            www_authenticate: None,
            extra_headers: Vec::new(),
        });
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

fn render_ref_switcher(ctx: &RepoContext, current_rev: &str, target_prefix: &str) -> String {
    let mut out = format!(
        "<details class=\"ref-switcher\"><summary>{} &#9662;</summary><div class=\"ref-menu\">",
        html_escape(current_rev)
    );
    out.push_str("<div class=\"ref-menu-head\">Branches</div>");
    for (b, _) in &ctx.branches {
        out.push_str(&format!(
            "<a href=\"{}/{}\">{}</a>",
            target_prefix,
            html_escape(b),
            html_escape(b)
        ));
    }
    if !ctx.tags.is_empty() {
        out.push_str("<div class=\"ref-menu-head\">Tags</div>");
        for (t, _) in &ctx.tags {
            out.push_str(&format!(
                "<a href=\"{}/{}\">{}</a>",
                target_prefix,
                html_escape(t),
                html_escape(t)
            ));
        }
    }
    out.push_str("</div></details>");
    out
}

fn render_breadcrumbs(ctx: &RepoContext, rev: &str, subpath: &str) -> String {
    let mut out = format!(
        "<div class=\"breadcrumbs\"><a href=\"/tree/{}\"><strong>{}</strong></a>",
        html_escape(rev),
        html_escape(&ctx.name)
    );
    let parts: Vec<&str> = subpath.split('/').filter(|p| !p.is_empty()).collect();
    let mut acc = String::new();
    for (idx, part) in parts.iter().enumerate() {
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(part);
        out.push_str("<span class=\"sep\">/</span>");
        if idx + 1 == parts.len() {
            out.push_str(&format!("<strong>{}</strong>", html_escape(part)));
        } else {
            out.push_str(&format!(
                "<a href=\"/tree/{}/{}\">{}</a>",
                html_escape(rev),
                html_escape(&acc),
                html_escape(part)
            ));
        }
    }
    out.push_str("</div>");
    out
}

fn sorted_tree_entries(repo: &Repository, tree_oid: &Oid) -> Option<Vec<TreeEntry>> {
    let obj = repo.get_object(tree_oid).ok()?;
    let mut entries = Tree::decode(&obj.payload, repo.oid_kind()).ok()?.entries;
    // GitHub/GitLab standard: directories first, then files alphabetically.
    entries.sort_by(|a, b| {
        let a_dir = a.mode == TreeMode::Tree;
        let b_dir = b.mode == TreeMode::Tree;
        b_dir.cmp(&a_dir).then_with(|| a.name.cmp(&b.name))
    });
    Some(entries)
}

fn render_file_box(
    repo: &Repository,
    rev: &str,
    commit_oid: &Oid,
    commit: &Commit,
    tree_oid: &Oid,
    subpath: &str,
) -> String {
    let hex = commit_oid.to_hex();
    let short = &hex[..7.min(hex.len())];
    let subj = commit.message.lines().next().unwrap_or("");
    let mut out = format!(
        "<div class=\"box\"><div class=\"box-header\">\n\
           <div><strong>{}</strong> <span class=\"meta\">{}</span></div>\n\
           <div><a class=\"sha-pill\" href=\"/commit/{}\"><code>{}</code></a> \
           <a class=\"btn\" href=\"/commits/{}\">History</a></div>\n\
         </div>\n<table class=\"files\"><tbody>\n",
        html_escape(&commit.author.name),
        html_escape(subj),
        html_escape(&hex),
        html_escape(short),
        html_escape(rev),
    );

    if !subpath.is_empty() {
        let parent_href = match subpath.rsplit_once('/') {
            Some((parent, _)) => format!("/tree/{}/{}", html_escape(rev), html_escape(parent)),
            None => format!("/tree/{}", html_escape(rev)),
        };
        out.push_str(&format!(
            "<tr><td colspan=\"2\"><a href=\"{parent_href}\">..</a></td></tr>\n"
        ));
    }

    if let Some(entries) = sorted_tree_entries(repo, tree_oid) {
        for entry in entries {
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
            let icon = if is_dir { ICON_DIR } else { ICON_FILE };
            let eshort = &entry.oid.to_hex()[..7.min(entry.oid.to_hex().len())];
            out.push_str(&format!(
                "<tr><td><span class=\"entry-name\">{icon}<a href=\"{href}\">{}</a></span></td>\
                 <td class=\"meta\" style=\"text-align:right\"><code>{}</code></td></tr>\n",
                html_escape(&entry.name),
                html_escape(eshort)
            ));
        }
    }
    out.push_str("</tbody></table></div>\n");
    out
}

/// Safe, zero-JS server-side Markdown subset renderer for README files (strictly HTML-escaped).
fn render_safe_markdown(md: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    let mut in_list = false;
    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            if in_code {
                out.push_str("</code></pre>\n");
                in_code = false;
            } else {
                out.push_str("<pre><code>");
                in_code = true;
            }
            continue;
        }
        if in_code {
            out.push_str(&html_escape(line));
            out.push('\n');
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            out.push_str(&format!("<h3>{}</h3>\n", format_inline_code(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            out.push_str(&format!("<h2>{}</h2>\n", format_inline_code(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            out.push_str(&format!("<h1>{}</h1>\n", format_inline_code(rest)));
        } else if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            if !in_list {
                out.push_str("<ul>\n");
                in_list = true;
            }
            out.push_str(&format!("<li>{}</li>\n", format_inline_code(rest)));
        } else {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            out.push_str(&format!("<p>{}</p>\n", format_inline_code(trimmed)));
        }
    }
    if in_code {
        out.push_str("</code></pre>\n");
    }
    if in_list {
        out.push_str("</ul>\n");
    }
    out
}

fn format_inline_code(s: &str) -> String {
    let mut out = String::new();
    let mut in_tick = false;
    for part in s.split('`') {
        if in_tick {
            out.push_str("<code>");
            out.push_str(&html_escape(part));
            out.push_str("</code>");
        } else {
            out.push_str(&html_escape(part));
        }
        in_tick = !in_tick;
    }
    out
}

fn render_readme_box(repo: &Repository, tree_oid: &Oid) -> String {
    let Some(entries) = sorted_tree_entries(repo, tree_oid) else {
        return String::new();
    };
    let Some(readme) = entries.into_iter().find(|e| {
        e.mode != TreeMode::Tree
            && matches!(
                e.name.to_ascii_lowercase().as_str(),
                "readme.md" | "readme.markdown" | "readme.txt" | "readme"
            )
    }) else {
        return String::new();
    };
    let Ok(obj) = repo.get_object(&readme.oid) else {
        return String::new();
    };
    if obj.payload.contains(&0) {
        return String::new();
    }
    let text = String::from_utf8_lossy(&obj.payload);
    let body = render_safe_markdown(&text);
    format!(
        "<div class=\"box\"><div class=\"box-header\"><strong>{}</strong></div>\
         <div class=\"readme\">{body}</div></div>\n",
        html_escape(&readme.name)
    )
}

fn render_overview(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let mut left = format!(
        "<div class=\"toolbar\"><div class=\"toolbar-left\">{}</div></div>\n",
        render_ref_switcher(ctx, &ctx.default_branch, "/tree")
    );
    if let Some(head_oid) = resolve_rev_simple(repo, "HEAD") {
        if let Some(c) = load_commit(repo, &head_oid) {
            left.push_str(&render_file_box(
                repo,
                &ctx.default_branch,
                &head_oid,
                &c,
                &c.tree,
                "",
            ));
            left.push_str(&render_readme_box(repo, &c.tree));
        }
    } else {
        left.push_str("<div class=\"box\"><div class=\"box-body\"><p class=\"meta\">Empty repository (no commits yet).</p></div></div>");
    }

    let sidebar = format!(
        "<aside>\n\
           <div class=\"sidebar-section\">\n\
             <h3>About</h3>\n\
             <p class=\"meta\">{}</p>\n\
             <div class=\"stat-row\"><span>Default branch</span><strong>{}</strong></div>\n\
             <div class=\"stat-row\"><span>Branches</span><strong>{}</strong></div>\n\
             <div class=\"stat-row\"><span>Tags</span><strong>{}</strong></div>\n\
             <div class=\"stat-row\"><span>Git LFS</span><strong>{}</strong></div>\n\
           </div>\n\
         </aside>",
        html_escape(&ctx.description),
        html_escape(&ctx.default_branch),
        ctx.branches.len(),
        ctx.tags.len(),
        if ctx.has_lfs { "active" } else { "enabled" },
    );

    let html = format!("<div class=\"repo-grid\"><div>{left}</div>{sidebar}</div>");
    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!("{} — arks", ctx.name),
        &html,
    )
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

fn render_tree(repo: &Repository, ctx: &RepoContext, rev: &str, subpath: &str) -> HttpResponse {
    let Some(commit_oid) = resolve_rev_simple(repo, rev) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not Found",
            "<h1>Revision not found</h1>",
        );
    };
    let Some(c) = load_commit(repo, &commit_oid) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not Found",
            "<h1>Commit not found</h1>",
        );
    };
    let Some((mode, oid)) = find_tree_entry(repo, &c.tree, subpath) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not Found",
            "<h1>Path not found</h1>",
        );
    };
    if mode != TreeMode::Tree {
        return render_blob(repo, ctx, rev, subpath);
    }
    let mut html = format!(
        "<div class=\"toolbar\"><div class=\"toolbar-left\">{}{}</div></div>\n",
        render_ref_switcher(ctx, rev, "/tree"),
        render_breadcrumbs(ctx, rev, subpath)
    );
    html.push_str(&render_file_box(repo, rev, &commit_oid, &c, &oid, subpath));
    html.push_str(&render_readme_box(repo, &oid));
    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!(
            "{} — {}",
            ctx.name,
            if subpath.is_empty() { rev } else { subpath }
        ),
        &html,
    )
}

fn render_blob(repo: &Repository, ctx: &RepoContext, rev: &str, subpath: &str) -> HttpResponse {
    let Some(commit_oid) = resolve_rev_simple(repo, rev) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not Found",
            "<h1>Revision not found</h1>",
        );
    };
    let Some(c) = load_commit(repo, &commit_oid) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not Found",
            "<h1>Commit not found</h1>",
        );
    };
    let Some((mode, oid)) = find_tree_entry(repo, &c.tree, subpath) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not Found",
            "<h1>File not found</h1>",
        );
    };
    if mode == TreeMode::Tree {
        return render_tree(repo, ctx, rev, subpath);
    }
    let Ok(obj) = repo.get_object(&oid) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not Found",
            "<h1>Blob missing</h1>",
        );
    };

    let raw_href = format!("/raw/{}/{}", html_escape(rev), html_escape(subpath));
    let mut html = format!(
        "<div class=\"toolbar\"><div class=\"toolbar-left\">{}{}</div></div>\n",
        render_ref_switcher(ctx, rev, "/tree"),
        render_breadcrumbs(ctx, rev, subpath)
    );

    if obj.payload.contains(&0) {
        html.push_str(&format!(
            "<div class=\"box\"><div class=\"box-header\">\
               <span class=\"meta\">Binary file &middot; {} bytes</span>\
               <a class=\"btn\" href=\"{raw_href}\">Raw</a>\
             </div><div class=\"box-body\"><p class=\"meta\">Binary file not displayed.</p></div></div>",
            obj.payload.len()
        ));
    } else {
        let text = String::from_utf8_lossy(&obj.payload);
        let lines: Vec<&str> = text.lines().collect();
        html.push_str(&format!(
            "<div class=\"box\"><div class=\"box-header\">\
               <span class=\"meta\">{} lines &middot; {} bytes</span>\
               <a class=\"btn\" href=\"{raw_href}\">Raw</a>\
             </div><div style=\"overflow-x:auto\"><table class=\"code-table\"><tbody>\n",
            lines.len(),
            obj.payload.len()
        ));
        for (idx, line) in lines.iter().enumerate() {
            let n = idx + 1;
            html.push_str(&format!(
                "<tr id=\"L{n}\"><td class=\"ln\"><a href=\"#L{n}\">{n}</a></td><td>{}</td></tr>\n",
                html_escape(line)
            ));
        }
        html.push_str("</tbody></table></div></div>\n");
    }

    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!("{subpath} — {}", ctx.name),
        &html,
    )
}

fn render_raw(repo: &Repository, rev: &str, subpath: &str) -> HttpResponse {
    let Some(commit_oid) = resolve_rev_simple(repo, rev) else {
        return HttpResponse {
            status: 404,
            content_type: "text/plain".into(),
            body: b"not found\n".to_vec(),
            www_authenticate: None,
            extra_headers: Vec::new(),
        };
    };
    let Some(c) = load_commit(repo, &commit_oid) else {
        return HttpResponse {
            status: 404,
            content_type: "text/plain".into(),
            body: b"not found\n".to_vec(),
            www_authenticate: None,
            extra_headers: Vec::new(),
        };
    };
    let Some((mode, oid)) = find_tree_entry(repo, &c.tree, subpath) else {
        return HttpResponse {
            status: 404,
            content_type: "text/plain".into(),
            body: b"not found\n".to_vec(),
            www_authenticate: None,
            extra_headers: Vec::new(),
        };
    };
    if mode == TreeMode::Tree {
        return HttpResponse {
            status: 400,
            content_type: "text/plain".into(),
            body: b"path is a directory\n".to_vec(),
            www_authenticate: None,
            extra_headers: Vec::new(),
        };
    }
    let Ok(obj) = repo.get_object(&oid) else {
        return HttpResponse {
            status: 404,
            content_type: "text/plain".into(),
            body: b"not found\n".to_vec(),
            www_authenticate: None,
            extra_headers: Vec::new(),
        };
    };
    let ct = if obj.payload.contains(&0) {
        "application/octet-stream"
    } else {
        "text/plain; charset=utf-8"
    };
    HttpResponse {
        status: 200,
        content_type: ct.into(),
        body: obj.payload,
        www_authenticate: None,
        extra_headers: vec![("X-Content-Type-Options".into(), "nosniff".into())],
    }
}

fn render_commits(repo: &Repository, ctx: &RepoContext, rev: &str) -> HttpResponse {
    let Some(mut cur) = resolve_rev_simple(repo, rev) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Commits,
            "Not Found",
            "<h1>Revision not found</h1>",
        );
    };
    let mut html = format!(
        "<div class=\"toolbar\"><div class=\"toolbar-left\">{}</div></div>\n\
         <div class=\"box\"><div class=\"box-header\"><strong>Commits on {}</strong></div>\n\
         <table class=\"files\"><tbody>\n",
        render_ref_switcher(ctx, rev, "/commits"),
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
            "<tr><td><a href=\"/commit/{}\"><strong>{}</strong></a>\
             <div class=\"meta\">{}</div></td>\
             <td style=\"text-align:right\"><a class=\"sha-pill\" href=\"/commit/{}\"><code>{}</code></a> \
             <a class=\"btn\" href=\"/tree/{}\">Browse</a></td></tr>\n",
            html_escape(&hex),
            html_escape(subj),
            html_escape(&c.author.name),
            html_escape(&hex),
            html_escape(short),
            html_escape(&hex),
        ));
        let Some(&parent) = c.parents.first() else {
            break;
        };
        cur = parent;
    }
    html.push_str("</tbody></table></div>\n");
    render_page(
        200,
        ctx,
        ActiveTab::Commits,
        &format!("Commits ({rev}) — {}", ctx.name),
        &html,
    )
}

fn render_commit_detail(repo: &Repository, ctx: &RepoContext, oid_hex: &str) -> HttpResponse {
    let Some(oid) = resolve_rev_simple(repo, oid_hex) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Commits,
            "Not Found",
            "<h1>Commit not found</h1>",
        );
    };
    let Some(c) = load_commit(repo, &oid) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Commits,
            "Not Found",
            "<h1>Commit not found</h1>",
        );
    };
    let hex = oid.to_hex();
    let parents_html: String = if c.parents.is_empty() {
        "none (root commit)".into()
    } else {
        c.parents
            .iter()
            .map(|p| {
                let ph = p.to_hex();
                let ps = &ph[..7.min(ph.len())];
                format!(
                    "<a class=\"sha-pill\" href=\"/commit/{}\"><code>{}</code></a>",
                    html_escape(&ph),
                    html_escape(ps)
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    let mut html = format!(
        "<div class=\"box\"><div class=\"box-header\">\
           <div><strong>{}</strong></div>\
           <div><a class=\"btn\" href=\"/tree/{}\">Browse files</a></div>\
         </div><div class=\"box-body\">\
           <pre style=\"margin:0 0 12px;white-space:pre-wrap\">{}</pre>\
           <div class=\"meta\">Author: <strong>{}</strong> &lt;{}&gt; &middot; Commit <code>{}</code> &middot; Parent(s): {parents_html}</div>\
         </div></div>\n",
        html_escape(c.message.lines().next().unwrap_or("")),
        html_escape(&hex),
        html_escape(&c.message),
        html_escape(&c.author.name),
        html_escape(&c.author.email),
        html_escape(&hex),
    );

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

    let mut changed: Vec<(&str, &str, Option<Oid>, Option<Oid>)> = Vec::new();
    for (p, &o) in &cur_files {
        match parent_files.get(p) {
            None => changed.push(("A", p, None, Some(o))),
            Some(&po) if po != o => changed.push(("M", p, Some(po), Some(o))),
            _ => {}
        }
    }
    for (p, &po) in &parent_files {
        if !cur_files.contains_key(p) {
            changed.push(("D", p, Some(po), None));
        }
    }

    html.push_str(&format!(
        "<p class=\"meta\">Showing <strong>{}</strong> changed file(s)</p>\n",
        changed.len()
    ));

    for (status, path, old_oid, new_oid) in changed {
        let badge_cls = match status {
            "A" => "badge badge-add",
            "D" => "badge badge-del",
            _ => "badge badge-accent",
        };
        let view_link = if new_oid.is_some() {
            format!(
                "<a class=\"btn\" href=\"/blob/{}/{}\">View file</a>",
                html_escape(&hex),
                html_escape(path)
            )
        } else {
            String::new()
        };
        html.push_str(&format!(
            "<div class=\"box\"><div class=\"box-header\">\
               <div><span class=\"{badge_cls}\">{status}</span> <code>{}</code></div>\
               <div>{view_link}</div>\
             </div><div style=\"overflow-x:auto\"><table class=\"code-table\"><tbody>\n",
            html_escape(path)
        ));
        html.push_str(&render_simple_diff(repo, old_oid, new_oid));
        html.push_str("</tbody></table></div></div>\n");
    }

    render_page(
        200,
        ctx,
        ActiveTab::Commits,
        &format!("Commit {} — {}", &hex[..7.min(hex.len())], ctx.name),
        &html,
    )
}

fn render_simple_diff(repo: &Repository, old_oid: Option<Oid>, new_oid: Option<Oid>) -> String {
    let old_bytes = old_oid
        .and_then(|o| repo.get_object(&o).ok())
        .map(|o| o.payload)
        .unwrap_or_default();
    let new_bytes = new_oid
        .and_then(|o| repo.get_object(&o).ok())
        .map(|o| o.payload)
        .unwrap_or_default();
    if old_bytes.contains(&0) || new_bytes.contains(&0) {
        return "<tr><td class=\"meta\" style=\"padding:10px 16px\">Binary files differ</td></tr>\n"
            .into();
    }
    let old_text = String::from_utf8_lossy(&old_bytes);
    let new_text = String::from_utf8_lossy(&new_bytes);
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();

    // Trim common prefix and suffix for compact, clean hunks.
    let mut prefix = 0usize;
    while prefix < old_lines.len()
        && prefix < new_lines.len()
        && old_lines[prefix] == new_lines[prefix]
    {
        prefix += 1;
    }
    let mut old_end = old_lines.len();
    let mut new_end = new_lines.len();
    while old_end > prefix && new_end > prefix && old_lines[old_end - 1] == new_lines[new_end - 1] {
        old_end -= 1;
        new_end -= 1;
    }

    let mut out = format!(
        "<tr class=\"diff-hunk\"><td class=\"ln\"></td><td>@@ -{},{} +{},{} @@</td></tr>\n",
        prefix + 1,
        old_end.saturating_sub(prefix),
        prefix + 1,
        new_end.saturating_sub(prefix)
    );
    let ctx_start = prefix.saturating_sub(3);
    for (idx, line) in old_lines[ctx_start..prefix].iter().enumerate() {
        out.push_str(&format!(
            "<tr><td class=\"ln\">{}</td><td> {}</td></tr>\n",
            ctx_start + idx + 1,
            html_escape(line)
        ));
    }
    for (idx, line) in old_lines[prefix..old_end].iter().enumerate() {
        out.push_str(&format!(
            "<tr class=\"diff-del\"><td class=\"ln\">{}</td><td><span class=\"diff-sig\">-</span>{}</td></tr>\n",
            prefix + idx + 1,
            html_escape(line)
        ));
    }
    for (idx, line) in new_lines[prefix..new_end].iter().enumerate() {
        out.push_str(&format!(
            "<tr class=\"diff-add\"><td class=\"ln\">{}</td><td><span class=\"diff-sig\">+</span>{}</td></tr>\n",
            prefix + idx + 1,
            html_escape(line)
        ));
    }
    let ctx_end = (new_end + 3).min(new_lines.len());
    for (idx, line) in new_lines[new_end..ctx_end].iter().enumerate() {
        out.push_str(&format!(
            "<tr><td class=\"ln\">{}</td><td> {}</td></tr>\n",
            new_end + idx + 1,
            html_escape(line)
        ));
    }
    out
}

fn render_branches(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let mut html = String::from(
        "<div class=\"box\"><div class=\"box-header\"><strong>Branches</strong></div>\n\
         <table class=\"files\"><tbody>\n",
    );
    for (b, oid) in &ctx.branches {
        let hex = oid.to_hex();
        let short = &hex[..7.min(hex.len())];
        let subj = load_commit(repo, oid)
            .and_then(|c| c.message.lines().next().map(String::from))
            .unwrap_or_default();
        let def_badge = if *b == ctx.default_branch {
            " <span class=\"badge badge-accent\">default</span>"
        } else {
            ""
        };
        html.push_str(&format!(
            "<tr><td><a href=\"/tree/{}\"><strong>{}</strong></a>{def_badge}\
             <div class=\"meta\">{}</div></td>\
             <td style=\"text-align:right\"><a class=\"sha-pill\" href=\"/commit/{}\"><code>{}</code></a></td></tr>\n",
            html_escape(b),
            html_escape(b),
            html_escape(&subj),
            html_escape(&hex),
            html_escape(short),
        ));
    }
    html.push_str("</tbody></table></div>\n");
    render_page(
        200,
        ctx,
        ActiveTab::Branches,
        &format!("Branches — {}", ctx.name),
        &html,
    )
}

fn render_tags(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let mut html = String::from(
        "<div class=\"box\"><div class=\"box-header\"><strong>Tags</strong></div>\n\
         <table class=\"files\"><tbody>\n",
    );
    if ctx.tags.is_empty() {
        html.push_str("<tr><td class=\"meta\">No tags yet.</td></tr>\n");
    }
    for (t, oid) in &ctx.tags {
        let hex = oid.to_hex();
        let short = &hex[..7.min(hex.len())];
        let subj = load_commit(repo, oid)
            .and_then(|c| c.message.lines().next().map(String::from))
            .unwrap_or_default();
        html.push_str(&format!(
            "<tr><td><a href=\"/tree/{}\"><strong>{}</strong></a>\
             <div class=\"meta\">{}</div></td>\
             <td style=\"text-align:right\"><a class=\"sha-pill\" href=\"/commit/{}\"><code>{}</code></a></td></tr>\n",
            html_escape(t),
            html_escape(t),
            html_escape(&subj),
            html_escape(&hex),
            html_escape(short),
        ));
    }
    html.push_str("</tbody></table></div>\n");
    render_page(
        200,
        ctx,
        ActiveTab::Tags,
        &format!("Tags — {}", ctx.name),
        &html,
    )
}

fn render_refs(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let refs = repo.refs().list_refs("refs/").unwrap_or_default();
    let mut html = String::from(
        "<div class=\"box\"><div class=\"box-header\"><strong>All References</strong></div>\n\
         <table class=\"files\"><tbody>\n",
    );
    for (name, oid) in refs {
        let hex = oid.to_hex();
        let short = &hex[..7.min(hex.len())];
        let link_rev = name
            .strip_prefix("refs/heads/")
            .or_else(|| name.strip_prefix("refs/tags/"))
            .unwrap_or(&name);
        html.push_str(&format!(
            "<tr><td><a href=\"/tree/{}\">{}</a></td>\
             <td style=\"text-align:right\"><a class=\"sha-pill\" href=\"/commit/{}\"><code>{}</code></a></td></tr>\n",
            html_escape(link_rev),
            html_escape(&name),
            html_escape(&hex),
            html_escape(short)
        ));
    }
    html.push_str("</tbody></table></div>\n");
    render_page(
        200,
        ctx,
        ActiveTab::Branches,
        &format!("References — {}", ctx.name),
        &html,
    )
}
