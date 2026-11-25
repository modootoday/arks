//! Commit pages: `commit`, `commitdiff`, and the plain-text
//! `commitdiff_plain` and `patch` (a mail `git am` can apply).

use super::html::{esc, href, link, table};
use super::model::{Change, Ctx};
use super::params::Params;
use super::project::{date, person};
use super::{not_found, page, text};
use crate::cmd::unified_diff::{StatRow, stat_change, write_file_patch_ctx, write_stat};
use arks_object::Commit;
use arks_oid::Oid;
use arks_server::HttpResponse;

/// The commit `h` names (default HEAD) and its first parent.
fn target(ctx: &Ctx, p: &Params) -> Option<(Oid, Commit, Option<Commit>)> {
    let rev = p.hash.as_deref().unwrap_or("HEAD");
    let (oid, c) = ctx.resolve_commit(rev)?;
    let parent = c.parents.first().and_then(|o| ctx.commit(o));
    Some((oid, c, parent))
}

fn blob_bytes(ctx: &Ctx, oid: Option<Oid>) -> Option<Vec<u8>> {
    oid.map(|o| ctx.blob(&o).unwrap_or_default())
}

/// The unified diff of `changes` (`diff --git` per path, three lines of
/// context).
fn diff_text(ctx: &Ctx, changes: &[Change]) -> Vec<u8> {
    let mut out = Vec::new();
    for ch in changes {
        let (a, b) = (blob_bytes(ctx, ch.old), blob_bytes(ctx, ch.new));
        let _ = write_file_patch_ctx(
            &mut out,
            ctx.repo.object_format(),
            &ch.path,
            a.as_deref(),
            b.as_deref(),
            3,
        );
    }
    out
}

fn stat_text(ctx: &Ctx, changes: &[Change]) -> Vec<u8> {
    let rows: Vec<StatRow> = changes
        .iter()
        .map(|ch| {
            let (a, b) = (blob_bytes(ctx, ch.old), blob_bytes(ctx, ch.new));
            StatRow {
                path: ch.path.clone(),
                change: stat_change(a.as_deref(), b.as_deref()),
            }
        })
        .collect();
    let mut out = Vec::new();
    let _ = write_stat(&mut out, &rows);
    out
}

fn status(ch: &Change) -> &'static str {
    match (ch.old, ch.new) {
        (None, _) => "new file",
        (_, None) => "deleted file",
        _ => "modified",
    }
}

/// The header table every commit page shares.
fn header(ctx: &Ctx, oid: &Oid, c: &Commit) -> String {
    let pn = Some(ctx.name.as_str());
    let hex = oid.to_hex();
    let tree = c.tree.to_hex();
    let mut facts = vec![
        vec!["author".into(), person(&c.author)],
        vec!["".into(), esc(&date(&c.author, "rfc2822"))],
        vec!["committer".into(), person(&c.committer)],
        vec!["".into(), esc(&date(&c.committer, "rfc2822"))],
        vec!["commit".into(), format!("<code>{hex}</code>")],
        vec![
            "tree".into(),
            format!(
                "{} ({})",
                link(
                    &href(pn, &[("a", "tree"), ("h", &tree), ("hb", &hex)]),
                    &tree
                ),
                link(&href(pn, &[("a", "tree"), ("hb", &hex)]), "tree")
            ),
        ],
    ];
    for parent in &c.parents {
        let ph = parent.to_hex();
        facts.push(vec![
            "parent".into(),
            format!(
                "{} ({})",
                link(&href(pn, &[("a", "commit"), ("h", &ph)]), &ph),
                link(&href(pn, &[("a", "commitdiff"), ("h", &hex)]), "diff")
            ),
        ]);
    }
    format!(
        "<h2 class=\"subject\">{}</h2>\n{}",
        esc(c.subject()),
        table("object_header", &["", ""], &facts)
    )
}

