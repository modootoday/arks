//! Embedded single-binary SSR Forge Web UI for `arks serve` (E8 — GitHub/GitLab common interface).
//!
//! Compile-time embedded assets (`include_str!`), strict HTML entity escaping on every dynamic
//! value, and `Content-Security-Policy: default-src 'self'; script-src 'self'`.
//! Never interferes with Git smart/dumb HTTP (`info/refs`, `git-upload-pack`,
//! `git-receive-pack`, `objects/`, `HEAD`) or Git LFS (`info/lfs/`).
//!
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.8 (E8).

mod code;
mod diff;
mod forge;
mod highlight;

use crate::http::{HttpRequest, HttpResponse};
use crate::serve::PageHandler;
use arks_object::{Commit, ObjectKind};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub const WEB_UI_CSP: &str = "default-src 'self'; script-src 'self'";
const STYLESHEET: &str = include_str!("../web_ui.css");
const HTMX_JS: &str = include_str!("../assets/htmx.min.js");
const DIFF2HTML_JS: &str = include_str!("../assets/diff2html.min.js");
const DIFF2HTML_CSS: &str = include_str!("../assets/diff2html.min.css");
const APP_JS: &str = include_str!("../assets/app.js");
const FAVICON_SVG: &str = include_str!("../assets/favicon.svg");

pub(crate) const ICON_DIR: &str = "<svg class=\"icon icon-dir\" viewBox=\"0 0 16 16\" aria-hidden=\"true\"><path d=\"M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5a.25.25 0 0 1-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75Z\"/></svg>";
pub(crate) const ICON_FILE: &str = "<svg class=\"icon icon-file\" viewBox=\"0 0 16 16\" aria-hidden=\"true\"><path d=\"M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25Zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h9.5a.25.25 0 0 0 .25-.25V6h-2.75A1.75 1.75 0 0 1 9 4.25V1.5Zm6.75.062V4.25c0 .138.112.25.25.25h2.688l-.011-.013-2.914-2.914-.013-.011Z\"/></svg>";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveTab {
    Code,
    Issues,
    Pulls,
    Actions,
    Projects,
    Wiki,
    Security,
    Insights,
    Settings,
}

pub(crate) struct RepoContext {
    pub name: String,
    pub description: String,
    pub default_branch: String,
    pub branches: Vec<(String, Oid)>,
    pub tags: Vec<(String, Oid)>,
    pub clone_url: String,
    pub has_lfs: bool,
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

pub(crate) fn render_partial(status: u16, fragment: &str) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "text/html; charset=utf-8".into(),
        body: fragment.as_bytes().to_vec(),
        www_authenticate: None,
        extra_headers: vec![
            ("Content-Security-Policy".into(), WEB_UI_CSP.into()),
            ("X-Content-Type-Options".into(), "nosniff".into()),
        ],
    }
}

