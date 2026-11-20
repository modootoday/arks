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
    /// 1-based line in its source and the line as written (`check-ignore -v`).
    line: usize,
    text: String,
}

/// The pattern that decided a path (`git check-ignore -v`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreMatch {
    /// Source file as git shows it (`.gitignore`, `dir/.gitignore`,
    /// `.git/info/exclude`, or the global excludes path).
    pub source: String,
    pub line: usize,
    pub pattern: String,
    /// `false` when a `!` pattern matched (the path is re-included).
    pub excluded: bool,
}

fn parse(text: &str) -> Vec<Pat> {
    let mut out = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
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
            line: lineno + 1,
            text: raw.trim_end_matches('\r').to_string(),
        });
    }
    out
}

/// wildmatch with `WM_PATHNAME` semantics.
pub(crate) fn wildmatch(p: &[u8], t: &[u8]) -> bool {
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
    global_name: String,
    /// Command-line patterns (`ls-files -x` / `-X`), checked first.
    cli: Vec<Pat>,
    /// Whether `.gitignore`-style files, info/exclude and core.excludesFile
    /// apply (`--exclude-standard`); off for `-x`-only listings.
    standard: bool,
    /// Per-directory exclude file name (`--exclude-per-directory`).
    per_dir_name: String,
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
        let global = global_path.as_deref().map(read).unwrap_or_default();
        let global_name = global_path
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        Self {
            work: work.to_path_buf(),
            per_dir: HashMap::new(),
            info,
            global,
            global_name,
            cli: Vec::new(),
            standard: true,
            per_dir_name: ".gitignore".to_string(),
        }
    }

    /// Rules from explicit sources only: no `.gitignore`, info/exclude or
    /// core.excludesFile until [`Ignore::per_directory`] names a file.
    pub fn explicit(work: &Path) -> Self {
        let mut ig = Self::new(work);
        ig.standard = false;
        ig.info.clear();
        ig.global.clear();
        ig
    }

    /// Add command-line patterns (one per line), checked before any file.
    pub fn add_patterns(&mut self, text: &str) {
        self.cli.extend(parse(text));
    }

    /// Add command-line exclude patterns (`clean -e`), builder form.
    pub fn with_cli_patterns(mut self, patterns: &[String]) -> Self {
        self.add_patterns(&patterns.join("\n"));
        self
    }

    /// Read `name` in every directory as its exclude file.
    pub fn per_directory(&mut self, name: &str) {
        self.per_dir_name = name.to_string();
        self.per_dir.clear();
        self.standard = true;
    }

    fn load_dir(&mut self, dir: &str) {
        if !self.per_dir.contains_key(dir) {
            if !self.standard {
                self.per_dir.insert(dir.to_string(), Vec::new());
                return;
            }
            let path = if dir.is_empty() {
                self.work.join(&self.per_dir_name)
            } else {
                self.work.join(dir).join(&self.per_dir_name)
            };
            let pats = fs::read_to_string(path)
                .map(|t| parse(&t))
                .unwrap_or_default();
            self.per_dir.insert(dir.to_string(), pats);
        }
    }

    /// The last pattern in `pats` matching the path.
    fn decide<'a>(
        pats: &'a [Pat],
        rel_to_base: &str,
        basename: &str,
        is_dir: bool,
    ) -> Option<&'a Pat> {
        pats.iter().rev().find(|p| {
            if p.dir_only && !is_dir {
                return false;
            }
            let subject = if p.anchored { rel_to_base } else { basename };
            wildmatch(&p.glob, subject.as_bytes())
        })
    }

    /// Is the work-tree-relative `rel` excluded? Callers must not ask about
    /// paths below an excluded directory (git never re-includes those).
    pub fn is_ignored(&mut self, rel: &str, is_dir: bool) -> bool {
        self.matching(rel, is_dir).is_some_and(|m| m.excluded)
    }

    /// The pattern deciding `rel` itself (no parent-directory check).
    pub fn matching(&mut self, rel: &str, is_dir: bool) -> Option<IgnoreMatch> {
        let found = |src: String, p: &Pat| IgnoreMatch {
            source: src,
            line: p.line,
            pattern: p.text.clone(),
            excluded: !p.negate,
        };
        let basename = rel.rsplit('/').next().unwrap_or(rel).to_string();
        if let Some(p) = Self::decide(&self.cli, rel, &basename, is_dir) {
            return Some(found(String::new(), p));
        }
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
            if let Some(p) = Self::decide(&self.per_dir[&dir], &base_rel, &basename, is_dir) {
                let src = if dir.is_empty() {
                    ".gitignore".to_string()
                } else {
                    format!("{dir}/.gitignore")
                };
                return Some(found(src, p));
            }
            if dir.is_empty() {
                break;
            }
            dir = match dir.rfind('/') {
                Some(i) => dir[..i].to_string(),
                None => String::new(),
            };
        }
        if let Some(p) = Self::decide(&self.info, rel, &basename, is_dir) {
            return Some(found(".git/info/exclude".to_string(), p));
        }
        Self::decide(&self.global, rel, &basename, is_dir)
            .map(|p| found(self.global_name.clone(), p))
    }

    /// `check-ignore` view: the pattern excluding a leading directory of
    /// `rel` (git never re-includes below it), else the one matching `rel`.
    pub fn matching_with_parents(&mut self, rel: &str, is_dir: bool) -> Option<IgnoreMatch> {
        let parts: Vec<&str> = rel.split('/').collect();
        for i in 1..parts.len() {
            let dir = parts[..i].join("/");
            if let Some(m) = self.matching(&dir, true).filter(|m| m.excluded) {
                return Some(m);
            }
        }
        self.matching(rel, is_dir)
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
