//! `-L <start>,<end>` / `-L :<funcname>` line ranges shared by `blame` and
//! `log -L`: numbers, `+N` / `-N` offsets, `/regex/` (searched after the
//! previous range), `^/regex/` (searched from the top) and funcname ranges
//! (from a function line matching the regex to the line before the next
//! function line, as the default funcname pattern sees them).
//! Primary: <https://git-scm.com/docs/git-blame#_specifying_ranges>,
//! <https://git-scm.com/docs/line-range-format>.

use crate::cmd::bre::Bre;
use crate::error::CliError;

/// How a malformed range is reported: `Usage` for syntax git rejects with
/// the usage text, `Fatal` for the rest.
#[derive(Debug)]
pub enum RangeError {
    Usage,
    Fatal(String),
}

impl RangeError {
    pub fn into_cli(self, usage: &str) -> CliError {
        match self {
            RangeError::Usage => CliError::Exit(129, usage.to_string()),
            RangeError::Fatal(m) => CliError::fatal(m),
        }
    }
}

/// Lines of `content` without their terminators.
#[cfg(test)]
pub fn lines_of(content: &[u8]) -> Vec<&[u8]> {
    if content.is_empty() {
        return Vec::new();
    }
    let body = content.strip_suffix(b"\n").unwrap_or(content);
    body.split(|c| *c == b'\n').collect()
}

/// git's default function-line test: the line starts with a letter, `_` or `$`.
pub fn is_funcname_line(line: &[u8]) -> bool {
    line.first()
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_' || *c == b'$')
}

/// Read `/regex/` (with `\/` escapes) at the start of `s`; returns the
/// pattern and the rest.
fn take_regex(s: &str) -> Option<(String, &str)> {
    let body = s.strip_prefix('/')?;
    let b = body.as_bytes();
    let mut pat = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() && b[i + 1] == b'/' {
            pat.push('/');
            i += 2;
            continue;
        }
        if b[i] == b'/' {
            return Some((pat, &body[i + 1..]));
        }
        let ch = body[i..].chars().next()?;
        pat.push(ch);
        i += ch.len_utf8();
    }
    None
}

fn take_number(s: &str) -> Option<(usize, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    Some((s[..end].parse().ok()?, &s[end..]))
}

fn search(lines: &[&[u8]], pat: &str, from: usize) -> Option<usize> {
    let re = Bre::new(pat, false);
    (from..lines.len()).find(|&i| re.is_match(lines[i]))
}

/// Split a `log -L` argument into its range part and path
/// (`<start>,<end>:<file>` or `:<funcname>:<file>`).
pub fn split_log_arg(arg: &str) -> Option<(String, String)> {
    if let Some(rest) = arg.strip_prefix(':').or_else(|| arg.strip_prefix("^:")) {
        let caret = arg.starts_with('^');
        let colon = rest.find(':')?;
        let range = format!("{}:{}", if caret { "^" } else { "" }, &rest[..colon]);
        return Some((range, rest[colon + 1..].to_string()));
    }
    // Skip <start>[,<end>] then expect ':'.
    let mut s = arg;
    let skip_one = |s: &str| -> Option<usize> {
        let t = s.strip_prefix('^').unwrap_or(s);
        let off = s.len() - t.len();
        if t.starts_with('/') {
            let (_, rest) = take_regex(t)?;
            return Some(s.len() - rest.len());
        }
        let t2 = t.trim_start_matches(['+', '-']);
        let digits = t2.find(|c: char| !c.is_ascii_digit()).unwrap_or(t2.len());
        Some(off + (t.len() - t2.len()) + digits)
    };
    let n = skip_one(s)?;
    s = &s[n..];
    if let Some(rest) = s.strip_prefix(',') {
        let m = skip_one(rest)?;
        s = &rest[m..];
    }
    let path = s.strip_prefix(':')?;
    let range = &arg[..arg.len() - s.len()];
    Some((range.to_string(), path.to_string()))
}

