//! Code browsing views: 3-pane tree explorer, HTMX live `/search`, syntax-highlighted
//! `/blob/` + LFS pointer card + inline image preview, `/blame/`, `/commits` with HTMX
//! pagination, `/branches`, `/tags`, and `/raw/`.

use super::highlight::{compute_language_stats, detect_language, highlight_line};
use super::{
    ActiveTab, ICON_DIR, ICON_FILE, RepoContext, collect_recent_commits, format_timestamp,
    html_escape, render_page, render_partial, resolve_rev_oid,
};
use crate::http::HttpResponse;
use arks_object::{TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;

/// Resolve a path inside a root tree to either a subtree OID or a blob OID.
fn resolve_tree_path(repo: &Repository, root_tree: Oid, subpath: &str) -> Option<(TreeMode, Oid)> {
    let parts: Vec<&str> = subpath.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Some((TreeMode::Tree, root_tree));
    }
    let mut cur = root_tree;
    for (idx, part) in parts.iter().enumerate() {
        let tree = super::load_tree(repo, &cur)?;
        let entry = tree.entries.into_iter().find(|e| e.name == *part)?;
        if idx + 1 == parts.len() {
            return Some((entry.mode, entry.oid));
        }
        if entry.mode != TreeMode::Tree {
            return None;
        }
        cur = entry.oid;
    }
    None
}

fn sorted_tree_entries(repo: &Repository, tree_oid: Oid) -> Vec<TreeEntry> {
    let Some(tree) = super::load_tree(repo, &tree_oid) else {
        return Vec::new();
    };
    let mut entries = tree.entries;
    entries.sort_by(|a, b| {
        let ad = a.mode == TreeMode::Tree;
        let bd = b.mode == TreeMode::Tree;
        bd.cmp(&ad).then_with(|| a.name.cmp(&b.name))
    });
    entries
}