pub fn commit(ctx: &Ctx, p: &Params) -> HttpResponse {
    let Some((oid, c, parent)) = target(ctx, p) else {
        return not_found(ctx, "Unknown commit object");
    };
    let hex = oid.to_hex();
    let pn = Some(ctx.name.as_str());
    let changes = ctx.changes(parent.as_ref(), &c);
    let rows: Vec<Vec<String>> = changes
        .iter()
        .map(|ch| {
            let file = link(
                &href(pn, &[("a", "blob"), ("hb", &hex), ("f", &ch.path)]),
                &ch.path,
            );
            let name = if ch.new.is_some() {
                file
            } else {
                esc(&ch.path)
            };
            let mut links = vec![link(&href(pn, &[("a", "commitdiff"), ("h", &hex)]), "diff")];
            if ch.new.is_some() {
                links.push(link(
                    &href(pn, &[("a", "blob"), ("hb", &hex), ("f", &ch.path)]),
                    "blob",
                ));
            }
            vec![name, status(ch).into(), links.join(" | ")]
        })
        .collect();
    let body = format!(
        "{}<pre class=\"message\">{}</pre>\n{}",
        header(ctx, &oid, &c),
        esc(c.message.trim_end()),
        table("diff_tree", &["File", "Change", ""], &rows)
    );
    page(ctx, "commit", Some(&hex), &body)
}

pub fn commitdiff(ctx: &Ctx, p: &Params) -> HttpResponse {
    let Some((oid, c, parent)) = target(ctx, p) else {
        return not_found(ctx, "Unknown commit object");
    };
    let hex = oid.to_hex();
    let changes = ctx.changes(parent.as_ref(), &c);
    let diff = String::from_utf8_lossy(&diff_text(ctx, &changes)).into_owned();
    let mut lines = String::new();
    for l in diff.lines() {
        let class = if l.starts_with("diff --git") {
            "head"
        } else if l.starts_with("@@") {
            "hunk"
        } else if l.starts_with("+++") || l.starts_with("---") || l.starts_with("index ") {
            "head"
        } else if l.starts_with('+') {
            "add"
        } else if l.starts_with('-') {
            "del"
        } else {
            "ctx"
        };
        lines.push_str(&format!("<div class=\"{class}\">{}</div>", esc(l)));
        lines.push('\n');
    }
    let body = format!(
        "{}<pre class=\"message\">{}</pre>\n<p>{}</p>\n<pre class=\"diff\">\n{lines}</pre>\n",
        header(ctx, &oid, &c),
        esc(c.message.trim_end()),
        link(
            &href(Some(&ctx.name), &[("a", "commitdiff_plain"), ("h", &hex)]),
            "raw"
        ),
    );
    page(ctx, "commitdiff", Some(&hex), &body)
}

pub fn commitdiff_plain(ctx: &Ctx, p: &Params) -> HttpResponse {
    let Some((oid, c, parent)) = target(ctx, p) else {
        return not_found(ctx, "Unknown commit object");
    };
    let changes = ctx.changes(parent.as_ref(), &c);
    let mut out = format!(
        "From: {} <{}>\nDate: {}\nSubject: {}\nX-Git-Commit: {}\n\n{}\n---\n\n",
        c.author.name,
        c.author.email,
        date(&c.author, "rfc2822"),
        c.subject(),
        oid.to_hex(),
        c.message.trim_end()
    )
    .into_bytes();
    out.extend(diff_text(ctx, &changes));
    text(200, "text/plain; charset=utf-8", out)
}

pub fn patch(ctx: &Ctx, p: &Params) -> HttpResponse {
    let Some((oid, c, parent)) = target(ctx, p) else {
        return not_found(ctx, "Unknown commit object");
    };
    let changes = ctx.changes(parent.as_ref(), &c);
    let (subject, body) = c
        .message
        .split_once('\n')
        .map(|(s, b)| (s.trim_end(), b.trim()))
        .unwrap_or((c.message.trim_end(), ""));
    let mut out = format!(
        "From {} Mon Sep 17 00:00:00 2001\nFrom: {} <{}>\nDate: {}\nSubject: [PATCH] {}\n\n",
        oid.to_hex(),
        c.author.name,
        c.author.email,
        date(&c.author, "rfc2822"),
        subject
    )
    .into_bytes();
    if !body.is_empty() {
        out.extend(format!("{body}\n").into_bytes());
    }
    out.extend(b"---\n");
    out.extend(stat_text(ctx, &changes));
    out.extend(b"\n");
    out.extend(diff_text(ctx, &changes));
    out.extend(b"-- \narks\n\n");
    text(200, "text/plain; charset=utf-8", out)
}
