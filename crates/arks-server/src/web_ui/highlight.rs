//! Zero-dependency, XSS-safe server-side syntax highlighter and language breakdown calculator.

use super::{html_escape, load_blob, load_tree};
use arks_object::TreeMode;
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::BTreeMap;

/// Language metadata for sidebar breakdown bar.
pub struct LangStat {
    pub name: &'static str,
    pub color: &'static str,
    pub bytes: usize,
    pub pct: f64,
}

/// Detect display language and badge color from a file path.
pub fn detect_language(path: &str) -> Option<(&'static str, &'static str)> {
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some(("Rust", "#dea584")),
        "js" | "mjs" | "cjs" => Some(("JavaScript", "#f1e05a")),
        "ts" | "tsx" => Some(("TypeScript", "#3178c6")),
        "py" => Some(("Python", "#3572A5")),
        "rb" => Some(("Ruby", "#701516")),
        "go" => Some(("Go", "#00ADD8")),
        "html" | "htm" => Some(("HTML", "#e34c26")),
        "css" | "scss" => Some(("CSS", "#563d7c")),
        "json" => Some(("JSON", "#292929")),
        "toml" => Some(("TOML", "#9c4221")),
        "yaml" | "yml" => Some(("YAML", "#cb171e")),
        "sh" | "bash" | "zsh" => Some(("Shell", "#89e051")),
        "sql" => Some(("SQL", "#e38c00")),
        "c" | "h" | "cpp" | "cc" | "hpp" => Some(("C/C++", "#f34b7d")),
        "md" | "markdown" => Some(("Markdown", "#083fa1")),
        _ => None,
    }
}

/// Walk up to `max_files` blobs in `tree_oid` and compute top language percentages.
pub fn compute_language_stats(repo: &Repository, tree_oid: Oid, max_files: usize) -> Vec<LangStat> {
    let mut totals: BTreeMap<(&'static str, &'static str), usize> = BTreeMap::new();
    let mut seen = 0usize;
    walk_tree_langs(repo, tree_oid, &mut totals, &mut seen, max_files, 0);
    let grand: usize = totals.values().sum();
    if grand == 0 {
        return Vec::new();
    }
    let mut stats: Vec<LangStat> = totals
        .into_iter()
        .map(|((name, color), bytes)| LangStat {
            name,
            color,
            bytes,
            pct: (bytes as f64 / grand as f64) * 100.0,
        })
        .collect();
    stats.sort_by_key(|a| std::cmp::Reverse(a.bytes));
    stats.truncate(6);
    stats
}

fn walk_tree_langs(
    repo: &Repository,
    tree_oid: Oid,
    totals: &mut BTreeMap<(&'static str, &'static str), usize>,
    seen: &mut usize,
    max_files: usize,
    depth: usize,
) {
    if depth > 6 || *seen >= max_files {
        return;
    }
    let Some(tree) = load_tree(repo, &tree_oid) else {
        return;
    };
    for entry in tree.entries {
        if *seen >= max_files {
            break;
        }
        if entry.mode == TreeMode::Tree {
            walk_tree_langs(repo, entry.oid, totals, seen, max_files, depth + 1);
        } else if let Some(lang) = detect_language(&entry.name) {
            if let Some(blob) = load_blob(repo, &entry.oid) {
                *totals.entry(lang).or_insert(0) += blob.len();
                *seen += 1;
            }
        }
    }
}