/// Render `/` or `/tree/<rev>[/<path>]` with 3-pane layout (left file tree + explorer + README + About/Languages).
pub fn render_tree(repo: &Repository, ctx: &RepoContext, rev: &str, subpath: &str) -> HttpResponse {
    let Some(commit_oid) = resolve_rev_oid(repo, rev) else {
        return render_page(
            200,
            ctx,
            ActiveTab::Code,
            &ctx.name,
            "<div class=\"card\"><div class=\"card-body\">\
             <h2>Empty Repository</h2>\
             <p class=\"muted\">Push your first commit to browse files here.</p></div></div>",
        );
    };
    let Some(commit) = super::load_commit(repo, &commit_oid) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not found",
            "<div class=\"card\"><div class=\"card-body\">Revision is not a commit.</div></div>",
        );
    };

    let Some((mode, target_oid)) = resolve_tree_path(repo, commit.tree, subpath) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Path not found",
            "<div class=\"card\"><div class=\"card-body\">Path not found in tree.</div></div>",
        );
    };
    if mode != TreeMode::Tree {
        return render_blob(repo, ctx, rev, subpath);
    }

    let root_entries = sorted_tree_entries(repo, commit.tree);
    let entries = sorted_tree_entries(repo, target_oid);

    // Left File Tree Sidebar
    let mut sidebar_tree = String::new();
    for e in &root_entries {
        let icon = if e.mode == TreeMode::Tree {
            ICON_DIR
        } else {
            ICON_FILE
        };
        let route = if e.mode == TreeMode::Tree {
            "tree"
        } else {
            "blob"
        };
        sidebar_tree.push_str(&format!(
            "<a href=\"/{route}/{}/{}\">{icon}<span>{}</span></a>",
            html_escape(rev),
            html_escape(&e.name),
            html_escape(&e.name)
        ));
    }

    // Branch switcher dropdown items
    let mut branch_items = String::new();
    for (b, _) in &ctx.branches {
        branch_items.push_str(&format!(
            "<a href=\"/tree/{}\"><span>{}</span><span class=\"muted\">branch</span></a>",
            html_escape(b),
            html_escape(b)
        ));
    }
    for (t, _) in &ctx.tags {
        branch_items.push_str(&format!(
            "<a href=\"/tree/{}\"><span>{}</span><span class=\"muted\">tag</span></a>",
            html_escape(t),
            html_escape(t)
        ));
    }

    // Main file rows
    let mut rows = String::new();
    if !subpath.is_empty() {
        let parent = subpath.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        let href = if parent.is_empty() {
            format!("/tree/{}", html_escape(rev))
        } else {
            format!("/tree/{}/{}", html_escape(rev), html_escape(parent))
        };
        rows.push_str(&format!(
            "<tr><td><a href=\"{href}\">..</a></td><td class=\"muted\">parent directory</td></tr>"
        ));
    }

    let mut readme_oid: Option<Oid> = None;
    for e in &entries {
        let full = if subpath.is_empty() {
            e.name.clone()
        } else {
            format!("{subpath}/{}", e.name)
        };
        let is_dir = e.mode == TreeMode::Tree;
        if !is_dir && e.name.eq_ignore_ascii_case("readme.md") {
            readme_oid = Some(e.oid);
        }
        let icon = if is_dir { ICON_DIR } else { ICON_FILE };
        let route = if is_dir { "tree" } else { "blob" };
        rows.push_str(&format!(
            "<tr><td>{icon} <a href=\"/{route}/{}/{}\">{}</a></td>\
             <td class=\"mono muted\" style=\"text-align:right\">{}</td></tr>",
            html_escape(rev),
            html_escape(&full),
            html_escape(&e.name),
            html_escape(&e.oid.to_hex()[..7])
        ));
    }

    // README preview
    let readme_html = readme_oid
        .and_then(|oid| super::load_blob(repo, &oid))
        .map(|bytes| {
            let text = String::from_utf8_lossy(&bytes);
            format!(
                "<div class=\"card\"><div class=\"card-header\"><strong>README.md</strong>\
                 <span class=\"badge\">GFM</span></div>\
                 <div class=\"card-body md\">{}</div></div>",
                render_safe_markdown(&text)
            )
        })
        .unwrap_or_default();

    // Languages bar
    let lang_stats = compute_language_stats(repo, commit.tree, 300);
    let mut lang_bar = String::new();
    let mut lang_legend = String::new();
    for st in &lang_stats {
        lang_bar.push_str(&format!(
            "<div style=\"width:{:.1}%;background:{}\" title=\"{} {:.1}%\"></div>",
            st.pct.max(1.5),
            st.color,
            st.name,
            st.pct
        ));
        lang_legend.push_str(&format!(
            "<span><i class=\"lang-dot\" style=\"background:{}\"></i><strong>{}</strong> \
             <span class=\"muted\">{:.1}%</span></span>",
            st.color, st.name, st.pct
        ));
    }
    let langs_section = if lang_stats.is_empty() {
        String::new()
    } else {
        format!(
            "<div class=\"sidebar-section\"><h3>Languages</h3>\
             <div class=\"lang-bar\">{lang_bar}</div>\
             <div class=\"lang-legend\">{lang_legend}</div></div>"
        )
    };

    let subject = commit.message.lines().next().unwrap_or("(no message)");
    let sig_badge = if super::commit_has_sig(&commit) {
        "<span class=\"badge badge-green\">Verified</span>"
    } else {
        ""
    };

    let body = format!(
        "<div class=\"layout-3\">\
         <aside class=\"tree-sidebar card\"><div class=\"card-header\"><strong>Files</strong>\
         <span class=\"muted mono\">{}</span></div>\
         <div class=\"tree-sidebar-list\">{sidebar_tree}</div></aside>\
         <div>\
           <div class=\"toolbar\">\
             <div class=\"toolbar-group\">\
               <details class=\"popover\">\
                 <summary class=\"btn\"><strong>{}</strong> ▾</summary>\
                 <div class=\"popover-panel\"><div class=\"popover-head\">Switch branch / tag</div>\
                 <div class=\"popover-list\">{branch_items}</div></div>\
               </details>\
               <input class=\"input\" type=\"search\" name=\"q\" placeholder=\"Find file (HTMX)...\" \
                 hx-get=\"/search?rev={}\" hx-trigger=\"input changed delay:200ms, search\" \
                 hx-target=\"#file-search-results\" />\
             </div>\
             <div class=\"toolbar-group\">\
               <a class=\"btn\" href=\"/commits/{}\">History</a>\
               <a class=\"btn\" href=\"/compare\">Compare</a>\
             </div>\
           </div>\
           <div id=\"file-search-results\"></div>\
           <div class=\"card\">\
             <div class=\"card-header\">\
               <div><strong>{}</strong> <a href=\"/commit/{}\" class=\"muted\">{}</a></div>\
               <div class=\"toolbar-group\">{sig_badge}<a href=\"/commit/{}\" class=\"mono muted\">{}</a>\
               <span class=\"muted\">{}</span></div>\
             </div>\
             <table class=\"tree-table\">{rows}</table>\
           </div>\
           {readme_html}\
         </div>\
         <aside>\
           <div class=\"sidebar-section\"><h3>About</h3>\
             <p class=\"muted\">{}</p>\
             <div class=\"toolbar-group\">\
               <span class=\"badge badge-green\">Smart HTTP</span>\
               <span class=\"badge\">HTMX v2</span>\
               {}\
             </div>\
           </div>\
           {langs_section}\
           <div class=\"sidebar-section\"><h3>Releases &amp; Refs</h3>\
             <div><a href=\"/branches\">{} branches</a> · <a href=\"/tags\">{} tags</a></div>\
           </div>\
         </aside></div>",
        html_escape(rev),
        html_escape(rev),
        html_escape(rev),
        html_escape(rev),
        html_escape(&commit.author.name),
        html_escape(&commit_oid.to_hex()),
        html_escape(subject),
        html_escape(&commit_oid.to_hex()),
        html_escape(&commit_oid.to_hex()[..7]),
        html_escape(&format_timestamp(commit.author.timestamp)),
        html_escape(&ctx.description),
        if ctx.has_lfs {
            "<span class=\"badge badge-gold\">Git LFS</span>"
        } else {
            ""
        },
        ctx.branches.len(),
        ctx.tags.len(),
    );

    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!("{} — {}", ctx.name, rev),
        &body,
    )
}

