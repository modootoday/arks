//! Project-level pages: the project list, summary, shortlog, log, heads,
//! tags and the annotated tag view.

use super::html::{esc, href, link, table};
use super::model::{Ctx, RefRow, parse_tag};
use super::params::Params;
use super::{PAGE, SUMMARY_ROWS, error, not_found, page, text};
use arks_object::{Commit, ObjectKind, Signature};
use arks_oid::Oid;
use arks_server::HttpResponse;
use std::collections::HashMap;

/// A signature date in git's `--date=<mode>` shape.
pub(super) fn date(sig: &Signature, mode: &str) -> String {
    crate::cmd::date_spec::format(mode, sig).unwrap_or_default()
}

/// `name <email>` escaped.
pub(super) fn person(sig: &Signature) -> String {
    esc(&format!("{} <{}>", sig.name, sig.email))
}

pub fn project_list(ctx: &Ctx) -> HttpResponse {
    let p = ctx.name.as_str();
    let last = ctx
        .last_change()
        .map(|s| date(&s, "relative"))
        .unwrap_or_default();
    let row = vec![
        link(&href(Some(p), &[("a", "summary")]), p),
        esc(&ctx.description()),
        esc(&ctx.owner().unwrap_or_default()),
        esc(&last),
        [
            ("summary", "summary"),
            ("shortlog", "shortlog"),
            ("log", "log"),
            ("tree", "tree"),
        ]
        .iter()
        .map(|(a, t)| link(&href(Some(p), &[("a", a)]), t))
        .collect::<Vec<_>>()
        .join(" | "),
    ];
    let body = table(
        "project_list",
        &["Project", "Description", "Owner", "Last Change", ""],
        &[row],
    );
    let doc = super::html::frame(None, "project_list", "", &body);
    text(200, "text/html; charset=utf-8", doc.into_bytes())
}

pub fn summary(ctx: &Ctx) -> HttpResponse {
    let mut facts = vec![vec!["description".into(), esc(&ctx.description())]];
    if let Some(o) = ctx.owner() {
        facts.push(vec!["owner".into(), esc(&o)]);
    }
    if let Some(s) = ctx.last_change() {
        facts.push(vec!["last change".into(), esc(&date(&s, "rfc2822"))]);
    }
    for (i, u) in ctx.clone_urls().iter().enumerate() {
        let label = if i == 0 { "URL" } else { "" };
        facts.push(vec![label.into(), esc(u)]);
    }
    let mut body = table("projects_list", &["", ""], &facts);
    if let Some((tip, _)) = ctx.resolve_commit("HEAD") {
        let commits = ctx.walk(tip, 0, SUMMARY_ROWS + 1);
        body.push_str(&section(ctx, "shortlog", &[("a", "shortlog")]));
        body.push_str(&shortlog_table(
            ctx,
            &commits[..commits.len().min(SUMMARY_ROWS)],
        ));
        if commits.len() > SUMMARY_ROWS {
            body.push_str(&more(ctx, "shortlog"));
        }
    }
    let tags = ctx.refs("tags");
    if !tags.is_empty() {
        body.push_str(&section(ctx, "tags", &[("a", "tags")]));
        body.push_str(&tags_table(ctx, &tags[..tags.len().min(SUMMARY_ROWS)]));
        if tags.len() > SUMMARY_ROWS {
            body.push_str(&more(ctx, "tags"));
        }
    }
    let heads = ctx.refs("heads");
    if !heads.is_empty() {
        body.push_str(&section(ctx, "heads", &[("a", "heads")]));
        body.push_str(&heads_table(ctx, &heads[..heads.len().min(SUMMARY_ROWS)]));
        if heads.len() > SUMMARY_ROWS {
            body.push_str(&more(ctx, "heads"));
        }
    }
    page(ctx, "summary", None, &body)
}

fn section(ctx: &Ctx, title: &str, pairs: &[(&str, &str)]) -> String {
    format!("<h2>{}</h2>\n", link(&href(Some(&ctx.name), pairs), title))
}

fn more(ctx: &Ctx, action: &str) -> String {
    format!(
        "<p>{}</p>\n",
        link(&href(Some(&ctx.name), &[("a", action)]), "...")
    )
}

/// The commit a `h` parameter names (default HEAD), or an error page.
fn tip(ctx: &Ctx, p: &Params) -> Result<(String, Oid), HttpResponse> {
    let rev = p.hash.clone().unwrap_or_else(|| "HEAD".into());
    match ctx.resolve_commit(&rev) {
        Some((oid, _)) => Ok((rev, oid)),
        None => Err(not_found(ctx, "Unknown commit object")),
    }
}

