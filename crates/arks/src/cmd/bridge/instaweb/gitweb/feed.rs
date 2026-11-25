//! Machine-readable views: the `rss` and `atom` feeds of the newest
//! commits on HEAD, and the plain-text `project_index`.

use super::FEED_ITEMS;
use super::html::{esc, href};
use super::model::Ctx;
use super::params::percent_encode;
use super::project::date;
use super::text;
use arks_object::{Commit, Signature};
use arks_oid::Oid;
use arks_server::HttpResponse;

/// Absolute link for feeds (`http://<Host>/…`; relative without a Host).
fn abs(ctx: &Ctx, pairs: &[(&str, &str)]) -> String {
    let rel = href(Some(&ctx.name), pairs);
    match &ctx.host {
        Some(h) => format!("http://{h}{rel}"),
        None => rel,
    }
}

fn utc(sig: &Signature) -> Signature {
    // Shift into UTC so the timestamp shows with a `Z`-equivalent zone.
    Signature {
        timezone: "+0000".into(),
        ..sig.clone()
    }
}

fn recent(ctx: &Ctx) -> Vec<(Oid, Commit)> {
    ctx.resolve_commit("HEAD")
        .map(|(tip, _)| ctx.walk(tip, 0, FEED_ITEMS))
        .unwrap_or_default()
}

/// The commit message and changed paths, for an entry description.
fn entry_text(ctx: &Ctx, c: &Commit) -> String {
    let parent = c.parents.first().and_then(|o| ctx.commit(o));
    let mut s = c.message.trim_end().to_string();
    s.push_str("\n\n");
    for ch in ctx.changes(parent.as_ref(), c) {
        s.push_str(&ch.path);
        s.push('\n');
    }
    s
}

pub fn rss(ctx: &Ctx) -> HttpResponse {
    let mut x = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<rss version=\"2.0\">\n<channel>\n\
         <title>{} log</title>\n<link>{}</link>\n<description>{}</description>\n\
         <generator>arks instaweb</generator>\n",
        esc(&ctx.name),
        esc(&abs(ctx, &[("a", "summary")])),
        esc(&ctx.description()),
    );
    let items = recent(ctx);
    if let Some((_, c)) = items.first() {
        x.push_str(&format!(
            "<lastBuildDate>{}</lastBuildDate>\n",
            esc(&date(&utc(&c.committer), "rfc2822"))
        ));
    }
    for (oid, c) in &items {
        let hex = oid.to_hex();
        x.push_str(&format!(
            "<item>\n<title>{}</title>\n<author>{} ({})</author>\n<pubDate>{}</pubDate>\n\
             <guid isPermaLink=\"false\">{hex}</guid>\n<link>{}</link>\n\
             <description>{}</description>\n</item>\n",
            esc(c.subject()),
            esc(&c.author.email),
            esc(&c.author.name),
            esc(&date(&utc(&c.author), "rfc2822")),
            esc(&abs(ctx, &[("a", "commit"), ("h", &hex)])),
            esc(&entry_text(ctx, c)),
        ));
    }
    x.push_str("</channel>\n</rss>\n");
    text(200, "application/rss+xml; charset=utf-8", x.into_bytes())
}

pub fn atom(ctx: &Ctx) -> HttpResponse {
    let items = recent(ctx);
    let updated = items
        .first()
        .map(|(_, c)| date(&utc(&c.committer), "iso-strict"))
        .unwrap_or_default();
    let mut x = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\">\n\
         <title>{} log</title>\n<subtitle>{}</subtitle>\n<link href=\"{}\"/>\n\
         <link rel=\"self\" href=\"{}\"/>\n<id>{}</id>\n<updated>{}</updated>\n\
         <generator>arks instaweb</generator>\n",
        esc(&ctx.name),
        esc(&ctx.description()),
        esc(&abs(ctx, &[("a", "summary")])),
        esc(&abs(ctx, &[("a", "atom")])),
        esc(&abs(ctx, &[("a", "summary")])),
        esc(&updated),
    );
    for (oid, c) in &items {
        let hex = oid.to_hex();
        let link = abs(ctx, &[("a", "commit"), ("h", &hex)]);
        x.push_str(&format!(
            "<entry>\n<title type=\"text\">{}</title>\n<updated>{}</updated>\n\
             <author><name>{}</name><email>{}</email></author>\n<published>{}</published>\n\
             <link rel=\"alternate\" type=\"text/html\" href=\"{}\"/>\n<id>{}</id>\n\
             <content type=\"text\">{}</content>\n</entry>\n",
            esc(c.subject()),
            esc(&date(&utc(&c.committer), "iso-strict")),
            esc(&c.author.name),
            esc(&c.author.email),
            esc(&date(&utc(&c.author), "iso-strict")),
            esc(&link),
            esc(&link),
            esc(&entry_text(ctx, c)),
        ));
    }
    x.push_str("</feed>\n");
    text(200, "application/atom+xml; charset=utf-8", x.into_bytes())
}

/// `<project> <owner>` per line, each URL-escaped (spaces as `+`).
pub fn project_index(ctx: &Ctx) -> HttpResponse {
    let enc = |s: &str| percent_encode(s).replace("%20", "+");
    let line = format!(
        "{} {}\n",
        enc(&ctx.name),
        enc(&ctx.owner().unwrap_or_default())
    );
    text(200, "text/plain; charset=utf-8", line.into_bytes())
}