/// Render HTMX partial `/search?rev=<rev>&q=<query>` matching repository file paths.
pub fn render_search_partial(repo: &Repository, ctx: &RepoContext, query: &str) -> HttpResponse {
    let mut rev = ctx.default_branch.as_str();
    let mut q = String::new();
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == "rev" && !v.is_empty() {
                rev = v;
            } else if k == "q" {
                q = url_decode_basic(v);
            }
        }
    }
    let needle = q.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return render_partial(200, "");
    }
    let Some(commit_oid) = resolve_rev_oid(repo, rev) else {
        return render_partial(200, "");
    };
    let Some(commit) = super::load_commit(repo, &commit_oid) else {
        return render_partial(200, "");
    };

    let mut matches = Vec::new();
    search_tree_paths(repo, commit.tree, "", &needle, &mut matches, 30);

    let mut html = format!(
        "<div class=\"card\"><div class=\"card-header\"><strong>Search results for &ldquo;{}&rdquo;</strong>\
         <span class=\"muted\">{} match(es)</span></div><table class=\"tree-table\">",
        html_escape(&q),
        matches.len()
    );
    if matches.is_empty() {
        html.push_str("<tr><td class=\"muted\">No matching files.</td></tr>");
    } else {
        for m in matches {
            html.push_str(&format!(
                "<tr><td>{ICON_FILE} <a href=\"/blob/{}/{}\">{}</a></td></tr>",
                html_escape(rev),
                html_escape(&m),
                html_escape(&m)
            ));
        }
    }
    html.push_str("</table></div>");
    render_partial(200, &html)
}

