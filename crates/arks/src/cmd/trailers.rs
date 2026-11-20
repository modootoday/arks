//! Commit-message trailers and message cleanup (`interpret-trailers`,
//! `commit --trailer`, `--cleanup`, `stripspace` rules).
//!
//! Behaviour follows git-interpret-trailers(1): the trailer block is the last
//! paragraph of the message (never the title paragraph), before any trailing
//! comment / blank lines and, unless `--no-divider`, before a `---` patch
//! divider; it is a trailer block when every line is a trailer, or when at
//! least 25% are and one is git-generated (`Signed-off-by: `,
//! `(cherry picked from commit `). Existing trailers are re-printed as
//! `token: value`. New trailers apply `--where`, `--if-exists` and
//! `--if-missing` (each applies to the `--trailer` options after it); token
//! comparison is case-insensitive over the shorter token, value comparison
//! is case-insensitive.
//! Primary: <https://git-scm.com/docs/git-interpret-trailers>,
//! <https://git-scm.com/docs/git-commit#Documentation/git-commit.txt---cleanupltmodegt>,
//! <https://git-scm.com/docs/git-stripspace>.

use crate::error::{CliError, CliResult};

/// Where a new trailer goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    End,
    Start,
    After,
    Before,
}

/// What to do when a trailer with the same token exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IfExists {
    AddIfDifferentNeighbor,
    AddIfDifferent,
    Add,
    Replace,
    DoNothing,
}

/// What to do when no trailer with the same token exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IfMissing {
    Add,
    DoNothing,
}

/// One `--trailer` argument with the placement rules in force for it.
#[derive(Clone, Debug)]
pub struct NewTrailer {
    pub token: String,
    pub value: String,
    pub place: Where,
    pub if_exists: IfExists,
    pub if_missing: IfMissing,
}

/// Output switches of interpret-trailers.
#[derive(Clone, Copy, Debug, Default)]
pub struct Opts {
    pub trim_empty: bool,
    pub only_trailers: bool,
    pub only_input: bool,
    pub unfold: bool,
    pub no_divider: bool,
}

pub fn parse_where(v: &str) -> CliResult<Where> {
    Ok(match v.to_ascii_lowercase().as_str() {
        "end" => Where::End,
        "start" => Where::Start,
        "after" => Where::After,
        "before" => Where::Before,
        _ => return Err(unknown_value("where", v)),
    })
}

pub fn parse_if_exists(v: &str) -> CliResult<IfExists> {
    Ok(match v.to_ascii_lowercase().as_str() {
        "addifdifferentneighbor" => IfExists::AddIfDifferentNeighbor,
        "addifdifferent" => IfExists::AddIfDifferent,
        "add" => IfExists::Add,
        "replace" => IfExists::Replace,
        "donothing" => IfExists::DoNothing,
        _ => return Err(unknown_value("if-exists", v)),
    })
}

pub fn parse_if_missing(v: &str) -> CliResult<IfMissing> {
    Ok(match v.to_ascii_lowercase().as_str() {
        "add" => IfMissing::Add,
        "donothing" => IfMissing::DoNothing,
        _ => return Err(unknown_value("if-missing", v)),
    })
}

fn unknown_value(opt: &str, v: &str) -> CliError {
    CliError::Exit(
        129,
        format!("error: unknown value for --{opt}: {v}\nusage: git interpret-trailers [<options>]"),
    )
}

/// Split a `--trailer` argument (`token=value`, `token: value`, `token`).
pub fn split_arg(arg: &str) -> CliResult<(String, String)> {
    let pos = arg.find(['=', ':']);
    let (tok, val) = match pos {
        Some(i) => (&arg[..i], &arg[i + 1..]),
        None => (arg, ""),
    };
    let tok = tok.trim();
    if tok.is_empty() {
        return Err(CliError::fatal(format!(
            "empty trailer token in trailer '{arg}'"
        )));
    }
    Ok((tok.to_string(), val.trim().to_string()))
}

#[derive(Clone, Debug)]
enum Entry {
    Trailer { token: String, value: String },
    Other(String),
}

fn token_of(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let tok = line[..colon].trim_end_matches([' ', '\t']);
    if tok.is_empty() || !tok.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }
    Some((tok.to_string(), line[colon + 1..].trim().to_string()))
}

fn is_git_generated(line: &str) -> bool {
    line.starts_with("Signed-off-by: ") || line.starts_with("(cherry picked from commit ")
}

