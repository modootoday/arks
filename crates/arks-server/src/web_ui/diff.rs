//! Commit diff and branch/tag compare views with GitHub-exact dual line numbers
//! (Unified + Split SSR fallback) and vendored `diff2html` progressive enhancement.

use super::{ActiveTab, RepoContext, format_timestamp, html_escape, render_page, resolve_rev_oid};
use crate::http::HttpResponse;
use arks_object::TreeMode;
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::BTreeMap;

struct FileDiff {
    path: String,
    status: &'static str,
    adds: usize,
    dels: usize,
    rows: Vec<DiffRow>,
    raw_patch: String,
}

struct DiffRow {
    old_ln: Option<usize>,
    new_ln: Option<usize>,
    kind: DiffKind,
    text: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DiffKind {
    Context,
    Add,
    Del,
    Hunk,
}

fn flatten_tree(repo: &Repository, tree_oid: Oid, prefix: &str, out: &mut BTreeMap<String, Oid>) {
    let Some(tree) = super::load_tree(repo, &tree_oid) else {
        return;
    };
    for e in tree.entries {
        let full = if prefix.is_empty() {
            e.name
        } else {
            format!("{prefix}/{}", e.name)
        };
        if e.mode == TreeMode::Tree {
            flatten_tree(repo, e.oid, &full, out);
        } else {
            out.insert(full, e.oid);
        }
    }
}

fn read_text_lines(repo: &Repository, oid: Oid) -> Option<Vec<String>> {
    let raw = super::load_blob(repo, &oid)?;
    if raw.len() > 256 * 1024 || raw.contains(&0) {
        return None;
    }
    let s = String::from_utf8_lossy(&raw);
    Some(s.lines().map(ToString::to_string).collect())
}

fn diff_two_trees(repo: &Repository, base_tree: Option<Oid>, head_tree: Oid) -> Vec<FileDiff> {
    let mut base_map = BTreeMap::new();
    if let Some(bt) = base_tree {
        flatten_tree(repo, bt, "", &mut base_map);
    }
    let mut head_map = BTreeMap::new();
    flatten_tree(repo, head_tree, "", &mut head_map);

    let mut all_paths: Vec<String> = base_map.keys().chain(head_map.keys()).cloned().collect();
    all_paths.sort();
    all_paths.dedup();

    let mut diffs = Vec::new();
    for path in all_paths.into_iter().take(64) {
        let b = base_map.get(&path).copied();
        let h = head_map.get(&path).copied();
        if b == h {
            continue;
        }
        let status = match (b, h) {
            (None, Some(_)) => "added",
            (Some(_), None) => "deleted",
            _ => "modified",
        };
        let old_lines = b.and_then(|o| read_text_lines(repo, o)).unwrap_or_default();
        let new_lines = h.and_then(|o| read_text_lines(repo, o)).unwrap_or_default();
        diffs.push(build_file_diff(path, status, &old_lines, &new_lines));
    }
    diffs
}

fn build_file_diff(
    path: String,
    status: &'static str,
    old_lines: &[String],
    new_lines: &[String],
) -> FileDiff {
    let mut rows = Vec::new();
    let mut raw = format!("diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n");
    let mut adds = 0usize;
    let mut dels = 0usize;

    let hunk = format!("@@ -1,{} +1,{} @@", old_lines.len(), new_lines.len());
    raw.push_str(&hunk);
    raw.push('\n');
    rows.push(DiffRow {
        old_ln: None,
        new_ln: None,
        kind: DiffKind::Hunk,
        text: hunk,
    });

    // Simple prefix/suffix trimming + line-by-line delta (fast, bounded, exact line numbers)
    let mut pref = 0;
    while pref < old_lines.len() && pref < new_lines.len() && old_lines[pref] == new_lines[pref] {
        pref += 1;
    }
    let mut suff = 0;
    while suff + pref < old_lines.len()
        && suff + pref < new_lines.len()
        && old_lines[old_lines.len() - 1 - suff] == new_lines[new_lines.len() - 1 - suff]
    {
        suff += 1;
    }

    // Context before change (up to 3 lines)
    let ctx_start = pref.saturating_sub(3);
    for (idx, line) in old_lines.iter().enumerate().take(pref).skip(ctx_start) {
        raw.push(' ');
        raw.push_str(line);
        raw.push('\n');
        rows.push(DiffRow {
            old_ln: Some(idx + 1),
            new_ln: Some(idx + 1),
            kind: DiffKind::Context,
            text: line.clone(),
        });
    }
    // Deleted lines
    for (idx, line) in old_lines[pref..old_lines.len() - suff].iter().enumerate() {
        dels += 1;
        raw.push('-');
        raw.push_str(line);
        raw.push('\n');
        rows.push(DiffRow {
            old_ln: Some(pref + idx + 1),
            new_ln: None,
            kind: DiffKind::Del,
            text: line.clone(),
        });
    }
    // Added lines
    for (idx, line) in new_lines[pref..new_lines.len() - suff].iter().enumerate() {
        adds += 1;
        raw.push('+');
        raw.push_str(line);
        raw.push('\n');
        rows.push(DiffRow {
            old_ln: None,
            new_ln: Some(pref + idx + 1),
            kind: DiffKind::Add,
            text: line.clone(),
        });
    }
    // Context after change (up to 3 lines)
    let tail_old_start = old_lines.len() - suff;
    let tail_new_start = new_lines.len() - suff;
    for k in 0..suff.min(3) {
        let line = &old_lines[tail_old_start + k];
        raw.push(' ');
        raw.push_str(line);
        raw.push('\n');
        rows.push(DiffRow {
            old_ln: Some(tail_old_start + k + 1),
            new_ln: Some(tail_new_start + k + 1),
            kind: DiffKind::Context,
            text: line.clone(),
        });
    }

    FileDiff {
        path,
        status,
        adds,
        dels,
        rows,
        raw_patch: raw,
    }
}

fn render_diff_section(diffs: &[FileDiff]) -> String {
    let total_adds: usize = diffs.iter().map(|d| d.adds).sum();
    let total_dels: usize = diffs.iter().map(|d| d.dels).sum();
    let mut combined_raw = String::new();
    for d in diffs {
        combined_raw.push_str(&d.raw_patch);
    }

    let mut html = format!(
        "<div class=\"toolbar\"><div class=\"toolbar-group\">\
         <strong>{} changed file(s)</strong>\
         <span class=\"badge badge-green\">+{}</span>\
         <span class=\"badge\" style=\"color:var(--red)\">-{}</span></div>\
         <div class=\"btn-group\">\
           <button type=\"button\" class=\"btn active\" data-diff-mode=\"line-by-line\">Unified</button>\
           <button type=\"button\" class=\"btn\" data-diff-mode=\"side-by-side\">Split</button>\
         </div></div>\n\
         <script type=\"text/plain\" id=\"raw-diff-payload\">{}</script>\n\
         <div id=\"diff2html-mount\" data-diff2html-raw=\"raw-diff-payload\"></div>\n",
        diffs.len(),
        total_adds,
        total_dels,
        html_escape(&combined_raw),
    );

    // Dual-line-number SSR Unified & Split fallback (works even with JS disabled)
    html.push_str("<div id=\"ssr-diff-unified\">");
    for d in diffs {
        html.push_str(&format!(
            "<div class=\"card\"><div class=\"card-header\">\
             <span class=\"mono\"><strong>{}</strong> <span class=\"badge\">{}</span></span>\
             <span class=\"mono muted\">+{} -{}</span></div>\
             <div class=\"code-wrap\"><table class=\"diff-table\">",
            html_escape(&d.path),
            d.status,
            d.adds,
            d.dels
        ));
        for r in &d.rows {
            let cls = match r.kind {
                DiffKind::Add => " class=\"diff-add\"",
                DiffKind::Del => " class=\"diff-del\"",
                DiffKind::Hunk => " class=\"diff-hunk\"",
                DiffKind::Context => "",
            };
            let prefix = match r.kind {
                DiffKind::Add => "+",
                DiffKind::Del => "-",
                DiffKind::Context => " ",
                DiffKind::Hunk => "",
            };
            let old_s = r.old_ln.map(|n| n.to_string()).unwrap_or_default();
            let new_s = r.new_ln.map(|n| n.to_string()).unwrap_or_default();
            html.push_str(&format!(
                "<tr{cls}><td class=\"diff-ln\">{old_s}</td><td class=\"diff-ln\">{new_s}</td>\
                 <td class=\"diff-code\">{prefix}{}</td></tr>",
                html_escape(&r.text)
            ));
        }
        html.push_str("</table></div></div>");
    }
    html.push_str("</div>");

    html.push_str("<div id=\"ssr-diff-split\" hidden>");
    for d in diffs {
        html.push_str(&format!(
            "<div class=\"card\"><div class=\"card-header\">\
             <span class=\"mono\"><strong>{}</strong></span>\
             <span class=\"mono muted\">+{} -{}</span></div>\
             <div class=\"diff-split-grid\"><div><table class=\"diff-table\">",
            html_escape(&d.path),
            d.adds,
            d.dels
        ));
        for r in &d.rows {
            if r.kind != DiffKind::Add {
                let cls = match r.kind {
                    DiffKind::Del => " class=\"diff-del\"",
                    DiffKind::Hunk => " class=\"diff-hunk\"",
                    _ => "",
                };
                let old_s = r.old_ln.map(|n| n.to_string()).unwrap_or_default();
                html.push_str(&format!(
                    "<tr{cls}><td class=\"diff-ln\">{old_s}</td><td class=\"diff-code\">{}</td></tr>",
                    html_escape(&r.text)
                ));
            }
        }
        html.push_str("</table></div><div><table class=\"diff-table\">");
        for r in &d.rows {
            if r.kind != DiffKind::Del {
                let cls = match r.kind {
                    DiffKind::Add => " class=\"diff-add\"",
                    DiffKind::Hunk => " class=\"diff-hunk\"",
                    _ => "",
                };
                let new_s = r.new_ln.map(|n| n.to_string()).unwrap_or_default();
                html.push_str(&format!(
                    "<tr{cls}><td class=\"diff-ln\">{new_s}</td><td class=\"diff-code\">{}</td></tr>",
                    html_escape(&r.text)
                ));
            }
        }
        html.push_str("</table></div></div></div>");
    }
    html.push_str("</div>");
    html
}

/// Render `/commit/<oid>` with commit metadata, signature badge, and dual-line diff.
pub fn render_commit(repo: &Repository, ctx: &RepoContext, oid_str: &str) -> HttpResponse {
    let Some(oid) = resolve_rev_oid(repo, oid_str) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Commit not found",
            "<div class=\"card\"><div class=\"card-body\">Commit not found.</div></div>",
        );
    };
    let Some(commit) = super::load_commit(repo, &oid) else {
        return render_page(
            404,
            ctx,
            ActiveTab::Code,
            "Not a commit",
            "<div class=\"card\"><div class=\"card-body\">Object is not a commit.</div></div>",
        );
    };

    let parent_tree = commit
        .parents
        .first()
        .and_then(|p| super::load_commit(repo, p).map(|c| c.tree));
    let diffs = diff_two_trees(repo, parent_tree, commit.tree);
    let has_sig = super::commit_has_sig(&commit);
    let sig_badge = if has_sig {
        "<span class=\"badge badge-green\">Signed (gpgsig)</span>"
    } else {
        "<span class=\"badge\">Unsigned</span>"
    };

    let subject = commit.message.lines().next().unwrap_or("(no message)");
    let body = format!(
        "<div class=\"card\"><div class=\"card-header\">\
         <div><strong>{}</strong> {}</div>\
         <span class=\"mono muted\">{}</span></div>\
         <div class=\"card-body\">\
           <div class=\"muted\">Author: <strong>{}</strong> &lt;{}&gt; · {}</div>\
           <pre style=\"margin:12px 0 0;white-space:pre-wrap\">{}</pre>\
         </div></div>\n{}",
        html_escape(subject),
        sig_badge,
        html_escape(&oid.to_hex()),
        html_escape(&commit.author.name),
        html_escape(&commit.author.email),
        html_escape(&format_timestamp(commit.author.timestamp)),
        html_escape(&commit.message),
        render_diff_section(&diffs)
    );
    render_page(
        200,
        ctx,
        ActiveTab::Code,
        &format!("Commit {} — {}", &oid.to_hex()[..7], ctx.name),
        &body,
    )
}

