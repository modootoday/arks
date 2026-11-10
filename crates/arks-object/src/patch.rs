//! Minimal format-patch mailbox parse (claimed single-hunk subset).
//!
//! Primary: <https://git-scm.com/docs/git-format-patch>
//! Cite: <https://git-scm.com/docs/git-am>
//! Ownership: Ring 1 object/presentation format — not CLI I/O.

use crate::ObjectError;

/// One file change reconstructed from a single claimed hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFile {
    pub path: String,
    pub base: Option<Vec<u8>>,
    pub tip: Option<Vec<u8>>,
}

/// Parsed single-mailbox format-patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPatch {
    pub message: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub files: Vec<ParsedFile>,
}

/// Parse one format-patch mailbox. Multi-patch mailboxes and multi-hunk diffs error.
pub fn parse_patch(text: &str) -> Result<ParsedPatch, ObjectError> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() || !is_mailbox_from(lines[0]) {
        return Err(ObjectError::BadPatch(
            "patch is not a format-patch mailbox".into(),
        ));
    }
    let from_count = lines.iter().filter(|line| is_mailbox_from(line)).count();
    if from_count != 1 {
        return Err(ObjectError::BadPatch(
            "am applies one patch; multi-patch mailboxes are not claimed".into(),
        ));
    }
    let mut i = 1;
    let mut author_name = None;
    let mut author_email = None;
    let mut subject = String::new();
    while i < lines.len() && !lines[i].is_empty() {
        if let Some(rest) = lines[i].strip_prefix("From: ") {
            if let Some((name, email)) = parse_author(rest) {
                author_name = Some(name);
                author_email = Some(email);
            }
        } else if let Some(rest) = lines[i].strip_prefix("Subject: ") {
            subject = strip_patch_subject(rest);
        }
        i += 1;
    }
    if i < lines.len() && lines[i].is_empty() {
        i += 1;
    }
    let mut body = Vec::new();
    while i < lines.len() && lines[i] != "---" {
        body.push(lines[i]);
        i += 1;
    }
    if i >= lines.len() || lines[i] != "---" {
        return Err(ObjectError::BadPatch("patch missing diff separator".into()));
    }
    i += 1;
    let message = if body.is_empty() {
        ensure_nl(subject)
    } else {
        ensure_nl(body.join("\n"))
    };
    let mut files = Vec::new();
    while i < lines.len() {
        if lines[i].starts_with("diff --git ") {
            let (file, next) = parse_one_diff(&lines, i)?;
            files.push(file);
            i = next;
        } else {
            i += 1;
        }
    }
    Ok(ParsedPatch {
        message,
        author_name,
        author_email,
        files,
    })
}

fn ensure_nl(mut message: String) -> String {
    if !message.ends_with('\n') {
        message.push('\n');
    }
    message
}

fn is_mailbox_from(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    if parts.next() != Some("From") {
        return false;
    }
    match parts.next() {
        Some(token) => token.len() == 40 && token.chars().all(|c| c.is_ascii_hexdigit()),
        None => false,
    }
}

fn parse_author(text: &str) -> Option<(String, String)> {
    let start = text.find(" <")?;
    let end = text[start + 2..].find('>')? + start + 2;
    let name = text[..start].trim().to_owned();
    let email = text[start + 2..end].to_owned();
    if name.is_empty() || email.is_empty() {
        None
    } else {
        Some((name, email))
    }
}

fn strip_patch_subject(subject: &str) -> String {
    let subject = subject.trim();
    if let Some(rest) = subject.strip_prefix("[PATCH]") {
        return rest.trim().to_owned();
    }
    if let Some(rest) = subject.strip_prefix("[PATCH ") {
        if let Some(end) = rest.find(']') {
            return rest[end + 1..].trim().to_owned();
        }
    }
    subject.to_owned()
}

fn parse_one_diff(lines: &[&str], start: usize) -> Result<(ParsedFile, usize), ObjectError> {
    let path = path_from_git_header(lines[start])?;
    let mut i = start + 1;
    let mut old_absent = false;
    let mut new_absent = false;
    while i < lines.len() && !lines[i].starts_with("@@") && !lines[i].starts_with("diff --git ") {
        if lines[i].starts_with("--- /dev/null") {
            old_absent = true;
        }
        if lines[i].starts_with("+++ /dev/null") {
            new_absent = true;
        }
        i += 1;
    }
    if i >= lines.len() || !lines[i].starts_with("@@") {
        return Err(ObjectError::BadPatch(format!(
            "patch hunk missing for {path}"
        )));
    }
    i += 1;
    let mut old_lines = Vec::new();
    let mut new_lines = Vec::new();
    let mut old_nl = true;
    let mut new_nl = true;
    let mut last = b' ';
    while i < lines.len() && !lines[i].starts_with("diff --git ") {
        let line = lines[i];
        if line == "\\ No newline at end of file" {
            if last == b'-' {
                old_nl = false;
            }
            if last == b'+' {
                new_nl = false;
            }
            i += 1;
            continue;
        }
        if line.starts_with("@@") {
            return Err(ObjectError::BadPatch(
                "multi-hunk patches are not claimed".into(),
            ));
        }
        if let Some(rest) = line.strip_prefix('+') {
            new_lines.push(rest.to_owned());
            last = b'+';
        } else if let Some(rest) = line.strip_prefix('-') {
            old_lines.push(rest.to_owned());
            last = b'-';
        } else if let Some(rest) = line.strip_prefix(' ') {
            old_lines.push(rest.to_owned());
            new_lines.push(rest.to_owned());
            last = b' ';
        } else {
            break;
        }
        i += 1;
    }
    let base = if old_absent {
        None
    } else {
        Some(join_lines(&old_lines, old_nl))
    };
    let tip = if new_absent {
        None
    } else {
        Some(join_lines(&new_lines, new_nl))
    };
    Ok((ParsedFile { path, base, tip }, i))
}

fn path_from_git_header(line: &str) -> Result<String, ObjectError> {
    let rest = line
        .strip_prefix("diff --git a/")
        .ok_or_else(|| ObjectError::BadPatch(format!("unrecognized diff header: {line}")))?;
    let Some((left, right)) = rest.split_once(" b/") else {
        return Err(ObjectError::BadPatch(format!(
            "unrecognized diff header: {line}"
        )));
    };
    if left != right {
        return Err(ObjectError::BadPatch(
            "patch renames are not claimed".into(),
        ));
    }
    if left.is_empty() || left.contains('\\') {
        return Err(ObjectError::BadPatch(format!("unsafe patch path '{left}'")));
    }
    Ok(left.to_owned())
}

fn join_lines(lines: &[String], trailing_nl: bool) -> Vec<u8> {
    if lines.is_empty() {
        return Vec::new();
    }
    let mut text = lines.join("\n");
    if trailing_nl {
        text.push('\n');
    }
    text.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_non_mailbox() {
        assert!(parse_patch("not a patch").is_err());
    }
}