fn search_tree_paths(
    repo: &Repository,
    tree_oid: Oid,
    prefix: &str,
    needle: &str,
    out: &mut Vec<String>,
    limit: usize,
) {
    if out.len() >= limit {
        return;
    }
    for e in sorted_tree_entries(repo, tree_oid) {
        if out.len() >= limit {
            break;
        }
        let full = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        if e.mode == TreeMode::Tree {
            search_tree_paths(repo, e.oid, &full, needle, out, limit);
        } else if full.to_ascii_lowercase().contains(needle) {
            out.push(full);
        }
    }
}

/// Render `/blob/<rev>/<path>` with pure-Rust syntax highlighting, LFS pointer card, or image preview.
pub fn render_blob(repo: &Repository, ctx: &RepoContext, rev: &str, path: &str) -> HttpResponse {
    let Some(commit_oid) = resolve_rev_oid(repo, rev) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not found",
            "<div class=\"card\"><div class=\"card-body\">Revision not found.</div></div>",
        );
    };
    let Some(commit) = super::load_commit(repo, &commit_oid) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not found",
            "<div class=\"card\"><div class=\"card-body\">Not a commit.</div></div>",
        );
    };
    let Some((mode, oid)) = resolve_tree_path(repo, commit.tree, path) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not found",
            "<div class=\"card\"><div class=\"card-body\">File not found.</div></div>",
        );
    };
    if mode == TreeMode::Tree {
        return render_tree(repo, ctx, rev, path);
    }
    let Some(bytes) = super::load_blob(repo, &oid) else {
        return render_page(404, ctx, ActiveTab::Code, "Not found", "Blob not found.");
    };

    let raw_href = format!("/raw/{}/{}", html_escape(rev), html_escape(path));
    let blame_href = format!("/blame/{}/{}", html_escape(rev), html_escape(path));
    let tree_href = format!("/tree/{}", html_escape(rev));

    let content_html = if let Some((lfs_oid, lfs_size)) = parse_lfs_pointer(&bytes) {
        format!(
            "<div class=\"card-body\" style=\"text-align:center;padding:32px\">\
             <span class=\"badge badge-gold\">Stored with Git LFS</span>\
             <h3 style=\"margin:12px 0 6px\">Git LFS Pointer ({} bytes)</h3>\
             <code class=\"muted\">sha256:{}</code>\
             <div style=\"margin-top:14px\"><a class=\"btn btn-primary\" href=\"{raw_href}\">Download Pointer / Raw</a></div></div>",
            lfs_size,
            html_escape(&lfs_oid)
        )
    } else if is_image_path(path) {
        format!(
            "<div class=\"card-body\" style=\"text-align:center;padding:24px\">\
             <img src=\"{raw_href}\" alt=\"{}\" style=\"max-width:100%;max-height:520px;border-radius:6px\"></div>",
            html_escape(path)
        )
    } else if bytes.contains(&0) {
        format!(
            "<div class=\"card-body muted\">Binary file ({} bytes). \
             <a href=\"{raw_href}\">Download raw</a></div>",
            bytes.len()
        )
    } else {
        let text = String::from_utf8_lossy(&bytes);
        let lang = detect_language(path).map(|(n, _)| n);
        let mut tbl = String::from("<div class=\"code-wrap\"><table class=\"code-table\">");
        for (i, line) in text.lines().enumerate() {
            let n = i + 1;
            tbl.push_str(&format!(
                "<tr id=\"L{n}\"><td class=\"ln\"><a class=\"ln-anchor\" href=\"#L{n}\">{n}</a></td>\
                 <td class=\"code-line\">{}</td></tr>",
                highlight_line(line, lang)
            ));
        }
        tbl.push_str("</table></div>");
        tbl
    };

    let lang_badge = detect_language(path)
        .map(|(n, _)| format!("<span class=\"badge\">{n}</span>"))
        .unwrap_or_default();

    let body = format!(
        "<div class=\"toolbar\">\
           <div class=\"toolbar-group\"><a href=\"{tree_href}\"><strong>{}</strong></a> / \
           <span class=\"mono\">{}</span> {lang_badge}</div>\
           <div class=\"toolbar-group\">\
             <div class=\"btn-group\">\
               <a class=\"btn active\" href=\"/blob/{}/{}\">Code</a>\
               <a class=\"btn\" href=\"{blame_href}\">Blame</a>\
             </div>\
             <a class=\"btn\" href=\"{raw_href}\">Raw</a>\
           </div>\
         </div>\
         <div class=\"card\"><div class=\"card-header\">\
           <span class=\"mono\"><strong>{}</strong> · {} bytes</span>\
           <span class=\"muted mono\">{}</span></div>{content_html}</div>",
        html_escape(&ctx.name),
        html_escape(path),
        html_escape(rev),
        html_escape(path),
        html_escape(path),
        bytes.len(),
        html_escape(&oid.to_hex()[..7]),
    );
    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!("{path} — {}", ctx.name),
        &body,
    )
}

