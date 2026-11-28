//! Self-hosted forge tabs (`Issues`, `Pull Requests`, `Actions`, `Projects`, `Wiki`,
//! `Security`, `Insights`, `Settings`) surfacing live repository state cleanly.

use super::diff::render_diff_between_commits;
use super::store::{ForgeStore, parse_form_urlencoded};
use super::{
    ActiveTab, RepoContext, collect_recent_commits, format_timestamp, html_escape, load_commit,
    redirect_see_other, render_page, resolve_rev_oid,
};
use crate::http::{HttpRequest, HttpResponse};
use arks_object::TreeMode;
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BTreeMap, HashSet, VecDeque};

/// Render `/issues` backed by persistent `<git_dir>/arks-forge/store.json` with inline New Issue form.
pub fn render_issues(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let store = ForgeStore::load(repo.common_dir());
    let open_count = store.issues.iter().filter(|i| i.state == "open").count();
    let closed_count = store.issues.len().saturating_sub(open_count);

    let mut rows = String::new();
    for issue in store.issues.iter().rev() {
        let badge = if issue.state == "open" {
            "<span class=\"badge badge-green\">open</span>"
        } else {
            "<span class=\"badge badge-purple\">closed</span>"
        };
        rows.push_str(&format!(
            "<tr><td>{badge} <a href=\"/issues/{}\"><strong>#{} {}</strong></a>\
             <div class=\"muted\">opened by {} · {} · {} comment(s)</div></td>\
             <td style=\"text-align:right\"><a class=\"btn\" href=\"/issues/{}\">Open</a></td></tr>",
            issue.id,
            issue.id,
            html_escape(&issue.title),
            html_escape(&issue.author),
            html_escape(&format_timestamp(issue.created_at)),
            issue.comments.len(),
            issue.id
        ));
    }
    if rows.is_empty() {
        rows.push_str(
            "<tr><td colspan=\"2\" class=\"muted\">No issues yet. Use the form below to open the first issue.</td></tr>",
        );
    }

    let body = format!(
        "<div class=\"toolbar\"><div class=\"toolbar-group\">\
         <span class=\"badge badge-green\">{open_count} Open</span>\
         <span class=\"badge badge-purple\">{closed_count} Closed</span></div>\
         <span class=\"muted mono\">Persistent local forge store</span></div>\
         <div class=\"card\"><div class=\"card-header\"><strong>Issues</strong></div>\
         <table class=\"tree-table\">{rows}</table></div>\
         <div class=\"card\"><div class=\"card-header\"><strong>New Issue</strong></div>\
         <form class=\"card-body\" method=\"post\" action=\"/issues\" style=\"display:flex;flex-direction:column;gap:10px\">\
           <input class=\"input\" type=\"text\" name=\"title\" placeholder=\"Issue title\" required />\
           <textarea class=\"input\" name=\"body\" rows=\"4\" placeholder=\"Describe the issue...\"></textarea>\
           <div style=\"display:flex;gap:8px;align-items:center\">\
             <input class=\"input\" type=\"text\" name=\"author\" placeholder=\"Author (default: operator)\" />\
             <button class=\"btn btn-primary\" type=\"submit\">Create Issue</button>\
           </div>\
         </form></div>"
    );
    render_page(
        200,
        ctx,
        ActiveTab::Issues,
        &format!("Issues — {}", ctx.name),
        &body,
    )
}

/// Handle `POST /issues` creating a persistent issue and redirecting `303` to `/issues/<id>`.
pub fn handle_create_issue(
    repo: &Repository,
    ctx: &RepoContext,
    req: &HttpRequest,
) -> HttpResponse {
    let form = parse_form_urlencoded(&req.body);
    let title = form.get("title").map(|s| s.trim()).unwrap_or("");
    if title.is_empty() {
        return render_page(
            400,
            ctx,
            ActiveTab::Issues,
            "Invalid issue",
            "<div class=\"card\"><div class=\"card-body\">Issue title is required.</div></div>",
        );
    }
    let body = form.get("body").cloned().unwrap_or_default();
    let author = form
        .get("author")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("operator")
        .to_string();

    let mut store = ForgeStore::load(repo.common_dir());
    let id = store.create_issue(title.to_string(), body, author);
    if let Err(e) = store.save(repo.common_dir()) {
        return render_page(
            500,
            ctx,
            ActiveTab::Issues,
            "Store error",
            &format!(
                "<div class=\"card\"><div class=\"card-body\">Failed to save issue: {}</div></div>",
                html_escape(&e.to_string())
            ),
        );
    }
    redirect_see_other(&format!("/issues/{id}"))
}