fn is_blank(l: &str) -> bool {
    l.trim().is_empty()
}

/// Lines of `s` keeping their terminators.
fn lines_keep(s: &str) -> Vec<&str> {
    s.split_inclusive('\n').collect()
}

struct Layout<'a> {
    /// Message before the trailer block (or the whole message body).
    head: String,
    /// Trailer block entries (empty when there is none).
    block: Vec<Entry>,
    has_block: bool,
    /// Trailing comment / blank lines and the patch part, verbatim.
    tail: &'a str,
}

fn layout<'a>(input: &'a str, opts: &Opts) -> Layout<'a> {
    // Patch divider: a line starting with `---` followed by whitespace or EOL.
    let mut msg_end = input.len();
    if !opts.no_divider {
        let mut off = 0;
        for l in lines_keep(input) {
            if let Some(rest) = l.strip_prefix("---") {
                if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                    msg_end = off;
                    break;
                }
            }
            off += l.len();
        }
    }
    let msg = &input[..msg_end];
    let lines = lines_keep(msg);
    // Trailing comment and blank lines are not part of the message proper.
    let mut end = lines.len();
    while end > 0 && (is_blank(lines[end - 1]) || lines[end - 1].starts_with('#')) {
        end -= 1;
    }
    let body_len: usize = lines[..end].iter().map(|l| l.len()).sum();
    let tail = &input[body_len..];
    // The title paragraph never holds trailers.
    let mut title_end = 0;
    while title_end < end && !is_blank(lines[title_end]) {
        title_end += 1;
    }
    let mut start = end;
    while start > title_end && !is_blank(lines[start - 1]) {
        start -= 1;
    }
    let mut has_block = false;
    let mut block = Vec::new();
    if start > title_end && start < end {
        let para = &lines[start..end];
        let (mut trailers, mut others, mut generated) = (0usize, 0usize, false);
        let mut prev_trailer = false;
        for l in para {
            let t = l.trim_end_matches('\n');
            if t.starts_with('#') {
                continue;
            }
            if (t.starts_with(' ') || t.starts_with('\t')) && prev_trailer {
                continue;
            }
            if token_of(t).is_some() || t.starts_with("(cherry picked from commit ") {
                trailers += 1;
                prev_trailer = true;
                generated |= is_git_generated(t);
            } else {
                others += 1;
                prev_trailer = false;
            }
        }
        if trailers > 0 && (others == 0 || (generated && trailers * 3 >= others)) {
            has_block = true;
            for l in para {
                let t = l.trim_end_matches('\n');
                if (t.starts_with(' ') || t.starts_with('\t'))
                    && matches!(block.last(), Some(Entry::Trailer { .. }))
                {
                    if let Some(Entry::Trailer { value, .. }) = block.last_mut() {
                        if opts.unfold {
                            value.push(' ');
                            value.push_str(t.trim());
                        } else {
                            value.push('\n');
                            value.push_str(t);
                        }
                    }
                    continue;
                }
                match token_of(t) {
                    Some((token, value)) if !t.starts_with('#') => {
                        block.push(Entry::Trailer { token, value })
                    }
                    _ => block.push(Entry::Other(t.to_string())),
                }
            }
        }
    }
    let head_end = if has_block { start } else { end };
    let head: String = lines[..head_end].concat();
    Layout {
        head,
        block,
        has_block,
        tail,
    }
}

fn same_token(a: &str, b: &str) -> bool {
    let n = a.len().min(b.len());
    a.as_bytes()[..n].eq_ignore_ascii_case(&b.as_bytes()[..n])
}

fn same_trailer(e: &Entry, tok: &str, val: &str) -> bool {
    matches!(e, Entry::Trailer { token, value }
        if same_token(token, tok) && value.eq_ignore_ascii_case(val))
}