/// Render `/blame/<rev>/<path>` attributing each line to the latest commit touching it.
pub fn render_blame(repo: &Repository, ctx: &RepoContext, rev: &str, path: &str) -> HttpResponse {
    let Some(head_oid) = resolve_rev_oid(repo, rev) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not found",
            "Revision not found.",
        );
    };
    let commits = collect_recent_commits(repo, head_oid, 40);
    let Some((_, head_commit)) = commits.first() else {
        return render_page(404, ctx, ActiveTab::Code, "Not found", "Commit not found.");
    };
    let Some((_, blob_oid)) = resolve_tree_path(repo, head_commit.tree, path) else {
        return render_page(404, ctx, ActiveTab::Code, "Not found", "File not found.");
    };
    let Some(bytes) = super::load_blob(repo, &blob_oid) else {
        return render_page(404, ctx, ActiveTab::Code, "Not found", "Blob not found.");
    };
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();

    // Find oldest commit in recent history where each line already existed identically
    let mut attribution: Vec<(Oid, String)> =
        vec![(head_oid, head_commit.author.name.clone()); lines.len()];
    for (c_oid, c) in &commits {
        let Some((_, old_oid)) = resolve_tree_path(repo, c.tree, path) else {
            break;
        };
        let Some(old_bytes) = super::load_blob(repo, &old_oid) else {
            continue;
        };
        let old_text = String::from_utf8_lossy(&old_bytes);
        let old_lines: Vec<&str> = old_text.lines().collect();
        for (idx, line) in lines.iter().enumerate() {
            if old_lines.get(idx) == Some(line) {
                attribution[idx] = (*c_oid, c.author.name.clone());
            }
        }
    }

    let lang = detect_language(path).map(|(n, _)| n);
    let mut tbl = String::from("<div class=\"code-wrap\"><table class=\"code-table\">");
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        let (c_oid, author) = &attribution[i];
        tbl.push_str(&format!(
            "<tr id=\"L{n}\"><td class=\"blame-cell\"><a href=\"/commit/{}\">{}</a> · {}</td>\
             <td class=\"ln\"><a class=\"ln-anchor\" href=\"#L{n}\">{n}</a></td>\
             <td class=\"code-line\">{}</td></tr>",
            html_escape(&c_oid.to_hex()),
            html_escape(&c_oid.to_hex()[..7]),
            html_escape(author),
            highlight_line(line, lang)
        ));
    }
    tbl.push_str("</table></div>");

    let body = format!(
        "<div class=\"toolbar\">\
           <div class=\"toolbar-group\"><a href=\"/tree/{}\"><strong>{}</strong></a> / \
           <span class=\"mono\">{}</span> <span class=\"badge\">Blame</span></div>\
           <div class=\"btn-group\">\
             <a class=\"btn\" href=\"/blob/{}/{}\">Code</a>\
             <a class=\"btn active\" href=\"/blame/{}/{}\">Blame</a>\
           </div></div>\
         <div class=\"card\"><div class=\"card-header\"><strong>Blame: {}</strong>\
         <span class=\"muted\">{} lines</span></div>{tbl}</div>",
        html_escape(rev),
        html_escape(&ctx.name),
        html_escape(path),
        html_escape(rev),
        html_escape(path),
        html_escape(rev),
        html_escape(path),
        html_escape(path),
        lines.len()
    );
    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!("Blame {path} — {}", ctx.name),
        &body,
    )
}