/// Render `/issues/<id>` detail view with comments, close/reopen button, and add-comment form.
pub fn render_issue_detail(repo: &Repository, ctx: &RepoContext, id_str: &str) -> HttpResponse {
    let Ok(id) = id_str.parse::<u64>() else {
        return render_page(
            404,
            ctx,
            ActiveTab::Issues,
            "Not found",
            "Invalid issue ID.",
        );
    };
    let store = ForgeStore::load(repo.common_dir());
    let Some(issue) = store.issues.iter().find(|i| i.id == id) else {
        return render_page(404, ctx, ActiveTab::Issues, "Not found", "Issue not found.");
    };
    let badge = if issue.state == "open" {
        "<span class=\"badge badge-green\">open</span>"
    } else {
        "<span class=\"badge badge-purple\">closed</span>"
    };
    let toggle_label = if issue.state == "open" {
        "Close issue"
    } else {
        "Reopen issue"
    };

    let mut comments_html = String::new();
    for c in &issue.comments {
        comments_html.push_str(&format!(
            "<div class=\"card\"><div class=\"card-header\">\
             <span><strong>{}</strong> <span class=\"muted\">commented · {}</span></span></div>\
             <div class=\"card-body\"><pre style=\"margin:0;white-space:pre-wrap\">{}</pre></div></div>",
            html_escape(&c.author),
            html_escape(&format_timestamp(c.created_at)),
            html_escape(&c.body)
        ));
    }

    let body = format!(
        "<div class=\"toolbar\">\
           <div class=\"toolbar-group\"><a href=\"/issues\">← Issues</a> {badge} \
           <strong>#{} {}</strong></div>\
           <form method=\"post\" action=\"/issues/{}/state\">\
             <button class=\"btn\" type=\"submit\">{toggle_label}</button>\
           </form></div>\
         <div class=\"card\"><div class=\"card-header\">\
           <span><strong>{}</strong> <span class=\"muted\">opened · {}</span></span></div>\
           <div class=\"card-body\"><pre style=\"margin:0;white-space:pre-wrap\">{}</pre></div></div>\
         {comments_html}\
         <div class=\"card\"><div class=\"card-header\"><strong>Add Comment</strong></div>\
         <form class=\"card-body\" method=\"post\" action=\"/issues/{}/comment\" style=\"display:flex;flex-direction:column;gap:10px\">\
           <textarea class=\"input\" name=\"body\" rows=\"3\" placeholder=\"Write a comment...\" required></textarea>\
           <div style=\"display:flex;gap:8px\">\
             <input class=\"input\" type=\"text\" name=\"author\" placeholder=\"Author (default: operator)\" />\
             <button class=\"btn btn-primary\" type=\"submit\">Comment</button>\
           </div>\
         </form></div>",
        issue.id,
        html_escape(&issue.title),
        issue.id,
        html_escape(&issue.author),
        html_escape(&format_timestamp(issue.created_at)),
        html_escape(&issue.body),
        issue.id
    );
    render_page(
        200,
        ctx,
        ActiveTab::Issues,
        &format!("#{} {} — {}", issue.id, issue.title, ctx.name),
        &body,
    )
}

/// Handle `POST /issues/<id>/comment`.
pub fn handle_issue_comment(
    repo: &Repository,
    ctx: &RepoContext,
    id_str: &str,
    req: &HttpRequest,
) -> HttpResponse {
    let Ok(id) = id_str.parse::<u64>() else {
        return render_page(
            404,
            ctx,
            ActiveTab::Issues,
            "Not found",
            "Invalid issue ID.",
        );
    };
    let form = parse_form_urlencoded(&req.body);
    let body = form.get("body").map(|s| s.trim()).unwrap_or("");
    if body.is_empty() {
        return redirect_see_other(&format!("/issues/{id}"));
    }
    let author = form
        .get("author")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("operator")
        .to_string();
    let mut store = ForgeStore::load(repo.common_dir());
    if !store.add_issue_comment(id, author, body.to_string()) {
        return render_page(404, ctx, ActiveTab::Issues, "Not found", "Issue not found.");
    }
    let _ = store.save(repo.common_dir());
    redirect_see_other(&format!("/issues/{id}"))
}