fn apply(block: &mut Vec<Entry>, t: &NewTrailer) {
    let new = Entry::Trailer {
        token: t.token.clone(),
        value: t.value.clone(),
    };
    let matching: Vec<usize> = block
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Entry::Trailer { token, .. } if same_token(token, &t.token)))
        .map(|(i, _)| i)
        .collect();
    let after_like = matches!(t.place, Where::After | Where::End);
    let Some(&in_tok) = (if after_like {
        matching.last()
    } else {
        matching.first()
    }) else {
        if t.if_missing == IfMissing::Add {
            if after_like {
                block.push(new);
            } else {
                block.insert(0, new);
            }
        }
        return;
    };
    // Insertion point: after / before the matching item, or at the ends.
    let insert_at = |block: &Vec<Entry>| match t.place {
        Where::After => in_tok + 1,
        Where::Before => in_tok,
        Where::End => block.len(),
        Where::Start => 0,
    };
    match t.if_exists {
        IfExists::DoNothing => {}
        IfExists::Add => {
            let at = insert_at(block);
            block.insert(at, new);
        }
        IfExists::AddIfDifferent => {
            if !block.iter().any(|e| same_trailer(e, &t.token, &t.value)) {
                let at = insert_at(block);
                block.insert(at, new);
            }
        }
        IfExists::AddIfDifferentNeighbor => {
            let neighbor = match t.place {
                Where::End => block.last(),
                Where::Start => block.first(),
                _ => block.get(in_tok),
            };
            if !neighbor.is_some_and(|e| same_trailer(e, &t.token, &t.value)) {
                let at = insert_at(block);
                block.insert(at, new);
            }
        }
        IfExists::Replace => {
            let at = insert_at(block);
            block.insert(at, new);
            let old = if at <= in_tok { in_tok + 1 } else { in_tok };
            block.remove(old);
        }
    }
}

fn render_entry(e: &Entry) -> String {
    match e {
        Entry::Trailer { token, value } => format!("{token}: {value}\n"),
        Entry::Other(l) => format!("{l}\n"),
    }
}

/// Process one message the way `git interpret-trailers` does.
pub fn process(input: &str, new: &[NewTrailer], opts: &Opts) -> String {
    let lay = layout(input, opts);
    let mut block = lay.block;
    if !opts.only_input {
        for t in new {
            apply(&mut block, t);
        }
    }
    if opts.trim_empty {
        block.retain(|e| !matches!(e, Entry::Trailer { value, .. } if value.trim().is_empty()));
    }
    if opts.only_trailers {
        return block
            .iter()
            .filter(|e| matches!(e, Entry::Trailer { .. }))
            .map(render_entry)
            .collect();
    }
    let mut out = lay.head;
    if !lay.has_block {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        // A blank line separates new trailers from the message.
        if out.is_empty() || !out.trim_end_matches('\n').is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }
    }
    for e in &block {
        out.push_str(&render_entry(e));
    }
    out.push_str(lay.tail);
    out
}

