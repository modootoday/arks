//! HTML building blocks for the instaweb pages: escaping, links in the
//! query form, the page frame and the stylesheet. The markup is arks's own.

use super::params::percent_encode;

/// Escape text for HTML element content and attribute values.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if c.is_control() && c != '\n' && c != '\t' => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out
}

/// A page link in the query form: `/?p=<project>;a=<action>;…`.
pub fn href(project: Option<&str>, pairs: &[(&str, &str)]) -> String {
    let mut parts = Vec::new();
    if let Some(p) = project {
        parts.push(format!("p={}", percent_encode(p)));
    }
    for (k, v) in pairs {
        parts.push(format!("{k}={}", percent_encode(v)));
    }
    format!("/?{}", parts.join(";"))
}

/// `<a href="…">text</a>` with `text` escaped.
pub fn link(url: &str, text: &str) -> String {
    format!("<a href=\"{}\">{}</a>", esc(url), esc(text))
}

/// The page frame: head with feed links, a header naming the project and
/// action, the navigation bar (when a project is open) and the body.
pub fn frame(project: Option<&str>, action: &str, nav: &str, body: &str) -> String {
    let title = match project {
        Some(p) => format!("{p}/{action}"),
        None => "projects".to_string(),
    };
    let mut feeds = String::new();
    if let Some(p) = project {
        for (kind, ty) in [
            ("rss", "application/rss+xml"),
            ("atom", "application/atom+xml"),
        ] {
            feeds.push_str(&format!(
                "<link rel=\"alternate\" title=\"{} log {kind} feed\" href=\"{}\" type=\"{ty}\"/>\n",
                esc(p),
                esc(&href(Some(p), &[("a", kind)])),
            ));
        }
    }
    let crumbs = match project {
        Some(p) => format!(
            "{} / {} / {}",
            link("/", "projects"),
            link(&href(Some(p), &[("a", "summary")]), p),
            esc(action)
        ),
        None => link("/", "projects"),
    };
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\"/>\n\
         <meta name=\"generator\" content=\"arks instaweb\"/>\n<title>{}</title>\n\
         <link rel=\"stylesheet\" type=\"text/css\" href=\"/static/gitweb.css\"/>\n{feeds}\
         </head>\n<body>\n<header class=\"top\">{crumbs}</header>\n{nav}<main>\n{body}</main>\n\
         <footer>{}</footer>\n</body>\n</html>\n",
        esc(&title),
        project
            .map(|p| format!(
                "{} {}",
                link(&href(Some(p), &[("a", "rss")]), "RSS"),
                link(&href(Some(p), &[("a", "atom")]), "Atom")
            ))
            .unwrap_or_else(|| link(&href(None, &[("a", "project_index")]), "TXT")),
    )
}

/// The project navigation bar; `rev` keeps the browsed revision in links.
pub fn nav(project: &str, current: &str, rev: Option<&str>) -> String {
    let mut items = Vec::new();
    for a in [
        "summary",
        "shortlog",
        "log",
        "commit",
        "commitdiff",
        "tree",
        "heads",
        "tags",
    ] {
        let mut pairs = vec![("a", a)];
        if let Some(r) = rev.filter(|_| a != "summary" && a != "heads" && a != "tags") {
            pairs.push(if a == "tree" { ("hb", r) } else { ("h", r) });
        }
        if a == current {
            items.push(format!("<b>{}</b>", esc(a)));
        } else {
            items.push(link(&href(Some(project), &pairs), a));
        }
    }
    format!("<nav class=\"actions\">{}</nav>\n", items.join(" | "))
}

/// A table with a header row; cells are already HTML.
pub fn table(class: &str, head: &[&str], rows: &[Vec<String>]) -> String {
    let mut t = format!("<table class=\"{class}\">\n<tr>");
    for h in head {
        t.push_str(&format!("<th>{}</th>", esc(h)));
    }
    t.push_str("</tr>\n");
    for r in rows {
        t.push_str("<tr>");
        for c in r {
            t.push_str(&format!("<td>{c}</td>"));
        }
        t.push_str("</tr>\n");
    }
    t.push_str("</table>\n");
    t
}

/// The stylesheet served at `/static/gitweb.css`.
pub const CSS: &str = "body{font-family:sans-serif;font-size:small;margin:0 1em}\n\
header.top{background:#d9d8d1;padding:.4em;font-weight:bold}\n\
nav.actions{padding:.4em;border-bottom:1px solid #d9d8d1}\n\
table{border-collapse:collapse;margin:.5em 0}\n\
td,th{padding:.1em .5em;text-align:left;vertical-align:top}\n\
tr:nth-child(even){background:#f6f6f0}\n\
pre{font-family:monospace;margin:0}\n\
.diff .add{color:#008800}.diff .del{color:#cc0000}.diff .hunk{color:#990099}\n\
.diff .head{font-weight:bold}\n\
.lineno{color:#999;text-align:right;padding-right:.5em}\n\
.mode{font-family:monospace}\n\
footer{border-top:1px solid #d9d8d1;margin-top:1em;padding:.4em;text-align:right}\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_markup() {
        assert_eq!(
            esc("<a href=\"x\">&'"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }

    #[test]
    fn query_links() {
        assert_eq!(
            href(Some(".git"), &[("a", "tree"), ("f", "src/a b")]),
            "/?p=.git;a=tree;f=src/a%20b"
        );
    }
}