/// Handle `POST /issues/<id>/state` toggling open <-> closed.
pub fn handle_issue_state(repo: &Repository, ctx: &RepoContext, id_str: &str) -> HttpResponse {
    let Ok(id) = id_str.parse::<u64>() else {
        return render_page(
            404,
            ctx,
            ActiveTab::Issues,
            "Not found",
            "Invalid issue ID.",
        );
    };
    let mut store = ForgeStore::load(repo.common_dir());
    if store.toggle_issue_state(id).is_none() {
        return render_page(404, ctx, ActiveTab::Issues, "Not found", "Issue not found.");
    }
    let _ = store.save(repo.common_dir());
    redirect_see_other(&format!("/issues/{id}"))
}

/// Render `/pulls` listing active non-default branches linking to the PR Review Workbench.
pub fn render_pulls(_repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let mut rows = String::new();
    let mut open_prs = 0usize;
    for (branch, oid) in &ctx.branches {
        if branch == &ctx.default_branch {
            continue;
        }
        open_prs += 1;
        let pr_href = format!("/pulls/{}", html_escape(branch));
        rows.push_str(&format!(
            "<tr><td><span class=\"badge badge-green\">open</span> \
             <a href=\"{pr_href}\"><strong>{} ← {}</strong></a>\
             <div class=\"muted mono\">tip {}</div></td>\
             <td style=\"text-align:right\"><a class=\"btn btn-primary\" href=\"{pr_href}\">Review &amp; Merge</a></td></tr>",
            html_escape(&ctx.default_branch),
            html_escape(branch),
            html_escape(&oid.to_hex()[..7])
        ));
    }
    if rows.is_empty() {
        rows.push_str(
            "<tr><td colspan=\"2\" class=\"muted\">No feature branches ahead of default branch. \
             Push a branch or run <code>arks pr list</code>.</td></tr>",
        );
    }
    let body = format!(
        "<div class=\"toolbar\"><div class=\"toolbar-group\">\
         <span class=\"badge badge-green\">{open_prs} Active Branches</span>\
         <span class=\"muted\">Click any branch to open the PR Review &amp; Merge Workbench</span></div>\
         <a class=\"btn\" href=\"/compare\">Compare Refs</a></div>\
         <div class=\"card\"><div class=\"card-header\"><strong>Pull Requests</strong>\
         <span class=\"muted\">Synced with <code>arks pr</code></span></div>\
         <table class=\"tree-table\">{rows}</table></div>"
    );
    render_page(
        200,
        ctx,
        ActiveTab::Pulls,
        &format!("Pull Requests — {}", ctx.name),
        &body,
    )
}

