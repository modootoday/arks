//! The repository browser pages `arks instaweb` serves beside git HTTP, in
//! place of stock gitweb (a Perl CGI under an external httpd).
//!
//! Primary: <https://git-scm.com/docs/gitweb> (ACTIONS, AND URLS: the
//! actions, the path_info form and the per-repository description, owner
//! and clone URL sources) and <https://git-scm.com/docs/git-instaweb>.
//! There is no stock page oracle (the clean-room image's gitweb.cgi cannot
//! load the Perl CGI module), so the markup is arks's own; the pages carry
//! the same facts: commit ids, subjects, authors, dates, file lists and blob
//! bytes.
//!
//! One project is served: the git directory, named by its directory name.
//! Actions: `project_list`, `project_index`, `summary`, `shortlog`, `log`,
//! `commit`, `commitdiff`, `commitdiff_plain`, `patch`, `tree`, `blob`,
//! `blob_plain`, `heads`, `tags`, `tag`, `rss` and `atom`. `blame` and
//! `blame_incremental` are off (the documented default for blame) and answer
//! 403; the other documented actions answer 501.

mod commit;
mod feed;
mod html;
mod model;
mod params;
mod project;
mod tree;

use arks_config::ConfigSet;
use arks_object::ObjectKind;
use arks_repo::Repository;
use arks_server::{HttpRequest, HttpResponse, PageHandler};
use model::Ctx;
use params::{Params, Target, route};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Shortlog / log page size.
const PAGE: usize = 100;
/// Rows per section on the summary page.
const SUMMARY_ROWS: usize = 16;
/// Feed entries.
const FEED_ITEMS: usize = 20;

/// The page handler `serve-http --gitweb` installs for `git_dir`.
pub fn page_handler(git_dir: &Path) -> PageHandler {
    let git_dir = git_dir.to_path_buf();
    let name = project_name(&git_dir);
    Arc::new(move |req: &HttpRequest| {
        let target = route(&req.path, &req.query, &name)?;
        Some(match target {
            Target::Static(file) => static_file(&file),
            Target::Page(p) => match Repository::open(&git_dir) {
                Ok(repo) => {
                    let cfg = ConfigSet::load(Some(repo.common_dir()));
                    let host = req.host.clone().filter(|h| {
                        !h.is_empty()
                            && h.chars()
                                .all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c))
                    });
                    let ctx = Ctx {
                        repo,
                        name: name.clone(),
                        cfg,
                        host,
                    };
                    dispatch(&ctx, &p)
                }
                Err(e) => text(500, "text/plain; charset=utf-8", e.to_string().into_bytes()),
            },
        })
    })
}

/// The project name: the git directory's own name (`.git` for a repository
/// with a work tree, `<name>.git` for a bare one).
fn project_name(git_dir: &Path) -> String {
    let dir: PathBuf = std::fs::canonicalize(git_dir).unwrap_or_else(|_| git_dir.to_path_buf());
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".git".into())
}

fn dispatch(ctx: &Ctx, p: &Params) -> HttpResponse {
    if let Some(name) = &p.project {
        if *name != ctx.name {
            return error(None, 404, "No such project");
        }
    }
    let project = p.project.is_some();
    let action = match (&p.action, project) {
        (Some(a), _) => a.as_str(),
        (None, false) => "project_list",
        (None, true) if p.file.is_some() => default_for_path(ctx, p),
        (None, true) => "summary",
    };
    if !project && !matches!(action, "project_list" | "project_index") {
        return error(None, 400, "Project needed");
    }
    match action {
        "project_list" => project::project_list(ctx),
        "project_index" => feed::project_index(ctx),
        "summary" => project::summary(ctx),
        "shortlog" => project::shortlog(ctx, p),
        "log" => project::log(ctx, p),
        "heads" => project::heads(ctx),
        "tags" => project::tags(ctx),
        "tag" => project::tag(ctx, p),
        "commit" => commit::commit(ctx, p),
        "commitdiff" => commit::commitdiff(ctx, p),
        "commitdiff_plain" => commit::commitdiff_plain(ctx, p),
        "patch" => commit::patch(ctx, p),
        "tree" => tree::tree(ctx, p),
        "blob" => tree::blob(ctx, p),
        "blob_plain" => tree::blob_plain(ctx, p),
        "rss" => feed::rss(ctx),
        "atom" => feed::atom(ctx),
        "blame" | "blame_incremental" => error(Some(ctx), 403, "Permission denied"),
        a if params::ACTIONS.contains(&a) => error(Some(ctx), 501, "Action not implemented"),
        _ => error(Some(ctx), 400, "Unknown action"),
    }
}

/// With a path and no action: a tree shows as `tree`, a file as `blob`.
fn default_for_path(ctx: &Ctx, p: &Params) -> &'static str {
    let rev = p.hash_base.as_deref().unwrap_or("HEAD");
    let kind = ctx
        .resolve_commit(rev)
        .and_then(|(_, c)| ctx.at_path(&c, p.file.as_deref().unwrap_or("")))
        .and_then(|o| ctx.kind(&o));
    match kind {
        Some(ObjectKind::Blob) => "blob",
        _ => "tree",
    }
}

fn static_file(name: &str) -> HttpResponse {
    match name {
        "gitweb.css" => text(200, "text/css", html::CSS.as_bytes().to_vec()),
        _ => text(404, "text/plain; charset=utf-8", b"Not Found\n".to_vec()),
    }
}

pub(super) fn text(status: u16, content_type: &str, body: Vec<u8>) -> HttpResponse {
    HttpResponse {
        status,
        content_type: content_type.into(),
        body,
        www_authenticate: None,
        extra_headers: Vec::new(),
    }
}

/// A 200 HTML page for `action` with the project navigation.
pub(super) fn page(ctx: &Ctx, action: &str, rev: Option<&str>, body: &str) -> HttpResponse {
    let nav = html::nav(&ctx.name, action, rev);
    let doc = html::frame(Some(&ctx.name), action, &nav, body);
    text(200, "text/html; charset=utf-8", doc.into_bytes())
}

/// An error page: `<status> - <message>`.
pub(super) fn error(ctx: Option<&Ctx>, status: u16, msg: &str) -> HttpResponse {
    let body = format!("<div class=\"error\">{status} - {}</div>\n", html::esc(msg));
    let doc = html::frame(ctx.map(|c| c.name.as_str()), "error", "", &body);
    text(status, "text/html; charset=utf-8", doc.into_bytes())
}

pub(super) fn not_found(ctx: &Ctx, what: &str) -> HttpResponse {
    error(Some(ctx), 404, what)
}
