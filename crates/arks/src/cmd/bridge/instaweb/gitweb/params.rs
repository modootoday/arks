//! Request routing for the instaweb pages: which paths the pages own, the
//! query parameters (`p`, `a`, `h`, `hb`, `f`, `pg`, separated by `;` or
//! `&`) and the path_info form `<repo>/<action>/<revision>:/<path>`.
//!
//! Primary: <https://git-scm.com/docs/gitweb> (ACTIONS, AND URLS).

/// The documented actions this server knows by name.
pub const ACTIONS: &[&str] = &[
    "project_list",
    "project_index",
    "summary",
    "shortlog",
    "log",
    "commit",
    "commitdiff",
    "commitdiff_plain",
    "patch",
    "tree",
    "blob",
    "blob_plain",
    "heads",
    "tags",
    "tag",
    "rss",
    "atom",
    "blame",
    "blame_incremental",
    "blobdiff",
    "blobdiff_plain",
    "history",
    "remotes",
    "search",
    "snapshot",
];

/// Parsed page parameters.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Params {
    pub project: Option<String>,
    pub action: Option<String>,
    pub hash: Option<String>,
    pub hash_base: Option<String>,
    pub file: Option<String>,
    pub page: usize,
}

/// What a request path asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    Page(Params),
    Static(String),
}

/// Route `path?query` for the single project `project`; `None` leaves the
/// request to git HTTP.
pub fn route(path: &str, query: &str, project: &str) -> Option<Target> {
    if let Some(name) = path.strip_prefix("/static/") {
        return Some(Target::Static(name.to_string()));
    }
    let info = if path == "/" {
        ""
    } else if let Some(rest) = ["/gitweb.cgi", "/index.cgi"]
        .iter()
        .find_map(|s| path.strip_prefix(s))
    {
        if !rest.is_empty() && !rest.starts_with('/') {
            return None;
        }
        rest
    } else {
        let first = path.trim_start_matches('/').split('/').next().unwrap_or("");
        if percent_decode(first) != project {
            return None;
        }
        path
    };
    let mut p = Params::default();
    parse_path_info(info, &mut p);
    parse_query(query, &mut p);
    Some(Target::Page(p))
}

/// `<repo>/<action>/<revision>:/<path>`: the action segment is optional, the
/// revision defaults to HEAD downstream.
fn parse_path_info(info: &str, p: &mut Params) {
    let info = info.trim_start_matches('/');
    if info.is_empty() {
        return;
    }
    let (repo, rest) = info.split_once('/').unwrap_or((info, ""));
    p.project = Some(percent_decode(repo));
    let (action, rest) = match rest.split_once('/') {
        Some((a, r)) if ACTIONS.contains(&a) => (Some(a), r),
        None if ACTIONS.contains(&rest) => (Some(rest), ""),
        _ => (None, rest),
    };
    p.action = action.map(str::to_string);
    if rest.is_empty() {
        return;
    }
    let rest = percent_decode(rest);
    match rest.split_once(':') {
        Some((rev, path)) => {
            if !rev.is_empty() {
                p.hash_base = Some(rev.to_string());
            }
            let path = path.trim_matches('/');
            if !path.is_empty() {
                p.file = Some(path.to_string());
            }
        }
        None => p.hash = Some(rest.trim_end_matches('/').to_string()),
    }
}

/// `k=v` pairs separated by `;` or `&`; later values win.
pub fn parse_query(query: &str, p: &mut Params) {
    for pair in query.split([';', '&']).filter(|s| !s.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = percent_decode(v);
        let opt = || (!v.is_empty()).then(|| v.clone());
        match k {
            "p" => p.project = opt(),
            "a" => p.action = opt(),
            "h" => p.hash = opt(),
            "hb" => p.hash_base = opt(),
            "f" => p.file = opt().map(|f| f.trim_matches('/').to_string()),
            "pg" => p.page = v.parse().unwrap_or(0),
            _ => {}
        }
    }
}

/// Decode `%XX` escapes and `+` (as a space).
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                let hex = |c: u8| (c as char).to_digit(16);
                match (hex(b[i + 1]), hex(b[i + 2])) {
                    (Some(h), Some(l)) => {
                        out.push((h * 16 + l) as u8);
                        i += 2;
                    }
                    _ => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Escape a query value (unreserved characters and `/` stay).
pub fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(path: &str, query: &str) -> Params {
        match route(path, query, ".git") {
            Some(Target::Page(p)) => p,
            other => panic!("{path}?{query}: {other:?}"),
        }
    }

    #[test]
    fn query_form_takes_both_separators() {
        let p = page("/", "p=.git;a=commit&h=abc;f=src/a.txt;pg=2");
        assert_eq!(p.project.as_deref(), Some(".git"));
        assert_eq!(p.action.as_deref(), Some("commit"));
        assert_eq!(p.hash.as_deref(), Some("abc"));
        assert_eq!(p.file.as_deref(), Some("src/a.txt"));
        assert_eq!(p.page, 2);
    }

    #[test]
    fn path_info_form() {
        let p = page("/.git/tree/main:/src", "");
        assert_eq!(p.action.as_deref(), Some("tree"));
        assert_eq!(p.hash_base.as_deref(), Some("main"));
        assert_eq!(p.file.as_deref(), Some("src"));
        let p = page("/gitweb.cgi/.git/shortlog/feature", "");
        assert_eq!(p.action.as_deref(), Some("shortlog"));
        assert_eq!(p.hash.as_deref(), Some("feature"));
        let p = page("/.git", "");
        assert_eq!(p.project.as_deref(), Some(".git"));
        assert_eq!(p.action, None);
        let p = page("/.git/HEAD:/README.md", "");
        assert_eq!(p.action, None);
        assert_eq!(p.file.as_deref(), Some("README.md"));
    }

    #[test]
    fn git_http_paths_are_left_alone() {
        assert_eq!(route("/info/refs", "service=git-upload-pack", ".git"), None);
        assert_eq!(route("/objects/info/packs", "", ".git"), None);
        assert_eq!(route("/gitweb.cgix", "", ".git"), None);
        assert_eq!(
            route("/static/gitweb.css", "", ".git"),
            Some(Target::Static("gitweb.css".into()))
        );
    }

    #[test]
    fn percent_round_trip() {
        assert_eq!(percent_decode("a%20b+c%2"), "a b c%2");
        assert_eq!(percent_encode("a b/c;d"), "a%20b/c%3Bd");
    }
}
