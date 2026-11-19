//! gitignore(5) exclude rules for untracked-file listing.
//!
//! Sources, highest precedence first: `.gitignore` files from the path's
//! directory up to the work-tree root, `$GIT_DIR/info/exclude`, then
//! `core.excludesFile` (default `$XDG_CONFIG_HOME/git/ignore` or
//! `~/.config/git/ignore`). Within one source the last matching pattern
//! decides; `!` re-includes. A pattern with a non-trailing `/` is anchored to
//! its `.gitignore` directory, otherwise it matches the basename at any
//! depth; a trailing `/` matches directories only. Globs follow wildmatch
//! with `WM_PATHNAME` (`*`, `?`, `[...]`, `**`).
//! Primary: <https://git-scm.com/docs/gitignore>.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct Pat {
    glob: Vec<u8>,
    negate: bool,
    dir_only: bool,
    anchored: bool,
}

fn parse(text: &str) -> Vec<Pat> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let mut line = raw.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Trailing spaces are ignored unless escaped with a backslash.
        while line.ends_with(' ') && !line.ends_with("\\ ") {
            line = &line[..line.len() - 1];
        }
        if line.is_empty() {
            continue;
        }
        let mut negate = false;
        if let Some(rest) = line.strip_prefix('!') {
            negate = true;
            line = rest;
        } else if line.starts_with("\\!") || line.starts_with("\\#") {
            line = &line[1..];
        }
        let mut dir_only = false;
        if line.len() > 1 && line.ends_with('/') {
            dir_only = true;
            line = &line[..line.len() - 1];
        }
        let anchored = line.contains('/');
        let line = line.strip_prefix('/').unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        out.push(Pat {
            glob: line.as_bytes().to_vec(),
            negate,
            dir_only,
            anchored,
        });
    }
    out
}