/// Render `/compare` or `/compare/<base>..<head>` comparing two branches/tags/commits.
pub fn render_compare(repo: &Repository, ctx: &RepoContext, spec: &str) -> HttpResponse {
    let (base_ref, head_ref) = if let Some((b, h)) = spec.split_once("..") {
        (b.trim(), h.trim())
    } else if ctx.branches.len() >= 2 {
        (ctx.branches[0].0.as_str(), ctx.branches[1].0.as_str())
    } else {
        (ctx.default_branch.as_str(), ctx.default_branch.as_str())
    };

    let base_oid = resolve_rev_oid(repo, base_ref);
    let head_oid = resolve_rev_oid(repo, head_ref);

    let mut header = format!(
        "<div class=\"card\"><div class=\"card-header\">\
         <div><strong>Compare</strong> <span class=\"badge mono\">base: {}</span> \
         <span class=\"muted\">←</span> \
         <span class=\"badge badge-green mono\">head: {}</span></div>\
         <span class=\"muted\">URL: <code>/compare/&lt;base&gt;..&lt;head&gt;</code></span>\
         </div></div>",
        html_escape(base_ref),
        html_escape(head_ref),
    );

    let (Some(b_oid), Some(h_oid)) = (base_oid, head_oid) else {
        header.push_str(
            "<div class=\"card\"><div class=\"card-body muted\">Specify two valid refs (e.g. <code>/compare/main..feature</code>).</div></div>",
        );
        return render_page(
            200,
            ctx,
            ActiveTab::Pulls,
            &format!("Compare — {}", ctx.name),
            &header,
        );
    };

    let b_tree = super::load_commit(repo, &b_oid).map(|c| c.tree);
    let h_tree = super::load_commit(repo, &h_oid).map(|c| c.tree);

    let Some(ht) = h_tree else {
        header.push_str("<div class=\"card\"><div class=\"card-body muted\">Head ref is not a commit.</div></div>");
        return render_page(
            200,
            ctx,
            ActiveTab::Pulls,
            &format!("Compare — {}", ctx.name),
            &header,
        );
    };

    let diffs = diff_two_trees(repo, b_tree, ht);
    if diffs.is_empty() {
        header.push_str("<div class=\"card\"><div class=\"card-body muted\">There are no differences between these revisions.</div></div>");
    } else {
        header.push_str(&render_diff_section(&diffs));
    }

    render_page(
        200,
        ctx,
        ActiveTab::Pulls,
        &format!("Compare {base_ref}..{head_ref} — {}", ctx.name),
        &header,
    )
}

/// Render dual-line Unified/Split diff HTML between two commit OIDs for embedding in PR views.
pub(crate) fn render_diff_between_commits(
    repo: &Repository,
    base_oid: Oid,
    head_oid: Oid,
) -> String {
    let b_tree = super::load_commit(repo, &base_oid).map(|c| c.tree);
    let Some(h_tree) = super::load_commit(repo, &head_oid).map(|c| c.tree) else {
        return "<div class=\"card\"><div class=\"card-body muted\">Head commit unavailable.</div></div>".into();
    };
    let diffs = diff_two_trees(repo, b_tree, h_tree);
    if diffs.is_empty() {
        return "<div class=\"card\"><div class=\"card-body muted\">No file differences.</div></div>".into();
    }
    render_diff_section(&diffs)
}