/// Render `/pulls/<branch>` PR Review Workbench (Mergeability, Fast-Forward Merge, Review Comments, Dual-Line Diff).
pub fn render_pr_detail(repo: &Repository, ctx: &RepoContext, branch: &str) -> HttpResponse {
    let Some(base_oid) = resolve_rev_oid(repo, &ctx.default_branch) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Pulls,
            "Not found",
            "Default branch not found.",
        );
    };
    let Some(head_oid) = resolve_rev_oid(repo, branch) else {
        return render_page(404, ctx, ActiveTab::Pulls, "Not found", "Branch not found.");
    };

    let already_merged = base_oid == head_oid;
    let ff_ok = !already_merged && is_ancestor(repo, base_oid, head_oid);

    let merge_widget = if already_merged {
        "<span class=\"badge badge-purple\">Merged · Up to date with default branch</span>".into()
    } else if ff_ok {
        format!(
            "<form method=\"post\" action=\"/pulls/{}/merge\">\
             <button class=\"btn btn-primary\" type=\"submit\">Merge pull request (fast-forward)</button></form>",
            html_escape(branch)
        )
    } else {
        "<span class=\"badge\" style=\"color:var(--red)\">Diverged · Rebase or replay before fast-forward merge</span>".into()
    };

    let store = ForgeStore::load(repo.common_dir());
    let mut comments_html = String::new();
    for c in store.pr_comments.iter().filter(|c| c.branch == branch) {
        comments_html.push_str(&format!(
            "<div class=\"card\"><div class=\"card-header\">\
             <span><strong>{}</strong> <span class=\"muted\">reviewed · {}</span></span></div>\
             <div class=\"card-body\"><pre style=\"margin:0;white-space:pre-wrap\">{}</pre></div></div>",
            html_escape(&c.author),
            html_escape(&format_timestamp(c.created_at)),
            html_escape(&c.body)
        ));
    }

    let diff_html = render_diff_between_commits(repo, base_oid, head_oid);
    let body = format!(
        "<div class=\"toolbar\">\
           <div class=\"toolbar-group\"><a href=\"/pulls\">← Pull Requests</a> \
           <strong>{} ← {}</strong></div>\
           {merge_widget}</div>\
         {diff_html}\
         {comments_html}\
         <div class=\"card\"><div class=\"card-header\"><strong>Add Review Comment</strong></div>\
         <form class=\"card-body\" method=\"post\" action=\"/pulls/{}/comment\" style=\"display:flex;flex-direction:column;gap:10px\">\
           <textarea class=\"input\" name=\"body\" rows=\"3\" placeholder=\"Leave a review comment...\" required></textarea>\
           <div style=\"display:flex;gap:8px\">\
             <input class=\"input\" type=\"text\" name=\"author\" placeholder=\"Reviewer (default: operator)\" />\
             <button class=\"btn btn-primary\" type=\"submit\">Submit Comment</button>\
           </div>\
         </form></div>",
        html_escape(&ctx.default_branch),
        html_escape(branch),
        html_escape(branch)
    );
    render_page(
        200,
        ctx,
        ActiveTab::Pulls,
        &format!("PR {branch} — {}", ctx.name),
        &body,
    )
}

/// Handle `POST /pulls/<branch>/comment`.
pub fn handle_pr_comment(repo: &Repository, branch: &str, req: &HttpRequest) -> HttpResponse {
    let form = parse_form_urlencoded(&req.body);
    let body = form.get("body").map(|s| s.trim()).unwrap_or("");
    if !body.is_empty() {
        let author = form
            .get("author")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("operator")
            .to_string();
        let mut store = ForgeStore::load(repo.common_dir());
        store.add_pr_comment(branch.to_string(), author, body.to_string());
        let _ = store.save(repo.common_dir());
    }
    redirect_see_other(&format!("/pulls/{branch}"))
}

/// Handle `POST /pulls/<branch>/merge` performing a safe native fast-forward merge into default branch.
pub fn handle_pr_merge(repo: &mut Repository, ctx: &RepoContext, branch: &str) -> HttpResponse {
    let Some(base_oid) = resolve_rev_oid(repo, &ctx.default_branch) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Pulls,
            "Not found",
            "Default branch not found.",
        );
    };
    let Some(head_oid) = resolve_rev_oid(repo, branch) else {
        return render_page(404, ctx, ActiveTab::Pulls, "Not found", "Branch not found.");
    };
    if base_oid == head_oid {
        return redirect_see_other(&format!("/pulls/{branch}"));
    }
    if !is_ancestor(repo, base_oid, head_oid) {
        return render_page(
            409,
            ctx,
            ActiveTab::Pulls,
            "Merge conflict",
            "<div class=\"card\"><div class=\"card-body\">Fast-forward merge refused: branches have diverged.</div></div>",
        );
    }
    let target_ref = format!("refs/heads/{}", ctx.default_branch);
    if let Err(e) = repo
        .refs_mut()
        .update(&target_ref, &head_oid, Some(&base_oid))
    {
        return render_page(
            409,
            ctx,
            ActiveTab::Pulls,
            "Ref update conflict",
            &format!(
                "<div class=\"card\"><div class=\"card-body\">Ref update failed: {}</div></div>",
                html_escape(&e.to_string())
            ),
        );
    }
    redirect_see_other(&format!("/pulls/{branch}"))
}

fn is_ancestor(repo: &Repository, ancestor: Oid, descendant: Oid) -> bool {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(descendant);
    while let Some(oid) = queue.pop_front() {
        if oid == ancestor {
            return true;
        }
        if visited.len() > 2048 || !visited.insert(oid) {
            continue;
        }
        if let Some(c) = load_commit(repo, &oid) {
            for p in c.parents {
                queue.push_back(p);
            }
        }
    }
    false
}