/// Resolve one range spec against `lines`; `anchor` is the 0-based line the
/// previous range ended after (searches start there). Returns 1-based
/// inclusive bounds.
pub fn parse_one(
    spec: &str,
    lines: &[&[u8]],
    anchor: usize,
    path: &str,
) -> Result<(usize, usize), RangeError> {
    let total = lines.len();
    let funcname = spec
        .strip_prefix(':')
        .map(|f| (f, false))
        .or_else(|| spec.strip_prefix("^:").map(|f| (f, true)));
    if let Some((f, from_top)) = funcname {
        if f.contains(':') {
            return Err(RangeError::Usage);
        }
        let start_at = if from_top { 0 } else { anchor };
        let re = Bre::new(f, false);
        let Some(begin) =
            (start_at..total).find(|&i| is_funcname_line(lines[i]) && re.is_match(lines[i]))
        else {
            return Err(RangeError::Fatal(format!(
                "-L parameter '{f}' starting at line {}: no match",
                start_at + 1
            )));
        };
        let end = (begin + 1..total)
            .find(|&i| is_funcname_line(lines[i]))
            .unwrap_or(total);
        return Ok((begin + 1, end));
    }
    // <start>
    let (mut begin, rest): (usize, &str) =
        if let Some(r) = spec.strip_prefix('^').filter(|r| r.starts_with('/')) {
            let (pat, rest) = take_regex(r).ok_or(RangeError::Usage)?;
            let hit = search(lines, &pat, 0).ok_or_else(|| {
                RangeError::Fatal(format!("-L parameter '{pat}' starting at line 1: No match"))
            })?;
            (hit + 1, rest)
        } else if spec.starts_with('/') {
            let (pat, rest) = take_regex(spec).ok_or(RangeError::Usage)?;
            let hit = search(lines, &pat, anchor).ok_or_else(|| {
                RangeError::Fatal(format!(
                    "-L parameter '{pat}' starting at line {}: No match",
                    anchor + 1
                ))
            })?;
            (hit + 1, rest)
        } else if spec.starts_with(',') || spec.is_empty() {
            (1, spec)
        } else if let Some(r) = spec.strip_prefix('+') {
            let (n, rest) = take_number(r).ok_or(RangeError::Usage)?;
            (anchor + n, rest)
        } else {
            let (n, rest) = take_number(spec).ok_or(RangeError::Usage)?;
            if n == 0 {
                return Err(RangeError::Fatal("-L invalid line number: 0".into()));
            }
            (n, rest)
        };
    // <end>
    let mut end;
    if rest.is_empty() {
        end = total;
    } else {
        let Some(e) = rest.strip_prefix(',') else {
            return Err(RangeError::Usage);
        };
        if e.is_empty() {
            end = total;
        } else if let Some(r) = e.strip_prefix('+') {
            let (n, tail) = take_number(r).ok_or(RangeError::Usage)?;
            if !tail.is_empty() {
                return Err(RangeError::Usage);
            }
            if n == 0 {
                return Err(RangeError::Fatal("-L invalid empty range".into()));
            }
            end = begin + n - 1;
        } else if let Some(r) = e.strip_prefix('-') {
            let (n, tail) = take_number(r).ok_or(RangeError::Usage)?;
            if !tail.is_empty() {
                return Err(RangeError::Usage);
            }
            if n == 0 {
                return Err(RangeError::Fatal("-L invalid empty range".into()));
            }
            end = begin;
            begin = (begin + 1).saturating_sub(n).max(1);
        } else if e.starts_with('/') {
            let (pat, tail) = take_regex(e).ok_or(RangeError::Usage)?;
            if !tail.is_empty() {
                return Err(RangeError::Usage);
            }
            let hit = search(lines, &pat, begin).ok_or_else(|| {
                RangeError::Fatal(format!(
                    "-L parameter '{pat}' starting at line {}: No match",
                    begin + 1
                ))
            })?;
            end = hit + 1;
        } else {
            let (n, tail) = take_number(e).ok_or(RangeError::Usage)?;
            if !tail.is_empty() {
                return Err(RangeError::Usage);
            }
            if n == 0 {
                return Err(RangeError::Fatal("-L invalid line number: 0".into()));
            }
            end = n;
        }
    }
    if begin > end {
        std::mem::swap(&mut begin, &mut end);
    }
    if begin > total {
        return Err(RangeError::Fatal(format!(
            "file {path} has only {total} line{}",
            if total == 1 { "" } else { "s" }
        )));
    }
    Ok((begin, end.min(total)))
}