/// Render `/commits[/<rev>]` with HTMX `?after=<oid>&partial=1` pagination.
pub fn render_commits(
    repo: &Repository,
    ctx: &RepoContext,
    rev: &str,
    query: &str,
) -> HttpResponse {
    let mut after: Option<String> = None;
    let mut partial = false;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == "after" && !v.is_empty() {
                after = Some(v.to_string());
            } else if k == "partial" && v == "1" {
                partial = true;
            }
        }
    }
    let start_rev = after.as_deref().unwrap_or(rev);
    let Some(start_oid) = resolve_rev_oid(repo, start_rev) else {
        return render_page(404, ctx, ActiveTab::Code, "Commits", "Revision not found.");
    };
    let page_size = 25usize;
    let mut commits = collect_recent_commits(repo, start_oid, page_size + 1);
    let next_oid = if commits.len() > page_size {
        commits.pop().map(|(o, _)| o.to_hex())
    } else {
        None
    };

    let mut rows = String::new();
    for (oid, c) in &commits {
        let subject = c.message.lines().next().unwrap_or("(no message)");
        rows.push_str(&format!(
            "<tr><td><a href=\"/commit/{}\"><strong>{}</strong></a>\
             <div class=\"muted\">{} · {}</div></td>\
             <td class=\"mono\" style=\"text-align:right\"><a href=\"/commit/{}\">{}</a></td></tr>",
            html_escape(&oid.to_hex()),
            html_escape(subject),
            html_escape(&c.author.name),
            html_escape(&format_timestamp(c.author.timestamp)),
            html_escape(&oid.to_hex()),
            html_escape(&oid.to_hex()[..7]),
        ));
    }
    if let Some(next) = next_oid {
        rows.push_str(&format!(
            "<tr id=\"load-more-row\"><td colspan=\"2\" style=\"text-align:center;padding:12px\">\
             <button type=\"button\" class=\"btn\" hx-get=\"/commits/{}?after={}&amp;partial=1\" \
             hx-target=\"#load-more-row\" hx-swap=\"outerHTML\">Load more commits</button></td></tr>",
            html_escape(rev),
            html_escape(&next)
        ));
    }
    if partial {
        return render_partial(200, &rows);
    }

    let body = format!(
        "<div class=\"card\"><div class=\"card-header\"><strong>Commits on {}</strong>\
         <span class=\"muted\">HTMX pagination enabled</span></div>\
         <table class=\"tree-table\">{rows}</table></div>",
        html_escape(rev)
    );
    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!("Commits ({rev}) — {}", ctx.name),
        &body,
    )
}

/// Render `/branches` and `/tags`.
pub fn render_branches(ctx: &RepoContext) -> HttpResponse {
    let mut rows = String::new();
    for (b, oid) in &ctx.branches {
        let def = if b == &ctx.default_branch {
            " <span class=\"badge badge-green\">default</span>"
        } else {
            ""
        };
        rows.push_str(&format!(
            "<tr><td><a href=\"/tree/{}\"><strong>{}</strong></a>{def}</td>\
             <td class=\"mono\" style=\"text-align:right\"><a href=\"/commit/{}\">{}</a> \
             <a class=\"btn\" style=\"margin-left:8px\" href=\"/compare/{}..{}\">Compare</a></td></tr>",
            html_escape(b),
            html_escape(b),
            html_escape(&oid.to_hex()),
            html_escape(&oid.to_hex()[..7]),
            html_escape(&ctx.default_branch),
            html_escape(b),
        ));
    }
    let body = format!(
        "<div class=\"card\"><div class=\"card-header\"><strong>Branches ({})</strong></div>\
         <table class=\"tree-table\">{rows}</table></div>",
        ctx.branches.len()
    );
    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!("Branches — {}", ctx.name),
        &body,
    )
}