/// Render `/actions` inspecting real `.github/workflows/` and `.arks/workflows/` files in HEAD.
pub fn render_actions(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let mut workflows = Vec::new();
    if let Some(head_oid) = resolve_rev_oid(repo, &ctx.default_branch) {
        if let Some(commit) = super::load_commit(repo, &head_oid) {
            for dir_path in [".github/workflows", ".arks/workflows"] {
                collect_dir_names(repo, commit.tree, dir_path, &mut workflows);
            }
        }
    }

    let mut side = String::new();
    let mut main_rows = String::new();
    for wf in &workflows {
        let href = format!(
            "/blob/{}/{}",
            html_escape(&ctx.default_branch),
            html_escape(wf)
        );
        side.push_str(&format!("<a href=\"{href}\">{}</a>", html_escape(wf)));
        main_rows.push_str(&format!(
            "<tr><td><a href=\"{href}\"><strong>{}</strong></a></td>\
             <td class=\"muted mono\">{}</td>\
             <td style=\"text-align:right\"><a class=\"btn\" href=\"{href}\">View YAML</a></td></tr>",
            html_escape(wf),
            html_escape(&ctx.default_branch)
        ));
    }
    if workflows.is_empty() {
        side.push_str(
            "<div class=\"muted\" style=\"padding:8px\">No workflow files detected</div>",
        );
        main_rows.push_str(
            "<tr><td colspan=\"3\" class=\"muted\">No <code>.github/workflows/</code> or <code>.arks/workflows/</code> files in default branch. \
             Manage remote runs via <code>arks workflow list</code> / <code>arks run list</code>.</td></tr>",
        );
    }

    let body = format!(
        "<div class=\"layout-side\">\
         <aside class=\"card\"><div class=\"card-header\"><strong>Workflows</strong></div>\
         <div class=\"tree-sidebar-list\">{side}</div></aside>\
         <div><div class=\"card\"><div class=\"card-header\">\
         <strong>Actions &amp; Pipelines</strong>\
         <span class=\"badge\">CLI: arks workflow / arks run</span></div>\
         <table class=\"tree-table\">{main_rows}</table></div></div></div>"
    );
    render_page(
        200,
        ctx,
        ActiveTab::Actions,
        &format!("Actions — {}", ctx.name),
        &body,
    )
}

/// Render `/projects` Kanban view organized by repository branches & release tags.
pub fn render_projects(_repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let mut active_cards = String::new();
    for (b, oid) in &ctx.branches {
        if b != &ctx.default_branch {
            active_cards.push_str(&format!(
                "<div class=\"kanban-item\"><a href=\"/compare/{}..{}\"><strong>{}</strong></a>\
                 <div class=\"muted mono\">{}</div></div>",
                html_escape(&ctx.default_branch),
                html_escape(b),
                html_escape(b),
                html_escape(&oid.to_hex()[..7])
            ));
        }
    }
    if active_cards.is_empty() {
        active_cards.push_str("<div class=\"kanban-item muted\">No active feature branches</div>");
    }

    let mut done_cards = String::new();
    for (t, oid) in ctx.tags.iter().take(6) {
        done_cards.push_str(&format!(
            "<div class=\"kanban-item\"><a href=\"/tree/{}\"><strong>{}</strong></a>\
             <div class=\"muted mono\">{}</div></div>",
            html_escape(t),
            html_escape(t),
            html_escape(&oid.to_hex()[..7])
        ));
    }
    if done_cards.is_empty() {
        done_cards.push_str("<div class=\"kanban-item muted\">No release tags yet</div>");
    }

    let body = format!(
        "<div class=\"toolbar\"><strong>Projects Board</strong>\
         <span class=\"muted\">Live view mapped from branches &amp; release tags</span></div>\
         <div class=\"kanban-grid\">\
           <div class=\"kanban-col\"><strong>Default Branch</strong>\
             <div class=\"kanban-item\"><a href=\"/tree/{}\"><strong>{}</strong></a>\
             <div class=\"muted\">Primary integration branch</div></div></div>\
           <div class=\"kanban-col\"><strong>In Progress / Review</strong>{active_cards}</div>\
           <div class=\"kanban-col\"><strong>Shipped Tags</strong>{done_cards}</div>\
         </div>",
        html_escape(&ctx.default_branch),
        html_escape(&ctx.default_branch)
    );
    render_page(
        200,
        ctx,
        ActiveTab::Projects,
        &format!("Projects — {}", ctx.name),
        &body,
    )
}