/// wildmatch with `WM_PATHNAME` semantics.
fn wildmatch(p: &[u8], t: &[u8]) -> bool {
    let (mut pi, mut ti) = (0usize, 0usize);
    while pi < p.len() {
        match p[pi] {
            b'*' => {
                let double = p.get(pi + 1) == Some(&b'*');
                if double {
                    let at_start = pi == 0 || p[pi - 1] == b'/';
                    let after = pi + 2;
                    if at_start && after == p.len() {
                        return true;
                    }
                    if at_start && p.get(after) == Some(&b'/') {
                        // `**/` matches zero or more leading directories.
                        let rest = &p[after + 1..];
                        if wildmatch(rest, &t[ti..]) {
                            return true;
                        }
                        let mut k = ti;
                        while k < t.len() {
                            if t[k] == b'/' && wildmatch(rest, &t[k + 1..]) {
                                return true;
                            }
                            k += 1;
                        }
                        return false;
                    }
                    // Other `**` placements match anything, including `/`.
                    let rest = &p[after..];
                    return (ti..=t.len()).any(|k| wildmatch(rest, &t[k..]));
                }
                let rest = &p[pi + 1..];
                let mut k = ti;
                loop {
                    if wildmatch(rest, &t[k..]) {
                        return true;
                    }
                    if k >= t.len() || t[k] == b'/' {
                        return false;
                    }
                    k += 1;
                }
            }
            b'?' => {
                if ti >= t.len() || t[ti] == b'/' {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
            b'[' => {
                if ti >= t.len() || t[ti] == b'/' {
                    return false;
                }
                let c = t[ti];
                let mut j = pi + 1;
                let neg = matches!(p.get(j), Some(b'!') | Some(b'^'));
                if neg {
                    j += 1;
                }
                let mut hit = false;
                let mut first = true;
                while j < p.len() && (first || p[j] != b']') {
                    first = false;
                    let mut lo = p[j];
                    if lo == b'\\' && j + 1 < p.len() {
                        j += 1;
                        lo = p[j];
                    }
                    if j + 2 < p.len() && p[j + 1] == b'-' && p[j + 2] != b']' {
                        if (lo..=p[j + 2]).contains(&c) {
                            hit = true;
                        }
                        j += 3;
                    } else {
                        if lo == c {
                            hit = true;
                        }
                        j += 1;
                    }
                }
                if j >= p.len() || hit == neg {
                    return false;
                }
                pi = j + 1;
                ti += 1;
            }
            b'\\' if pi + 1 < p.len() => {
                if ti >= t.len() || t[ti] != p[pi + 1] {
                    return false;
                }
                pi += 2;
                ti += 1;
            }
            c => {
                if ti >= t.len() || t[ti] != c {
                    return false;
                }
                pi += 1;
                ti += 1;
            }
        }
    }
    ti == t.len()
}

/// Lazily loaded exclude rules for one work tree.
pub struct Ignore {
    work: PathBuf,
    per_dir: HashMap<String, Vec<Pat>>,
    info: Vec<Pat>,
    global: Vec<Pat>,
}

impl Ignore {
    pub fn new(work: &Path) -> Self {
        let read = |p: &Path| fs::read_to_string(p).map(|t| parse(&t)).unwrap_or_default();
        let git_dir = work.join(".git");
        let info = if git_dir.is_dir() {
            read(&git_dir.join("info").join("exclude"))
        } else {
            Vec::new()
        };
        let cfg = arks_config::ConfigSet::load_discovered(work);
        let global_path = cfg
            .get("core.excludesFile")
            .map(|v| {
                if let Some(rest) = v.strip_prefix("~/") {
                    std::env::var_os("HOME")
                        .map(PathBuf::from)
                        .unwrap_or_default()
                        .join(rest)
                } else {
                    PathBuf::from(v)
                }
            })
            .or_else(|| {
                std::env::var_os("XDG_CONFIG_HOME")
                    .filter(|v| !v.is_empty())
                    .map(|x| PathBuf::from(x).join("git").join("ignore"))
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(|h| PathBuf::from(h).join(".config").join("git").join("ignore"))
                    })
            });
        let global = global_path.map(|p| read(&p)).unwrap_or_default();
        Self {
            work: work.to_path_buf(),
            per_dir: HashMap::new(),
            info,
            global,
        }
    }

    fn load_dir(&mut self, dir: &str) {
        if !self.per_dir.contains_key(dir) {
            let path = if dir.is_empty() {
                self.work.join(".gitignore")
            } else {
                self.work.join(dir).join(".gitignore")
            };
            let pats = fs::read_to_string(path)
                .map(|t| parse(&t))
                .unwrap_or_default();
            self.per_dir.insert(dir.to_string(), pats);
        }
    }

    /// Decision of the last matching pattern in `pats` (`Some(true)` = excluded).
    fn decide(pats: &[Pat], rel_to_base: &str, basename: &str, is_dir: bool) -> Option<bool> {
        pats.iter().rev().find_map(|p| {
            if p.dir_only && !is_dir {
                return None;
            }
            let subject = if p.anchored { rel_to_base } else { basename };
            wildmatch(&p.glob, subject.as_bytes()).then_some(!p.negate)
        })
    }

    /// Is the work-tree-relative `rel` excluded? Callers must not ask about
    /// paths below an excluded directory (git never re-includes those).
    pub fn is_ignored(&mut self, rel: &str, is_dir: bool) -> bool {
        let basename = rel.rsplit('/').next().unwrap_or(rel).to_string();
        let mut dir = match rel.rfind('/') {
            Some(i) => rel[..i].to_string(),
            None => String::new(),
        };
        loop {
            let base_rel = if dir.is_empty() {
                rel.to_string()
            } else {
                rel[dir.len() + 1..].to_string()
            };
            self.load_dir(&dir);
            if let Some(d) = Self::decide(&self.per_dir[&dir], &base_rel, &basename, is_dir) {
                return d;
            }
            if dir.is_empty() {
                break;
            }
            dir = match dir.rfind('/') {
                Some(i) => dir[..i].to_string(),
                None => String::new(),
            };
        }
        if let Some(d) = Self::decide(&self.info, rel, &basename, is_dir) {
            return d;
        }
        Self::decide(&self.global, rel, &basename, is_dir).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildmatch_forms() {
        assert!(wildmatch(b"*.o", b"a.o"));
        assert!(!wildmatch(b"*.o", b"d/a.o"));
        assert!(wildmatch(b"**/foo", b"a/b/foo"));
        assert!(wildmatch(b"**/foo", b"foo"));
        assert!(wildmatch(b"a/**/b", b"a/b"));
        assert!(wildmatch(b"a/**/b", b"a/x/y/b"));
        assert!(wildmatch(b"abc/**", b"abc/x/y"));
        assert!(wildmatch(b"[a-c]?", b"bz"));
        assert!(!wildmatch(b"[!a-c]?", b"bz"));
    }

    #[test]
    fn parse_rules() {
        let p = parse("# c\n/target/\n!keep.log\n*.log\nsub/x\n");
        assert_eq!(p.len(), 4);
        assert!(p[0].dir_only && p[0].anchored);
        assert!(p[1].negate);
        assert!(!p[2].anchored);
        assert!(p[3].anchored);
    }
}