pub(crate) fn render_page(
    status: u16,
    ctx: &RepoContext,
    tab: ActiveTab,
    title: &str,
    main_html: &str,
) -> HttpResponse {
    let cls = |t: ActiveTab| if t == tab { "tab active" } else { "tab" };
    let open_prs = ctx
        .branches
        .iter()
        .filter(|(b, _)| b != &ctx.default_branch)
        .count();
    let arks_clone = format!("arks clone {}", ctx.clone_url);
    let git_clone = format!("git clone {}", ctx.clone_url);

    let doc = format!(
        "<!doctype html>\n<html lang=\"en\" data-theme=\"dark\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n\
         <title>{}</title>\n\
         <link rel=\"icon\" type=\"image/svg+xml\" href=\"/favicon.svg\">\n\
         <link rel=\"stylesheet\" href=\"/assets/diff2html.min.css\">\n\
         <link rel=\"stylesheet\" href=\"/style.css\">\n\
         <script src=\"/assets/htmx.min.js\" defer></script>\n\
         <script src=\"/assets/diff2html.min.js\" defer></script>\n\
         <script src=\"/assets/app.js\" defer></script>\n\
         </head>\n<body hx-boost=\"true\">\n\
         <header class=\"topbar\"><div class=\"topbar-inner\">\n\
         <div class=\"repo-head\">\n\
           <div class=\"repo-title\"><img src=\"/favicon.svg\" width=\"26\" height=\"26\" alt=\"arks\">\
           <a href=\"/\">{}</a><span class=\"badge\">arks serve</span>\
           <span class=\"badge badge-green\">HTMX v2</span></div>\n\
           <div class=\"head-actions\">\n\
             <button type=\"button\" class=\"btn\" data-action=\"open-cmd\">Search / ⌘K</button>\n\
             <details class=\"popover\">\n\
               <summary class=\"btn btn-primary\">&lt;&gt; Code ▾</summary>\n\
               <div class=\"popover-panel right\">\n\
                 <div class=\"popover-head\">Clone repository</div>\n\
                 <div style=\"padding:10px 12px;display:flex;flex-direction:column;gap:8px\">\n\
                   <div class=\"btn-group\">\n\
                     <button type=\"button\" class=\"btn active\" data-clone-cmd=\"{}\">arks</button>\n\
                     <button type=\"button\" class=\"btn\" data-clone-cmd=\"{}\">git</button>\n\
                   </div>\n\
                   <div style=\"display:flex;gap:6px;align-items:center\">\n\
                     <code id=\"clone-cmd-text\" class=\"input\" style=\"flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap\">{}</code>\n\
                     <button type=\"button\" class=\"btn\" data-copy-target=\"clone-cmd-text\">Copy</button>\n\
                   </div>\n\
                 </div>\n\
               </div>\n\
             </details>\n\
             <button type=\"button\" class=\"btn\" data-action=\"toggle-theme\">Theme</button>\n\
           </div>\n\
         </div>\n\
         <nav class=\"tabs\">\n\
           <a class=\"{}\" href=\"/\">Code</a>\n\
           <a class=\"{}\" href=\"/issues\">Issues</a>\n\
           <a class=\"{}\" href=\"/pulls\">Pull Requests <span class=\"counter\">{}</span></a>\n\
           <a class=\"{}\" href=\"/actions\">Actions</a>\n\
           <a class=\"{}\" href=\"/projects\">Projects</a>\n\
           <a class=\"{}\" href=\"/wiki\">Wiki</a>\n\
           <a class=\"{}\" href=\"/security\">Security</a>\n\
           <a class=\"{}\" href=\"/insights\">Insights</a>\n\
           <a class=\"{}\" href=\"/settings\">Settings</a>\n\
         </nav>\n\
         </div></header>\n\
         <main>\n{main_html}\n</main>\n\
         <div id=\"cmd-modal\" class=\"cmd-backdrop\">\n\
           <div class=\"cmd-dialog\">\n\
             <div class=\"popover-head\" style=\"display:flex;justify-content:space-between\">\n\
               <span>Quick Jump (⌘K)</span>\n\
               <button type=\"button\" class=\"btn\" data-action=\"close-cmd\">ESC</button>\n\
             </div>\n\
             <a class=\"cmd-item\" href=\"/\"><span>Code &amp; File Tree</span><span class=\"muted\">/</span></a>\n\
             <a class=\"cmd-item\" href=\"/commits/{}\"><span>Commits History</span><span class=\"muted\">/commits</span></a>\n\
             <a class=\"cmd-item\" href=\"/branches\"><span>Branches</span><span class=\"muted\">{}</span></a>\n\
             <a class=\"cmd-item\" href=\"/tags\"><span>Tags &amp; Releases</span><span class=\"muted\">{}</span></a>\n\
             <a class=\"cmd-item\" href=\"/compare\"><span>Compare Branches</span><span class=\"muted\">/compare</span></a>\n\
             <a class=\"cmd-item\" href=\"/actions\"><span>Actions &amp; Workflows</span><span class=\"muted\">/actions</span></a>\n\
             <a class=\"cmd-item\" href=\"/security\"><span>Security &amp; Signatures</span><span class=\"muted\">/security</span></a>\n\
             <a class=\"cmd-item\" href=\"/insights\"><span>Insights &amp; Contributors</span><span class=\"muted\">/insights</span></a>\n\
             <a class=\"cmd-item\" href=\"/settings\"><span>Repository Settings</span><span class=\"muted\">/settings</span></a>\n\
           </div>\n\
         </div>\n\
         </body>\n</html>\n",
        html_escape(title),
        html_escape(&ctx.name),
        html_escape(&arks_clone),
        html_escape(&git_clone),
        html_escape(&arks_clone),
        cls(ActiveTab::Code),
        cls(ActiveTab::Issues),
        cls(ActiveTab::Pulls),
        open_prs,
        cls(ActiveTab::Actions),
        cls(ActiveTab::Projects),
        cls(ActiveTab::Wiki),
        cls(ActiveTab::Security),
        cls(ActiveTab::Insights),
        cls(ActiveTab::Settings),
        html_escape(&ctx.default_branch),
        ctx.branches.len(),
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

fn static_asset(content_type: &str, body: &[u8], immutable: bool) -> HttpResponse {
    HttpResponse {
        status: 200,
        content_type: content_type.into(),
        body: body.to_vec(),
        www_authenticate: None,
        extra_headers: vec![
            ("X-Content-Type-Options".into(), "nosniff".into()),
            (
                "Cache-Control".into(),
                if immutable {
                    "public, max-age=31536000, immutable"
                } else {
                    "public, max-age=86400"
                }
                .into(),
            ),
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
    match path {
        "/style.css" => {
            return Some(static_asset(
                "text/css; charset=utf-8",
                STYLESHEET.as_bytes(),
                false,
            ));
        }
        "/assets/htmx.min.js" => {
            return Some(static_asset(
                "application/javascript; charset=utf-8",
                HTMX_JS.as_bytes(),
                true,
            ));
        }
        "/assets/diff2html.min.js" => {
            return Some(static_asset(
                "application/javascript; charset=utf-8",
                DIFF2HTML_JS.as_bytes(),
                true,
            ));
        }
        "/assets/diff2html.min.css" => {
            return Some(static_asset(
                "text/css; charset=utf-8",
                DIFF2HTML_CSS.as_bytes(),
                true,
            ));
        }
        "/assets/app.js" => {
            return Some(static_asset(
                "application/javascript; charset=utf-8",
                APP_JS.as_bytes(),
                false,
            ));
        }
        "/favicon.svg" => {
            return Some(static_asset("image/svg+xml", FAVICON_SVG.as_bytes(), false));
        }
        _ => {}
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
        return Some(render_partial(500, "Repository unavailable\n"));
    };
    let ctx = RepoContext::from_repo(&repo, req.host.as_deref());
    let query = req.query.as_str();

    if path == "/" || path.is_empty() {
        return Some(code::render_tree(&repo, &ctx, &ctx.default_branch, ""));
    }
    if path == "/search" {
        return Some(code::render_search_partial(&repo, &ctx, query));
    }
    if path == "/branches" || path == "/refs" {
        return Some(code::render_branches(&ctx));
    }
    if path == "/tags" {
        return Some(code::render_tags(&ctx));
    }
    if path == "/commits" {
        return Some(code::render_commits(
            &repo,
            &ctx,
            &ctx.default_branch,
            query,
        ));
    }
    if let Some(rev) = path.strip_prefix("/commits/") {
        return Some(code::render_commits(&repo, &ctx, rev, query));
    }
    if let Some(oid_str) = path.strip_prefix("/commit/") {
        return Some(diff::render_commit(&repo, &ctx, oid_str));
    }
    if path == "/compare" {
        return Some(diff::render_compare(&repo, &ctx, ""));
    }
    if let Some(spec) = path.strip_prefix("/compare/") {
        return Some(diff::render_compare(&repo, &ctx, spec));
    }
    if let Some(rest) = path.strip_prefix("/tree/") {
        let (rev, subpath) = split_rev_and_path(rest);
        return Some(code::render_tree(&repo, &ctx, rev, subpath));
    }
    if let Some(rest) = path.strip_prefix("/blob/") {
        let (rev, subpath) = split_rev_and_path(rest);
        return Some(code::render_blob(&repo, &ctx, rev, subpath));
    }
    if let Some(rest) = path.strip_prefix("/blame/") {
        let (rev, subpath) = split_rev_and_path(rest);
        return Some(code::render_blame(&repo, &ctx, rev, subpath));
    }
    if let Some(rest) = path.strip_prefix("/raw/") {
        let (rev, subpath) = split_rev_and_path(rest);
        return Some(code::render_raw(&repo, rev, subpath));
    }
    if path == "/issues" {
        return Some(forge::render_issues(&repo, &ctx));
    }
    if path == "/pulls" {
        return Some(forge::render_pulls(&repo, &ctx));
    }
    if path == "/actions" {
        return Some(forge::render_actions(&repo, &ctx));
    }
    if path == "/projects" {
        return Some(forge::render_projects(&repo, &ctx));
    }
    if path == "/wiki" {
        return Some(forge::render_wiki(&repo, &ctx));
    }
    if path == "/security" {
        return Some(forge::render_security(&repo, &ctx));
    }
    if path == "/insights" {
        return Some(forge::render_insights(&repo, &ctx));
    }
    if path == "/settings" {
        return Some(forge::render_settings(&repo, &ctx));
    }

    Some(render_page(
        404,
        &ctx,
        ActiveTab::Code,
        "404 Not Found",
        "<div class=\"card\"><div class=\"card-body\">Page not found.</div></div>",
    ))
}

fn split_rev_and_path(rest: &str) -> (&str, &str) {
    match rest.split_once('/') {
        Some((r, p)) => (r, p),
        None => (rest, ""),
    }
}

pub(crate) fn head_branch_label(repo: &Repository) -> String {
    match repo.head_target() {
        Ok(RefTarget::Symbolic(s)) => s.strip_prefix("refs/heads/").unwrap_or(&s).to_string(),
        Ok(RefTarget::Direct(o)) => o.to_hex()[..7.min(o.to_hex().len())].to_string(),
        Err(_) => "HEAD".into(),
    }
}

pub(crate) fn resolve_rev_oid(repo: &Repository, rev: &str) -> Option<Oid> {
    if rev == "HEAD" {
        return repo.refs().resolve("HEAD").ok();
    }
    if let Ok(oid) = Oid::from_hex(rev) {
        if repo.get_object(&oid).is_ok() {
            return Some(oid);
        }
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

pub(crate) fn load_commit(repo: &Repository, oid: &Oid) -> Option<Commit> {
    let obj = repo.get_object(oid).ok()?;
    if obj.kind != ObjectKind::Commit {
        return None;
    }
    Commit::decode(&obj.payload).ok()
}

pub(crate) fn load_tree(repo: &Repository, oid: &Oid) -> Option<arks_object::Tree> {
    let obj = repo.get_object(oid).ok()?;
    if obj.kind != ObjectKind::Tree {
        return None;
    }
    arks_object::Tree::decode(&obj.payload, repo.oid_kind()).ok()
}

pub(crate) fn load_blob(repo: &Repository, oid: &Oid) -> Option<Vec<u8>> {
    let obj = repo.get_object(oid).ok()?;
    if obj.kind != ObjectKind::Blob {
        return None;
    }
    Some(obj.payload)
}

pub(crate) fn commit_has_sig(commit: &Commit) -> bool {
    commit.gpgsig.is_some()
}

pub(crate) fn collect_recent_commits(
    repo: &Repository,
    start: Oid,
    limit: usize,
) -> Vec<(Oid, Commit)> {
    let mut out = Vec::new();
    let mut visited = HashSet::new();
    let mut cur = Some(start);
    while let Some(oid) = cur {
        if out.len() >= limit || !visited.insert(oid) {
            break;
        }
        let Some(commit) = load_commit(repo, &oid) else {
            break;
        };
        let next = commit.parents.first().copied();
        out.push((oid, commit));
        cur = next;
    }
    out
}

pub(crate) fn format_timestamp(ts: i64) -> String {
    format!("unix:{ts}")
}