/// Render `/wiki` inspecting `docs/` Markdown files in the repository.
pub fn render_wiki(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let mut docs = Vec::new();
    if let Some(head_oid) = resolve_rev_oid(repo, &ctx.default_branch) {
        if let Some(commit) = super::load_commit(repo, &head_oid) {
            collect_dir_names(repo, commit.tree, "docs", &mut docs);
        }
    }
    docs.retain(|p| p.ends_with(".md"));
    let mut links = String::new();
    for d in docs.iter().take(30) {
        let href = format!(
            "/blob/{}/{}",
            html_escape(&ctx.default_branch),
            html_escape(d)
        );
        links.push_str(&format!("<a href=\"{href}\">{}</a>", html_escape(d)));
    }
    if links.is_empty() {
        links.push_str(
            "<div class=\"muted\" style=\"padding:8px\">No Markdown pages in docs/</div>",
        );
    }
    let body = format!(
        "<div class=\"layout-side\">\
         <aside class=\"card\"><div class=\"card-header\"><strong>Wiki &amp; Docs</strong></div>\
         <div class=\"tree-sidebar-list\">{links}</div></aside>\
         <div class=\"card\"><div class=\"card-header\"><strong>Repository Wiki</strong></div>\
         <div class=\"card-body\"><p class=\"muted\">Browse repository Markdown documentation directly from \
         <code>docs/</code> on <code>{}</code>, or host a companion <code>*.wiki.git</code> repository with <code>arks serve</code>.</p></div></div></div>",
        html_escape(&ctx.default_branch)
    );
    render_page(
        200,
        ctx,
        ActiveTab::Wiki,
        &format!("Wiki — {}", ctx.name),
        &body,
    )
}

/// Render `/security` showing real commit signature coverage and repository transport posture.
pub fn render_security(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let commits = resolve_rev_oid(repo, &ctx.default_branch)
        .map(|o| collect_recent_commits(repo, o, 50))
        .unwrap_or_default();
    let total = commits.len();
    let signed = commits
        .iter()
        .filter(|(_, c)| super::commit_has_sig(c))
        .count();
    let pct = if total == 0 {
        0.0
    } else {
        (signed as f64 / total as f64) * 100.0
    };

    let body = format!(
        "<div class=\"stat-grid\">\
           <div class=\"stat-card\"><div class=\"muted\">Signed Commits (recent {total})</div>\
             <div class=\"stat-val\" style=\"color:var(--green)\">{signed} / {total} ({pct:.0}%)</div>\
             <div class=\"muted\">SSH · OpenPGP · X.509 (G37/G38)</div></div>\
           <div class=\"stat-card\"><div class=\"muted\">Web UI CSP Policy</div>\
             <div class=\"stat-val\" style=\"font-size:15px;margin-top:8px\" class=\"mono\">default-src 'self'</div>\
             <div class=\"muted\">Zero external CDN · strict HTML escaping</div></div>\
           <div class=\"stat-card\"><div class=\"muted\">Native Git LFS Store</div>\
             <div class=\"stat-val\">{}</div>\
             <div class=\"muted\">SHA-256 verified transfer</div></div>\
         </div>\
         <div class=\"card\"><div class=\"card-header\"><strong>Security Posture</strong></div>\
         <div class=\"card-body muted\">Verify commit signatures locally with <code>arks log --show-signature</code> \
         or <code>arks verify-commit HEAD</code>. Configure TLS and HTTP Basic/SSH auth via <code>arks serve config</code>.</div></div>",
        if ctx.has_lfs { "Active" } else { "Standby" }
    );
    render_page(
        200,
        ctx,
        ActiveTab::Security,
        &format!("Security — {}", ctx.name),
        &body,
    )
}