pub fn render_tags(ctx: &RepoContext) -> HttpResponse {
    let mut rows = String::new();
    for (t, oid) in &ctx.tags {
        rows.push_str(&format!(
            "<tr><td><a href=\"/tree/{}\"><strong>{}</strong></a></td>\
             <td class=\"mono\" style=\"text-align:right\"><a href=\"/commit/{}\">{}</a></td></tr>",
            html_escape(t),
            html_escape(t),
            html_escape(&oid.to_hex()),
            html_escape(&oid.to_hex()[..7]),
        ));
    }
    if rows.is_empty() {
        rows.push_str("<tr><td colspan=\"2\" class=\"muted\">No tags yet.</td></tr>");
    }
    let body = format!(
        "<div class=\"card\"><div class=\"card-header\"><strong>Tags ({})</strong></div>\
         <table class=\"tree-table\">{rows}</table></div>",
        ctx.tags.len()
    );
    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!("Tags — {}", ctx.name),
        &body,
    )
}

/// Serve `/raw/<rev>/<path>` as raw bytes with appropriate safe Content-Type.
pub fn render_raw(repo: &Repository, rev: &str, path: &str) -> HttpResponse {
    let Some(commit_oid) = resolve_rev_oid(repo, rev) else {
        return render_partial(404, "Not found\n");
    };
    let Some(commit) = super::load_commit(repo, &commit_oid) else {
        return render_partial(404, "Not found\n");
    };
    let Some((_, oid)) = resolve_tree_path(repo, commit.tree, path) else {
        return render_partial(404, "Not found\n");
    };
    let Some(bytes) = super::load_blob(repo, &oid) else {
        return render_partial(404, "Not found\n");
    };
    let ct = match path
        .rsplit('.')
        .next()
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "text/plain; charset=utf-8",
    };
    HttpResponse {
        status: 200,
        content_type: ct.into(),
        body: bytes,
        www_authenticate: None,
        extra_headers: vec![("X-Content-Type-Options".into(), "nosniff".into())],
    }
}

fn parse_lfs_pointer(bytes: &[u8]) -> Option<(String, u64)> {
    if bytes.len() > 1024 {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    if !text.starts_with("version https://git-lfs.github.com/spec/v1") {
        return None;
    }
    let mut oid = None;
    let mut size = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("oid sha256:") {
            oid = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("size ") {
            size = rest.trim().parse::<u64>().ok();
        }
    }
    Some((oid?, size?))
}

fn is_image_path(path: &str) -> bool {
    matches!(
        path.rsplit('.')
            .next()
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp" | "gif")
    )
}

fn url_decode_basic(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(val as char);
                i += 3;
                continue;
            }
            out.push('%');
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Safe, XSS-proof server-side Markdown renderer (headings, fenced code, lists, paragraphs).
pub fn render_safe_markdown(input: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    let mut in_list = false;
    for line in input.lines() {
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
            out.push_str(&format!("<h3>{}</h3>\n", html_escape(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            out.push_str(&format!("<h2>{}</h2>\n", html_escape(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            out.push_str(&format!("<h1>{}</h1>\n", html_escape(rest)));
        } else if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            if !in_list {
                out.push_str("<ul>\n");
                in_list = true;
            }
            out.push_str(&format!("<li>{}</li>\n", html_escape(rest)));
        } else {
            if in_list {
                out.push_str("</ul>\n");
                in_list = false;
            }
            out.push_str(&format!("<p>{}</p>\n", html_escape(trimmed)));
        }
    }
    if in_list {
        out.push_str("</ul>\n");
    }
    if in_code {
        out.push_str("</code></pre>\n");
    }
    out
}