fn keywords_for(lang: &str) -> &'static [&'static str] {
    match lang {
        "Rust" => &[
            "fn", "pub", "let", "mut", "const", "static", "struct", "enum", "impl", "trait", "for",
            "in", "while", "loop", "if", "else", "match", "return", "use", "mod", "crate", "self",
            "Self", "where", "async", "await", "move", "ref", "type", "as", "break", "continue",
            "true", "false",
        ],
        "JavaScript" | "TypeScript" => &[
            "function",
            "const",
            "let",
            "var",
            "return",
            "if",
            "else",
            "for",
            "while",
            "switch",
            "case",
            "break",
            "import",
            "export",
            "from",
            "default",
            "class",
            "extends",
            "new",
            "async",
            "await",
            "try",
            "catch",
            "throw",
            "type",
            "interface",
            "true",
            "false",
            "null",
            "undefined",
        ],
        "Python" => &[
            "def", "class", "return", "if", "elif", "else", "for", "in", "while", "import", "from",
            "as", "with", "try", "except", "finally", "raise", "async", "await", "yield", "lambda",
            "True", "False", "None", "and", "or", "not",
        ],
        "Ruby" => &[
            "def", "end", "class", "module", "if", "elsif", "else", "unless", "do", "while",
            "until", "return", "require", "include", "yield", "begin", "rescue", "ensure", "true",
            "false", "nil",
        ],
        "Go" => &[
            "func",
            "package",
            "import",
            "type",
            "struct",
            "interface",
            "return",
            "if",
            "else",
            "for",
            "range",
            "switch",
            "case",
            "go",
            "defer",
            "chan",
            "select",
            "var",
            "const",
            "map",
            "true",
            "false",
            "nil",
        ],
        "Shell" => &[
            "if", "then", "else", "elif", "fi", "for", "do", "done", "while", "case", "esac",
            "function", "return", "export", "local", "set", "exit",
        ],
        "SQL" => &[
            "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE",
            "CREATE", "TABLE", "INDEX", "JOIN", "LEFT", "INNER", "ON", "ORDER", "BY", "GROUP",
            "LIMIT", "AND", "OR", "NOT", "NULL", "AS",
        ],
        _ => &[],
    }
}

/// Highlight a single line of source code safely (all text is HTML-escaped).
pub fn highlight_line(line: &str, lang: Option<&str>) -> String {
    let Some(lang) = lang else {
        return html_escape(line);
    };
    let trimmed = line.trim_start();
    // Single-line comment fast path
    let is_comment = match lang {
        "Rust" | "JavaScript" | "TypeScript" | "Go" | "C/C++" => trimmed.starts_with("//"),
        "Python" | "Ruby" | "Shell" | "YAML" | "TOML" => trimmed.starts_with('#'),
        "SQL" => trimmed.starts_with("--"),
        _ => false,
    };
    if is_comment {
        return format!("<span class=\"cm\">{}</span>", html_escape(line));
    }

    let kws = keywords_for(lang);
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len() + 32);
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        // String literal
        if b == b'"' || b == b'\'' {
            let quote = b;
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                let cur = bytes[i];
                i += 1;
                if cur == quote {
                    break;
                }
            }
            out.push_str("<span class=\"str\">");
            out.push_str(&html_escape(&line[start..i]));
            out.push_str("</span>");
            continue;
        }
        // Inline comment
        if matches!(lang, "Rust" | "JavaScript" | "TypeScript" | "Go" | "C/C++")
            && b == b'/'
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'/'
        {
            out.push_str("<span class=\"cm\">");
            out.push_str(&html_escape(&line[i..]));
            out.push_str("</span>");
            break;
        }
        // Identifier or keyword
        if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &line[start..i];
            if kws.contains(&word) {
                out.push_str("<span class=\"kw\">");
                out.push_str(&html_escape(word));
                out.push_str("</span>");
            } else if word.starts_with(|c: char| c.is_ascii_uppercase())
                && word.chars().any(|c| c.is_ascii_lowercase())
            {
                out.push_str("<span class=\"ty\">");
                out.push_str(&html_escape(word));
                out.push_str("</span>");
            } else if i < bytes.len() && bytes[i] == b'(' {
                out.push_str("<span class=\"fn\">");
                out.push_str(&html_escape(word));
                out.push_str("</span>");
            } else {
                out.push_str(&html_escape(word));
            }
            continue;
        }
        // Numeric literal
        if b.is_ascii_digit() {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_hexdigit() || bytes[i] == b'.' || bytes[i] == b'_')
            {
                i += 1;
            }
            out.push_str("<span class=\"num\">");
            out.push_str(&html_escape(&line[start..i]));
            out.push_str("</span>");
            continue;
        }
        // Single UTF-8 char fallback
        let ch = line[i..].chars().next().unwrap_or(' ');
        i += ch.len_utf8();
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            other => out.push(other),
        }
    }
    out
}