/// Page navigation below a shortlog or log.
fn pager(ctx: &Ctx, action: &str, rev: &str, pg: usize, more: bool) -> String {
    let mut links = Vec::new();
    let pg_s = |n: usize| n.to_string();
    if pg > 0 {
        let prev = pg_s(pg - 1);
        links.push(link(
            &href(Some(&ctx.name), &[("a", action), ("h", rev), ("pg", &prev)]),
            "prev",
        ));
    }
    if more {
        let next = pg_s(pg + 1);
        links.push(link(
            &href(Some(&ctx.name), &[("a", action), ("h", rev), ("pg", &next)]),
            "next",
        ));
    }
    format!("<p class=\"pager\">{}</p>\n", links.join(" | "))
}

pub fn shortlog(ctx: &Ctx, p: &Params) -> HttpResponse {
    let (rev, oid) = match tip(ctx, p) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut commits = ctx.walk(oid, p.page * PAGE, PAGE + 1);
    let more = commits.len() > PAGE;
    commits.truncate(PAGE);
    let mut body = shortlog_table(ctx, &commits);
    body.push_str(&pager(ctx, "shortlog", &rev, p.page, more));
    page(ctx, "shortlog", Some(&rev), &body)
}

pub fn log(ctx: &Ctx, p: &Params) -> HttpResponse {
    let (rev, oid) = match tip(ctx, p) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let mut commits = ctx.walk(oid, p.page * PAGE, PAGE + 1);
    let more = commits.len() > PAGE;
    commits.truncate(PAGE);
    let decor = decorations(ctx);
    let mut body = String::new();
    for (oid, c) in &commits {
        let hex = oid.to_hex();
        body.push_str(&format!(
            "<div class=\"log_entry\" id=\"{hex}\">\n<h3>{}{}</h3>\n\
             <div class=\"log_meta\">{} [{}]</div>\n<div class=\"log_links\">{} | {} | {}</div>\n\
             <pre class=\"log_body\">{}</pre>\n</div>\n",
            link(
                &href(Some(&ctx.name), &[("a", "commit"), ("h", &hex)]),
                c.subject()
            ),
            decor
                .get(oid)
                .map(|d| refs_html(ctx, d))
                .unwrap_or_default(),
            person(&c.author),
            esc(&date(&c.author, "rfc2822")),
            link(
                &href(Some(&ctx.name), &[("a", "commit"), ("h", &hex)]),
                "commit"
            ),
            link(
                &href(Some(&ctx.name), &[("a", "commitdiff"), ("h", &hex)]),
                "commitdiff"
            ),
            link(
                &href(Some(&ctx.name), &[("a", "tree"), ("hb", &hex)]),
                "tree"
            ),
            esc(c.message.trim_end()),
        ));
    }
    body.push_str(&pager(ctx, "log", &rev, p.page, more));
    page(ctx, "log", Some(&rev), &body)
}

/// Ref names by the commit they peel to.
pub(super) fn decorations(ctx: &Ctx) -> HashMap<Oid, Vec<(String, String)>> {
    let mut map: HashMap<Oid, Vec<(String, String)>> = HashMap::new();
    for kind in ["heads", "tags"] {
        for r in ctx.refs(kind) {
            if let Some((c, _)) = r.commit {
                map.entry(c).or_default().push((kind.to_string(), r.name));
            }
        }
    }
    map
}

fn refs_html(ctx: &Ctx, refs: &[(String, String)]) -> String {
    refs.iter()
        .map(|(kind, name)| {
            let full = format!("refs/{kind}/{name}");
            format!(
                " <span class=\"ref {kind}\">{}</span>",
                link(
                    &href(Some(&ctx.name), &[("a", "shortlog"), ("h", &full)]),
                    name
                )
            )
        })
        .collect()
}

/// One row per commit: date, author, subject (with refs) and links.
pub(super) fn shortlog_table(ctx: &Ctx, commits: &[(Oid, Commit)]) -> String {
    let decor = decorations(ctx);
    let rows: Vec<Vec<String>> = commits
        .iter()
        .map(|(oid, c)| {
            let hex = oid.to_hex();
            let pn = Some(ctx.name.as_str());
            vec![
                format!(
                    "<span title=\"{}\">{}</span>",
                    esc(&date(&c.committer, "iso")),
                    esc(&date(&c.committer, "short"))
                ),
                esc(&c.author.name),
                format!(
                    "{}{}",
                    link(&href(pn, &[("a", "commit"), ("h", &hex)]), c.subject()),
                    decor
                        .get(oid)
                        .map(|d| refs_html(ctx, d))
                        .unwrap_or_default()
                ),
                format!(
                    "{} | {} | {}",
                    link(&href(pn, &[("a", "commit"), ("h", &hex)]), "commit"),
                    link(&href(pn, &[("a", "commitdiff"), ("h", &hex)]), "commitdiff"),
                    link(&href(pn, &[("a", "tree"), ("hb", &hex)]), "tree"),
                ),
            ]
        })
        .collect();
    table("shortlog", &["Date", "Author", "Subject", ""], &rows)
}

