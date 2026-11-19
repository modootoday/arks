//! gitattributes(5) lookup for `check-attr`.
//!
//! Sources, lowest precedence first: `core.attributesFile` (default
//! `$XDG_CONFIG_HOME/git/attributes` or `~/.config/git/attributes`), the
//! `.gitattributes` files from the work-tree root down to the path's
//! directory, then `$GIT_DIR/info/attributes`. Later matching lines win per
//! attribute. A pattern without `/` matches the basename at any depth,
//! otherwise it is anchored to its file's directory (wildmatch with
//! `WM_PATHNAME`). Values: `attr` (set), `-attr` (unset), `!attr`
//! (unspecified), `attr=value`. `[attr]<name> ...` defines a macro in the
//! root `.gitattributes` or `info/attributes`; the built-in `binary` macro
//! is `-diff -merge -text`.
//!
//! Primary: <https://git-scm.com/docs/gitattributes>.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// One attribute state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
    Set,
    Unset,
    Unspecified,
    Value(String),
}

impl AttrValue {
    pub fn display(&self) -> &str {
        match self {
            Self::Set => "set",
            Self::Unset => "unset",
            Self::Unspecified => "unspecified",
            Self::Value(v) => v,
        }
    }
}

struct Line {
    /// Directory (work-tree relative, `""` = root) the pattern is relative to.
    base: String,
    pattern: String,
    assigns: Vec<(String, AttrValue)>,
}

fn parse_assign(tok: &str) -> (String, AttrValue) {
    if let Some(k) = tok.strip_prefix('-') {
        (k.to_string(), AttrValue::Unset)
    } else if let Some(k) = tok.strip_prefix('!') {
        (k.to_string(), AttrValue::Unspecified)
    } else if let Some((k, v)) = tok.split_once('=') {
        (k.to_string(), AttrValue::Value(v.to_string()))
    } else {
        (tok.to_string(), AttrValue::Set)
    }
}

/// Attribute rules for one work tree.
pub struct Attributes {
    lines: Vec<Line>,
    macros: BTreeMap<String, Vec<(String, AttrValue)>>,
    /// Attribute names in git's registration order (built-in `binary`
    /// macro first, then first appearance in attr.c's read order: global,
    /// root `.gitattributes`, `info/attributes`, subdirectories); `-a`
    /// lists attributes in this order.
    order: Vec<String>,
}

impl Attributes {
    pub fn load(work: &Path, git_dir: &Path, rel: &str) -> Self {
        let mut a = Attributes {
            lines: Vec::new(),
            macros: BTreeMap::new(),
            order: ["binary", "diff", "merge", "text"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        a.macros.insert(
            "binary".into(),
            vec![
                ("diff".into(), AttrValue::Unset),
                ("merge".into(), AttrValue::Unset),
                ("text".into(), AttrValue::Unset),
            ],
        );
        let cfg = arks_config::ConfigSet::load(Some(git_dir));
        let global = cfg.get_path("core.attributesFile").or_else(|| {
            std::env::var_os("XDG_CONFIG_HOME")
                .filter(|v| !v.is_empty())
                .map(|x| PathBuf::from(x).join("git").join("attributes"))
                .or_else(|| {
                    std::env::var_os("HOME")
                        .map(|h| PathBuf::from(h).join(".config/git/attributes"))
                })
        });
        let mut subdirs = Vec::new();
        let mut dir = String::new();
        let comps: Vec<&str> = rel.split('/').collect();
        for c in &comps[..comps.len().saturating_sub(1)] {
            dir = if dir.is_empty() {
                c.to_string()
            } else {
                format!("{dir}/{c}")
            };
            subdirs.push(dir.clone());
        }
        let info = git_dir.join("info").join("attributes");
        // Registration follows attr.c's read order (info before subdirs).
        for p in global
            .iter()
            .cloned()
            .chain([work.join(".gitattributes"), info.clone()])
            .chain(subdirs.iter().map(|d| work.join(d).join(".gitattributes")))
        {
            a.register(&p);
        }
        // Precedence: global < root < subdirectories < info/attributes.
        if let Some(g) = &global {
            a.add_file(g, "", false);
        }
        a.add_file(&work.join(".gitattributes"), "", true);
        for d in &subdirs {
            a.add_file(&work.join(d).join(".gitattributes"), d, false);
        }
        a.add_file(&info, "", true);
        a
    }

    fn register(&mut self, path: &Path) {
        let Ok(text) = fs::read_to_string(path) else {
            return;
        };
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut toks = line.split_whitespace();
            let first = toks.next().unwrap_or("");
            let names = first
                .strip_prefix("[attr]")
                .into_iter()
                .map(str::to_string)
                .chain(toks.map(|t| parse_assign(t).0));
            for n in names {
                if !self.order.contains(&n) {
                    self.order.push(n);
                }
            }
        }
    }

    /// Attribute names in registration order (for `check-attr -a`).
    pub fn order(&self) -> &[String] {
        &self.order
    }

    fn add_file(&mut self, path: &Path, base: &str, macros_ok: bool) {
        let Ok(text) = fs::read_to_string(path) else {
            return;
        };
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut toks = line.split_whitespace();
            let Some(pat) = toks.next() else { continue };
            let assigns: Vec<(String, AttrValue)> = toks.map(parse_assign).collect();
            if let Some(name) = pat.strip_prefix("[attr]") {
                if macros_ok {
                    self.macros.insert(name.to_string(), assigns);
                }
                continue;
            }
            self.lines.push(Line {
                base: base.to_string(),
                pattern: pat.to_string(),
                assigns,
            });
        }
    }

    fn matches(line: &Line, rel: &str) -> bool {
        let sub = if line.base.is_empty() {
            rel
        } else {
            match rel.strip_prefix(&format!("{}/", line.base)) {
                Some(s) => s,
                None => return false,
            }
        };
        let pat = line.pattern.strip_prefix('/').unwrap_or(&line.pattern);
        if line.pattern.contains('/') {
            crate::cmd::ignore::wildmatch(pat.as_bytes(), sub.as_bytes())
        } else {
            let base = sub.rsplit('/').next().unwrap_or(sub);
            crate::cmd::ignore::wildmatch(pat.as_bytes(), base.as_bytes())
        }
    }

    /// Every attribute state for `rel` (macros expanded; unspecified kept).
    pub fn for_path(&self, rel: &str) -> BTreeMap<String, AttrValue> {
        let mut out = BTreeMap::new();
        for line in self.lines.iter().filter(|l| Self::matches(l, rel)) {
            for (k, v) in &line.assigns {
                if let (Some(expansion), AttrValue::Set) = (self.macros.get(k), v) {
                    for (mk, mv) in expansion {
                        out.insert(mk.clone(), mv.clone());
                    }
                }
                out.insert(k.clone(), v.clone());
            }
        }
        out
    }
}
