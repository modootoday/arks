//! `.mailmap` identity mapping for `log --use-mailmap` / `%aN` / `%aE`.
//!
//! Line forms: `Proper Name <commit@email>`, `<proper@email> <commit@email>`,
//! `Proper Name <proper@email> <commit@email>` and
//! `Proper Name <proper@email> Commit Name <commit@email>`; emails compare
//! case-insensitively, names as written. `mailmap.file` adds entries.
//!
//! Primary: <https://git-scm.com/docs/gitmailmap>.

use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct Mailmap {
    /// (commit name, commit email) -> (proper name, proper email)
    entries: Vec<Entry>,
}

/// (commit name, commit email, proper name, proper email)
type Entry = (Option<String>, String, Option<String>, Option<String>);

fn parse_line(line: &str) -> Option<Entry> {
    let line = line.split('#').next().unwrap_or("").trim();
    if line.is_empty() {
        return None;
    }
    // Split into (name, email) pieces.
    let mut parts: Vec<(String, String)> = Vec::new();
    let mut rest = line;
    while let Some(lt) = rest.find('<') {
        let gt = rest[lt..].find('>')? + lt;
        let name = rest[..lt].trim().to_string();
        let email = rest[lt + 1..gt].trim().to_string();
        parts.push((name, email));
        rest = &rest[gt + 1..];
    }
    let opt = |s: &str| (!s.is_empty()).then(|| s.to_string());
    match parts.as_slice() {
        [(n, e)] => Some((None, e.clone(), opt(n), None)),
        [(pn, pe), (cn, ce)] => Some((opt(cn), ce.clone(), opt(pn), opt(pe))),
        _ => None,
    }
}

impl Mailmap {
    pub fn load(work: Option<&Path>, cfg: &arks_config::ConfigSet) -> Self {
        let mut m = Mailmap::default();
        let mut files = Vec::new();
        if let Some(w) = work {
            files.push(w.join(".mailmap"));
        }
        if let Some(p) = cfg.get_path("mailmap.file") {
            files.push(p);
        }
        for f in files {
            if let Ok(t) = std::fs::read_to_string(&f) {
                m.entries.extend(t.lines().filter_map(parse_line));
            }
        }
        m
    }

    /// Add one `.mailmap` line.
    #[cfg(test)]
    pub fn add_line(&mut self, line: &str) {
        self.entries.extend(parse_line(line));
    }

    /// Mapped `(name, email)`.
    pub fn map(&self, name: &str, email: &str) -> (String, String) {
        let mut best: Option<&Entry> = None;
        for e in &self.entries {
            if !e.1.eq_ignore_ascii_case(email) {
                continue;
            }
            match &e.0 {
                Some(n) if n == name => {
                    best = Some(e);
                    break;
                }
                Some(_) => {}
                None => {
                    if best.is_none() {
                        best = Some(e);
                    }
                }
            }
        }
        match best {
            Some((_, _, pn, pe)) => (
                pn.clone().unwrap_or_else(|| name.to_string()),
                pe.clone().unwrap_or_else(|| email.to_string()),
            ),
            None => (name.to_string(), email.to_string()),
        }
    }
}