fn heads_table(ctx: &Ctx, heads: &[RefRow]) -> String {
    let pn = Some(ctx.name.as_str());
    let rows: Vec<Vec<String>> = heads
        .iter()
        .map(|r| {
            let full = format!("refs/heads/{}", r.name);
            vec![
                r.commit
                    .as_ref()
                    .map(|(_, c)| esc(&date(&c.committer, "short")))
                    .unwrap_or_default(),
                link(&href(pn, &[("a", "shortlog"), ("h", &full)]), &r.name),
                format!(
                    "{} | {} | {}",
                    link(&href(pn, &[("a", "shortlog"), ("h", &full)]), "shortlog"),
                    link(&href(pn, &[("a", "log"), ("h", &full)]), "log"),
                    link(&href(pn, &[("a", "tree"), ("hb", &full)]), "tree"),
                ),
            ]
        })
        .collect();
    table("heads", &["Date", "Head", ""], &rows)
}

fn tags_table(ctx: &Ctx, tags: &[RefRow]) -> String {
    let pn = Some(ctx.name.as_str());
    let rows: Vec<Vec<String>> = tags
        .iter()
        .map(|r| {
            let full = format!("refs/tags/{}", r.name);
            let target = if r.annotated {
                link(&href(pn, &[("a", "tag"), ("h", &r.oid.to_hex())]), &r.name)
            } else {
                link(&href(pn, &[("a", "commit"), ("h", &full)]), &r.name)
            };
            let mut links = Vec::new();
            if r.annotated {
                links.push(link(
                    &href(pn, &[("a", "tag"), ("h", &r.oid.to_hex())]),
                    "tag",
                ));
            }
            if r.commit.is_some() {
                links.push(link(&href(pn, &[("a", "commit"), ("h", &full)]), "commit"));
                links.push(link(
                    &href(pn, &[("a", "shortlog"), ("h", &full)]),
                    "shortlog",
                ));
                links.push(link(&href(pn, &[("a", "log"), ("h", &full)]), "log"));
            }
            vec![
                r.commit
                    .as_ref()
                    .map(|(_, c)| esc(&date(&c.committer, "short")))
                    .unwrap_or_default(),
                target,
                esc(r.tag_subject.as_deref().unwrap_or("")),
                links.join(" | "),
            ]
        })
        .collect();
    table("tags", &["Date", "Tag", "Message", ""], &rows)
}

pub fn heads(ctx: &Ctx) -> HttpResponse {
    page(ctx, "heads", None, &heads_table(ctx, &ctx.refs("heads")))
}

pub fn tags(ctx: &Ctx) -> HttpResponse {
    page(ctx, "tags", None, &tags_table(ctx, &ctx.refs("tags")))
}

pub fn tag(ctx: &Ctx, p: &Params) -> HttpResponse {
    let Some(oid) = p.hash.as_deref().and_then(|h| ctx.resolve(h)) else {
        return not_found(ctx, "Unknown tag object");
    };
    let Ok(obj) = ctx.repo.get_object(&oid) else {
        return not_found(ctx, "Unknown tag object");
    };
    if obj.kind != ObjectKind::Tag {
        return error(Some(ctx), 404, "Expected a tag object");
    }
    let t = parse_tag(&obj.payload);
    let pn = Some(ctx.name.as_str());
    let mut facts = Vec::new();
    if let Some(o) = t.object {
        let action = match t.kind.as_str() {
            "commit" => "commit",
            "tree" => "tree",
            "tag" => "tag",
            _ => "blob",
        };
        facts.push(vec![
            "object".into(),
            link(&href(pn, &[("a", action), ("h", &o.to_hex())]), &o.to_hex()),
        ]);
    }
    facts.push(vec!["type".into(), esc(&t.kind)]);
    facts.push(vec!["tag".into(), esc(&t.name)]);
    if let Some(s) = &t.tagger {
        facts.push(vec!["tagger".into(), person(s)]);
        facts.push(vec!["".into(), esc(&date(s, "rfc2822"))]);
    }
    let body = format!(
        "<h2>{}</h2>\n{}<pre class=\"tag_message\">{}</pre>\n",
        esc(&t.name),
        table("object_header", &["", ""], &facts),
        esc(t.message.trim_end())
    );
    page(ctx, "tag", None, &body)
}