/// Resolve every spec in order (each search anchored after the previous
/// range) and return the sorted, merged 1-based ranges.
pub fn parse_all(
    specs: &[String],
    lines: &[&[u8]],
    path: &str,
) -> Result<Vec<(usize, usize)>, RangeError> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut anchor = 0;
    for s in specs {
        let (b, e) = parse_one(s, lines, anchor, path)?;
        anchor = e;
        if b <= e {
            out.push((b, e));
        }
    }
    out.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (b, e) in out {
        match merged.last_mut() {
            Some(last) if b <= last.1 + 1 => last.1 = last.1.max(e),
            _ => merged.push((b, e)),
        }
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "int one(void)\n{\n\treturn 1;\n}\n\nint two(void)\n{\n\tint x = 2;\n\treturn x;\n}\n\nint three(void)\n{\n\treturn 3;\n}\n";

    fn r(specs: &[&str]) -> Vec<(usize, usize)> {
        let lines = lines_of(CODE.as_bytes());
        let specs: Vec<String> = specs.iter().map(|s| s.to_string()).collect();
        parse_all(&specs, &lines, "code.c").unwrap()
    }

    #[test]
    fn shapes() {
        assert_eq!(r(&["/int/,/^}/"]), vec![(1, 4)]);
        assert_eq!(r(&["5,/int/"]), vec![(5, 6)]);
        assert_eq!(r(&["/three/,-2"]), vec![(11, 12)]);
        assert_eq!(r(&["/{/,/{/"]), vec![(2, 7)]);
        assert_eq!(r(&["2,-5"]), vec![(1, 2)]);
        assert_eq!(r(&["3,1"]), vec![(1, 3)]);
        assert_eq!(r(&["14,100"]), vec![(14, 15)]);
        assert_eq!(r(&[":two"]), vec![(6, 11)]);
        assert_eq!(r(&["6,7", ":int"]), vec![(6, 7), (12, 15)]);
        assert_eq!(r(&["6,7", "^:int"]), vec![(1, 7)]);
        assert_eq!(r(&["2,3", "^/return/,+1"]), vec![(2, 3)]);
        assert_eq!(r(&["+2"]), vec![(2, 15)]);
    }

    #[test]
    fn errors() {
        let lines = lines_of(CODE.as_bytes());
        assert!(matches!(
            parse_one("x,1", &lines, 0, "c"),
            Err(RangeError::Usage)
        ));
        assert!(matches!(
            parse_one("3,+", &lines, 0, "c"),
            Err(RangeError::Usage)
        ));
        assert!(matches!(
            parse_one("0", &lines, 0, "c"),
            Err(RangeError::Fatal(_))
        ));
        assert!(matches!(
            parse_one("/two/,+0", &lines, 0, "c"),
            Err(RangeError::Fatal(_))
        ));
        assert!(matches!(
            parse_one("20,30", &lines, 0, "c"),
            Err(RangeError::Fatal(_))
        ));
    }

    #[test]
    fn log_args() {
        assert_eq!(
            split_log_arg("1,3:src/a.txt"),
            Some(("1,3".into(), "src/a.txt".into()))
        );
        assert_eq!(
            split_log_arg(":two:code.c"),
            Some((":two".into(), "code.c".into()))
        );
        assert_eq!(
            split_log_arg("/a:b/,+2:x"),
            Some(("/a:b/,+2".into(), "x".into()))
        );
    }
}