/// git `stripspace`: drop trailing whitespace, collapse runs of blank lines,
/// drop leading and trailing blank lines; with `strip_comments`, drop lines
/// starting with `#`. Non-empty output ends with a newline.
pub fn stripspace(text: &str, strip_comments: bool) -> String {
    let mut out = String::new();
    let mut pending_blank = false;
    for line in text.lines() {
        if strip_comments && line.starts_with('#') {
            continue;
        }
        let l = line.trim_end();
        if l.is_empty() {
            pending_blank = !out.is_empty();
            continue;
        }
        if pending_blank {
            out.push('\n');
            pending_blank = false;
        }
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// Commit message cleanup modes (`--cleanup=<mode>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cleanup {
    Strip,
    Whitespace,
    Verbatim,
    Scissors,
    Default,
}

pub fn parse_cleanup(v: &str) -> CliResult<Cleanup> {
    Ok(match v {
        "strip" => Cleanup::Strip,
        "whitespace" => Cleanup::Whitespace,
        "verbatim" => Cleanup::Verbatim,
        "scissors" => Cleanup::Scissors,
        "default" => Cleanup::Default,
        other => return Err(CliError::fatal(format!("Invalid cleanup mode {other}"))),
    })
}

/// Apply a cleanup mode; `edited` = the message went through the editor
/// (`default` then strips comments, otherwise only whitespace).
pub fn cleanup(msg: &str, mode: Cleanup, edited: bool) -> String {
    const SCISSORS: &str = "# ------------------------ >8 ------------------------";
    match mode {
        Cleanup::Verbatim => msg.to_string(),
        Cleanup::Whitespace => stripspace(msg, false),
        Cleanup::Strip => stripspace(msg, true),
        Cleanup::Default => stripspace(msg, edited),
        Cleanup::Scissors => {
            if !edited {
                return stripspace(msg, false);
            }
            let cut = msg
                .split_inclusive('\n')
                .scan(0usize, |off, l| {
                    let at = *off;
                    *off += l.len();
                    Some((at, l))
                })
                .find(|(_, l)| l.trim_end() == SCISSORS)
                .map_or(msg.len(), |(at, _)| at);
            stripspace(&msg[..cut], false)
        }
    }
}

/// `interpret-trailers [<options>] [(--trailer <token>[(=|:)<value>])...] [<file>...]`.
pub fn cmd_interpret_trailers(args: &[String]) -> CliResult<()> {
    use crate::cmd::opt_parse::{Item, Mode, parse};
    use std::io::{Read, Write};
    let p = parse(
        "interpret-trailers",
        args,
        crate::cmd::porcelain_opts::INTERPRET_TRAILERS,
        Mode::default(),
    )?;
    let mut place = Where::End;
    let mut if_exists = IfExists::AddIfDifferentNeighbor;
    let mut if_missing = IfMissing::Add;
    let mut new = Vec::new();
    for it in &p.items {
        let Item::Opt { key, val, neg } = it else {
            continue;
        };
        let v = val.as_deref().unwrap_or("");
        match (*key, *neg) {
            ("where", false) => place = parse_where(v)?,
            ("where", true) => place = Where::End,
            ("if-exists", false) => if_exists = parse_if_exists(v)?,
            ("if-exists", true) => if_exists = IfExists::AddIfDifferentNeighbor,
            ("if-missing", false) => if_missing = parse_if_missing(v)?,
            ("if-missing", true) => if_missing = IfMissing::Add,
            ("trailer", false) => {
                let Ok((token, value)) = split_arg(v) else {
                    eprintln!("error: empty trailer token in trailer '{v}'");
                    continue;
                };
                new.push(NewTrailer {
                    token,
                    value,
                    place,
                    if_exists,
                    if_missing,
                });
            }
            ("trailer", true) => new.clear(),
            _ => {}
        }
    }
    let parse_mode = p.on("parse");
    let opts = Opts {
        trim_empty: p.on("trim-empty"),
        only_trailers: p.on("only-trailers") || parse_mode,
        only_input: p.on("only-input") || parse_mode,
        unfold: p.on("unfold") || parse_mode,
        no_divider: p.on("no-divider"),
    };
    if opts.only_input && !new.is_empty() {
        return Err(CliError::fatal(
            "--trailer with --only-input does not make sense",
        ));
    }
    let in_place = p.on("in-place");
    let mut files = p.args();
    files.extend(p.after_dashdash());
    if files.is_empty() {
        if in_place {
            return Err(CliError::fatal("no input file given for in-place editing"));
        }
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        print!("{}", process(&buf, &new, &opts));
        return Ok(());
    }
    let mut out = std::io::stdout().lock();
    for f in &files {
        let text = std::fs::read_to_string(f)
            .map_err(|e| CliError::fatal(format!("could not open '{f}' for reading: {e}")))?;
        let result = process(&text, &new, &opts);
        if in_place {
            std::fs::write(f, result)
                .map_err(|e| CliError::fatal(format!("could not write '{f}': {e}")))?;
        } else {
            out.write_all(result.as_bytes())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add(tok: &str, val: &str) -> NewTrailer {
        NewTrailer {
            token: tok.into(),
            value: val.into(),
            place: Where::End,
            if_exists: IfExists::AddIfDifferentNeighbor,
            if_missing: IfMissing::Add,
        }
    }

    #[test]
    fn appends_after_blank_line() {
        let o = Opts::default();
        assert_eq!(
            process("s\n\nbody\n", &[add("A", "b")], &o),
            "s\n\nbody\n\nA: b\n"
        );
        assert_eq!(process("s\n", &[add("A", "b")], &o), "s\n\nA: b\n");
        assert_eq!(process("", &[add("A", "b")], &o), "\nA: b\n");
        assert_eq!(
            process("s\n\nKey: v\nOther : w\n\n# c\n", &[add("A", "b")], &o),
            "s\n\nKey: v\nOther: w\nA: b\n\n# c\n"
        );
    }

    #[test]
    fn neighbor_rule_and_divider() {
        let o = Opts::default();
        assert_eq!(
            process("s\n\nAcked-by: B\n", &[add("A", "b")], &o),
            "s\n\nAcked-by: B\n"
        );
        assert_eq!(
            process("s\n\nbody\n---\n x\n", &[add("A", "b")], &o),
            "s\n\nbody\n\nA: b\n---\n x\n"
        );
    }

    #[test]
    fn stripspace_rules() {
        assert_eq!(stripspace("\n\na  \n\n\n# c\nb\n\n", true), "a\n\nb\n");
        assert_eq!(stripspace("# c\n", false), "# c\n");
    }
}
