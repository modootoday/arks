//! git-config file syntax (§SYNTAX): sections, subsections, quoting, escapes,
//! continuation lines, comments.

use std::fmt;

/// One `key = value` occurrence; `key` is normalized to `section[.subsection].name`
/// with section and name lowercased and subsection kept case-sensitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    pub key: String,
    /// `None` for a bare `name` line (boolean true per git-config).
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad config line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse one config file body.
pub fn parse_config_text(text: &str) -> Result<Vec<RawEntry>, ParseError> {
    let mut out = Vec::new();
    let mut section: Option<String> = None;
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line_no = i + 1;
        let line = lines[i].trim_start_matches('\u{feff}');
        i += 1;
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let mut rest = trimmed;
        if rest.starts_with('[') {
            let (sec, after) = parse_section_header(rest).map_err(|m| ParseError {
                line: line_no,
                message: m,
            })?;
            section = Some(sec);
            rest = after.trim_start();
            if rest.is_empty() || rest.starts_with('#') || rest.starts_with(';') {
                continue;
            }
        }
        let Some(sec) = section.as_ref() else {
            return Err(ParseError {
                line: line_no,
                message: "variable outside any section".into(),
            });
        };
        let name_end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(rest.len());
        let name = &rest[..name_end];
        if name.is_empty() || !name.as_bytes()[0].is_ascii_alphabetic() {
            return Err(ParseError {
                line: line_no,
                message: "invalid variable name".into(),
            });
        }
        let after_name = rest[name_end..].trim_start();
        let key = format!("{sec}.{}", name.to_ascii_lowercase());
        if after_name.is_empty() || after_name.starts_with('#') || after_name.starts_with(';') {
            out.push(RawEntry { key, value: None });
            continue;
        }
        let Some(value_src) = after_name.strip_prefix('=') else {
            return Err(ParseError {
                line: line_no,
                message: "expected '=' after variable name".into(),
            });
        };
        // Join continuation lines (backslash-newline) into one logical value.
        let mut logical = value_src.to_string();
        while ends_with_unescaped_backslash(&logical) && i < lines.len() {
            logical.pop();
            logical.push_str(lines[i]);
            i += 1;
        }
        let value = parse_value(&logical).map_err(|m| ParseError {
            line: line_no,
            message: m,
        })?;
        out.push(RawEntry {
            key,
            value: Some(value),
        });
    }
    Ok(out)
}

fn ends_with_unescaped_backslash(s: &str) -> bool {
    let trailing = s.bytes().rev().take_while(|b| *b == b'\\').count();
    trailing % 2 == 1
}

fn parse_section_header(s: &str) -> Result<(String, &str), String> {
    let body = &s[1..];
    let name_end = body
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))
        .unwrap_or(body.len());
    let name = &body[..name_end];
    if name.is_empty() {
        return Err("empty section name".into());
    }
    let mut rest = &body[name_end..];
    if let Some(r) = rest.strip_prefix(']') {
        // `[section]` or legacy `[section.subsection]` (subsection lowercased).
        return Ok((name.to_ascii_lowercase(), r));
    }
    rest = rest.trim_start();
    let Some(q) = rest.strip_prefix('"') else {
        return Err("malformed section header".into());
    };
    let mut sub = String::new();
    let mut chars = q.char_indices();
    loop {
        let Some((idx, c)) = chars.next() else {
            return Err("unterminated subsection".into());
        };
        match c {
            '\\' => {
                if let Some((_, n)) = chars.next() {
                    sub.push(n);
                }
            }
            '"' => {
                let after = &q[idx + 1..];
                let Some(r) = after.strip_prefix(']') else {
                    return Err("expected ']' after subsection".into());
                };
                return Ok((format!("{}.{}", name.to_ascii_lowercase(), sub), r));
            }
            '\n' => return Err("newline in subsection".into()),
            other => sub.push(other),
        }
    }
}

fn parse_value(src: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut in_quote = false;
    // Whitespace seen outside quotes is held back so trailing runs get trimmed.
    let mut pending_ws = String::new();
    let mut started = false;
    let mut chars = src.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push_str(&pending_ws);
                pending_ws.clear();
                in_quote = !in_quote;
                started = true;
            }
            '\\' => {
                out.push_str(&pending_ws);
                pending_ws.clear();
                started = true;
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('b') => {
                        out.pop();
                    }
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some('\n') => {}
                    Some(other) => return Err(format!("invalid escape '\\{other}'")),
                    None => {}
                }
            }
            '#' | ';' if !in_quote => break,
            c if c.is_whitespace() && !in_quote => {
                if started {
                    pending_ws.push(c);
                }
            }
            c => {
                out.push_str(&pending_ws);
                pending_ws.clear();
                out.push(c);
                started = true;
            }
        }
    }
    if in_quote {
        return Err("unterminated quote".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv(text: &str) -> Vec<(String, Option<String>)> {
        parse_config_text(text)
            .unwrap()
            .into_iter()
            .map(|e| (e.key, e.value))
            .collect()
    }

    #[test]
    fn sections_subsections_and_case() {
        let got = kv("[User]\n\tName = A B\n[remote \"Origin\"]\n url = x # c\n[a.B]\nk\n");
        assert_eq!(
            got,
            vec![
                ("user.name".into(), Some("A B".into())),
                ("remote.Origin.url".into(), Some("x".into())),
                ("a.b.k".into(), None),
            ]
        );
    }

    #[test]
    fn quotes_escapes_continuation() {
        let got = kv("[s]\nv = \"  a;b \" c\\tq\nw = one \\\n two\n");
        assert_eq!(got[0].1.as_deref(), Some("  a;b  c\tq"));
        assert_eq!(got[1].1.as_deref(), Some("one  two"));
    }

    #[test]
    fn errors_are_reported() {
        assert!(parse_config_text("k = v\n").is_err());
        assert!(parse_config_text("[s]\nv = \"open\n").is_err());
    }
}