/// Render `/insights` showing live commit velocity, top contributors, and ref backend stats.
pub fn render_insights(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let commits = resolve_rev_oid(repo, &ctx.default_branch)
        .map(|o| collect_recent_commits(repo, o, 100))
        .unwrap_or_default();
    let mut authors: BTreeMap<String, usize> = BTreeMap::new();
    for (_, c) in &commits {
        *authors.entry(c.author.name.clone()).or_insert(0) += 1;
    }
    let mut contrib_rows = String::new();
    let mut sorted: Vec<_> = authors.into_iter().collect();
    sorted.sort_by_key(|a| std::cmp::Reverse(a.1));
    for (name, count) in sorted.into_iter().take(15) {
        contrib_rows.push_str(&format!(
            "<tr><td><strong>{}</strong></td><td class=\"mono\" style=\"text-align:right\">{} commit(s)</td></tr>",
            html_escape(&name),
            count
        ));
    }
    if contrib_rows.is_empty() {
        contrib_rows.push_str("<tr><td colspan=\"2\" class=\"muted\">No commits yet.</td></tr>");
    }

    let latest_ts = commits
        .first()
        .map(|(_, c)| format_timestamp(c.author.timestamp))
        .unwrap_or_else(|| "—".into());

    let body = format!(
        "<div class=\"stat-grid\">\
           <div class=\"stat-card\"><div class=\"muted\">Sampled Commits</div><div class=\"stat-val\">{}</div></div>\
           <div class=\"stat-card\"><div class=\"muted\">Branches</div><div class=\"stat-val\">{}</div></div>\
           <div class=\"stat-card\"><div class=\"muted\">Tags</div><div class=\"stat-val\">{}</div></div>\
           <div class=\"stat-card\"><div class=\"muted\">Latest Activity</div><div class=\"stat-val\" style=\"font-size:15px;margin-top:8px\">{}</div></div>\
         </div>\
         <div class=\"card\"><div class=\"card-header\"><strong>Contributors (recent 100 commits)</strong></div>\
         <table class=\"tree-table\">{contrib_rows}</table></div>",
        commits.len(),
        ctx.branches.len(),
        ctx.tags.len(),
        html_escape(&latest_ts)
    );
    render_page(
        200,
        ctx,
        ActiveTab::Insights,
        &format!("Insights — {}", ctx.name),
        &body,
    )
}

/// Render `/settings` showing live repository configuration, clone endpoints, and server flags.
pub fn render_settings(repo: &Repository, ctx: &RepoContext) -> HttpResponse {
    let reftable_active = repo.common_dir().join("reftable").is_dir();
    let ref_backend = if reftable_active { "reftable" } else { "files" };
    let body = format!(
        "<div class=\"card\"><div class=\"card-header\"><strong>Repository &amp; Server Settings</strong></div>\
         <table class=\"tree-table\">\
           <tr><td><strong>Repository Name</strong></td><td class=\"mono\">{}</td></tr>\
           <tr><td><strong>Default Branch</strong></td><td class=\"mono\">{}</td></tr>\
           <tr><td><strong>Clone Endpoint</strong></td><td class=\"mono\">{}</td></tr>\
           <tr><td><strong>Ref Storage Backend</strong></td><td class=\"mono\">{}</td></tr>\
           <tr><td><strong>Native Git LFS</strong></td><td>{}</td></tr>\
           <tr><td><strong>Embedded Web UI</strong></td><td><span class=\"badge badge-green\">Enabled</span> (opt-out: <code>--no-web-ui</code>)</td></tr>\
         </table></div>",
        html_escape(&ctx.name),
        html_escape(&ctx.default_branch),
        html_escape(&ctx.clone_url),
        ref_backend,
        if ctx.has_lfs {
            "<span class=\"badge badge-green\">Active</span>"
        } else {
            "<span class=\"badge\">Ready (no objects yet)</span>"
        }
    );
    render_page(
        200,
        ctx,
        ActiveTab::Settings,
        &format!("Settings — {}", ctx.name),
        &body,
    )
}

fn collect_dir_names(
    repo: &Repository,
    root_tree: arks_oid::Oid,
    dir_path: &str,
    out: &mut Vec<String>,
) {
    let mut cur = root_tree;
    for part in dir_path.split('/').filter(|s| !s.is_empty()) {
        let Some(tree) = super::load_tree(repo, &cur) else {
            return;
        };
        let Some(entry) = tree
            .entries
            .into_iter()
            .find(|e| e.name == part && e.mode == TreeMode::Tree)
        else {
            return;
        };
        cur = entry.oid;
    }
    let Some(tree) = super::load_tree(repo, &cur) else {
        return;
    };
    for e in tree.entries {
        if e.mode != TreeMode::Tree {
            out.push(format!("{dir_path}/{}", e.name));
        }
    }
}
