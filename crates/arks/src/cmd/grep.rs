//! `grep`: search tracked files in the working tree, the index (`--cached`),
//! trees (`<rev>...`), untracked files too (`--untracked`) or a plain
//! directory (`--no-index`).
//!
//! Output per git-grep: `<path>:<line>` with optional line (`-n`) and
//! column (`--column`) numbers, `<rev>:` prefixes for tree hits, context
//! lines (`-A`/`-B`/`-C`/`-<n>`, `-` separators, `--` between groups),
//! function lines (`-p` with `=`, `-W` whole functions), `--heading` /
//! `--break` layout, `-z` NUL-terminated names, `-c` counts, `-l`/`-L`
//! names, `-o` matched parts, `--color=always`, `Binary file <path> matches`
//! for binary blobs. Paths are shown relative to (and limited to) the
//! current directory unless `--full-name`. Patterns are basic (`-G`,
//! default), extended (`-E`), Perl-style (`-P`, ERE plus `\d`) or fixed
//! (`-F`), read from `-e` / `-f`, and combined with `--and`, `--or`,
//! `--not` and parentheses; `--all-match` keeps files matching every
//! alternative. Exit status 1 when nothing matched.
//! Primary: <https://git-scm.com/docs/git-grep>.

use crate::cmd::bre::Bre;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use std::collections::HashSet;
use std::fs;
use std::io::Write;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Syntax {
    #[default]
    Basic,
    Extended,
    Fixed,
    Perl,
}

#[derive(Default)]
struct Opts {
    line_numbers: bool,
    column: bool,
    icase: bool,
    count: bool,
    files_with: bool,
    files_without: bool,
    word: bool,
    invert: bool,
    only: bool,
    no_filename: bool,
    quiet: bool,
    syntax: Syntax,
    cached: bool,
    no_index: bool,
    untracked: bool,
    exclude_standard: Option<bool>,
    text: bool,
    skip_binary: bool,
    max_depth: Option<i64>,
    full_name: bool,
    null: bool,
    color: bool,
    brk: bool,
    heading: bool,
    before: usize,
    after: usize,
    show_function: bool,
    function_context: bool,
    all_match: bool,
    max_count: Option<usize>,
    pager: Option<Option<String>>,
}

/// Pattern expression tokens in argv order.
#[derive(Clone, Debug)]
enum Tok {
    Pat(String),
    And,
    Or,
    Not,
    Open,
    Close,
}

#[derive(Debug)]
enum Expr {
    Pat(usize),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

struct ExprParser<'a> {
    toks: &'a [Tok],
    i: usize,
    pats: Vec<String>,
}

impl ExprParser<'_> {
    fn or(&mut self) -> CliResult<Expr> {
        let mut l = self.and()?;
        loop {
            match self.toks.get(self.i) {
                Some(Tok::Or) => {
                    self.i += 1;
                    let r = self.and()?;
                    l = Expr::Or(Box::new(l), Box::new(r));
                }
                // Adjacent patterns are alternatives.
                Some(Tok::Pat(_)) | Some(Tok::Not) | Some(Tok::Open) => {
                    let r = self.and()?;
                    l = Expr::Or(Box::new(l), Box::new(r));
                }
                _ => return Ok(l),
            }
        }
    }

    fn and(&mut self) -> CliResult<Expr> {
        let mut l = self.unary()?;
        while matches!(self.toks.get(self.i), Some(Tok::And)) {
            self.i += 1;
            let r = self.unary()?;
            l = Expr::And(Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn unary(&mut self) -> CliResult<Expr> {
        match self.toks.get(self.i).cloned() {
            Some(Tok::Not) => {
                self.i += 1;
                Ok(Expr::Not(Box::new(self.unary()?)))
            }
            Some(Tok::Open) => {
                self.i += 1;
                let e = self.or()?;
                if !matches!(self.toks.get(self.i), Some(Tok::Close)) {
                    return Err(CliError::fatal("unmatched ( for expression group"));
                }
                self.i += 1;
                Ok(e)
            }
            Some(Tok::Pat(p)) => {
                self.i += 1;
                self.pats.push(p);
                Ok(Expr::Pat(self.pats.len() - 1))
            }
            Some(Tok::Close) => Err(CliError::fatal("unmatched ) for expression group")),
            _ => Err(CliError::fatal("incomplete pattern expression group")),
        }
    }
}

impl Expr {
    fn eval(&self, hit: &dyn Fn(usize) -> bool) -> bool {
        match self {
            Expr::Pat(i) => hit(*i),
            Expr::Not(e) => !e.eval(hit),
            Expr::And(a, b) => a.eval(hit) && b.eval(hit),
            Expr::Or(a, b) => a.eval(hit) || b.eval(hit),
        }
    }

    /// Top-level alternatives (for `--all-match`).
    fn alternatives(&self) -> Vec<&Expr> {
        match self {
            Expr::Or(a, b) => {
                let mut v = a.alternatives();
                v.extend(b.alternatives());
                v
            }
            e => vec![e],
        }
    }

    /// Patterns that appear without negation (their hits are shown).
    fn positive(&self, neg: bool, out: &mut Vec<usize>) {
        match self {
            Expr::Pat(i) if !neg => out.push(*i),
            Expr::Pat(_) => {}
            Expr::Not(e) => e.positive(!neg, out),
            Expr::And(a, b) | Expr::Or(a, b) => {
                a.positive(neg, out);
                b.positive(neg, out);
            }
        }
    }
}

fn escape_fixed(p: &str) -> String {
    let mut out = String::new();
    for c in p.chars() {
        if matches!(c, '.' | '[' | ']' | '\\' | '*' | '^' | '$') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `-P` patterns: Perl's `\d` / `\D` on top of extended syntax.
fn perl_to_ere(p: &str) -> String {
    p.replace("\\d", "[0-9]").replace("\\D", "[^0-9]")
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Matches of one pattern in `line` (`-w` enforces word boundaries).
fn pat_matches(p: &Bre, line: &[u8], word: bool) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut pos = 0;
    while pos <= line.len() {
        let mut from = pos;
        let mut found = None;
        while let Some((s, e)) = p.find_from(line, from) {
            let ok = !word
                || ((s == 0 || !is_word(line[s - 1]))
                    && (e >= line.len() || !is_word(line[e]))
                    && e > s);
            if ok {
                found = Some((s, e));
                break;
            }
            from = s + 1;
            if from > line.len() {
                break;
            }
        }
        let Some((s, e)) = found else { break };
        out.push((s, e));
        pos = if e > s { e } else { e + 1 };
    }
    out
}

/// Merged, ordered hit spans of several patterns.
fn union_hits(mut v: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    v.sort();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (s, e) in v {
        match out.last_mut() {
            Some(l) if s < l.1 => l.1 = l.1.max(e),
            _ => out.push((s, e)),
        }
    }
    out
}

/// Default function-header rule: a line starting with a letter, `_` or `$`.
fn is_funcname(line: &[u8]) -> bool {
    line.first()
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_' || *c == b'$')
}

fn parse_num(v: &str, what: &str) -> CliResult<usize> {
    v.parse::<usize>().map_err(|_| {
        CliError::Exit(
            129,
            format!("error: option `{what}' expects a numerical value"),
        )
    })
}

struct Parsed {
    o: Opts,
    toks: Vec<Tok>,
    rest: Vec<String>,
    paths: Vec<String>,
    dashdash: bool,
}

/// Split bundled short options (`-in`, `-C2`, `-e<pat>`, `-3`) into
/// separate arguments; values of valued options are left alone.
fn expand_bundles(args: &[String]) -> Vec<String> {
    const VALUED_SHORT: &str = "ABCefm";
    const VALUED_LONG: [&str; 8] = [
        "--after-context",
        "--before-context",
        "--context",
        "--max-count",
        "--max-depth",
        "--threads",
        "-e",
        "-f",
    ];
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == "--" {
            out.extend(args[i - 1..].iter().cloned());
            break;
        }
        if VALUED_LONG.contains(&a.as_str()) || ["-A", "-B", "-C", "-m"].contains(&a.as_str()) {
            out.push(a.clone());
            if let Some(v) = args.get(i) {
                out.push(v.clone());
                i += 1;
            }
            continue;
        }
        if a.len() >= 2 && a.starts_with('-') && a[1..].bytes().all(|b| b.is_ascii_digit()) {
            out.push("-C".into());
            out.push(a[1..].to_string());
            continue;
        }
        if !a.starts_with('-') || a.starts_with("--") || a.len() <= 2 {
            out.push(a.clone());
            continue;
        }
        let body = &a[1..];
        if body.bytes().all(|b| b.is_ascii_digit()) {
            out.push("-C".into());
            out.push(body.to_string());
            continue;
        }
        for (k, c) in body.char_indices() {
            if VALUED_SHORT.contains(c) {
                out.push(format!("-{c}"));
                if k + 1 < body.len() {
                    out.push(body[k + 1..].to_string());
                } else if let Some(v) = args.get(i) {
                    out.push(v.clone());
                    i += 1;
                }
                break;
            }
            if c == 'O' {
                out.push(format!("-O{}", &body[k + 1..]));
                break;
            }
            out.push(format!("-{c}"));
        }
    }
    out
}

fn parse(args: &[String]) -> CliResult<Parsed> {
    let args = &expand_bundles(args);
    let mut o = Opts {
        max_depth: None,
        ..Default::default()
    };
    let mut toks: Vec<Tok> = Vec::new();
    let mut rest: Vec<String> = Vec::new();
    let mut paths: Vec<String> = Vec::new();
    let mut dashdash = false;
    let mut i = 0;
    let need = |i: usize, name: &str| -> CliResult<String> {
        args.get(i)
            .cloned()
            .ok_or_else(|| CliError::Exit(129, format!("error: switch `{name}' requires a value")))
    };
    while i < args.len() {
        let a = args[i].clone();
        i += 1;
        if dashdash {
            paths.push(a);
            continue;
        }
        match a.as_str() {
            "--" => dashdash = true,
            "-n" | "--line-number" => o.line_numbers = true,
            "--no-line-number" => o.line_numbers = false,
            "--column" => o.column = true,
            "--no-column" => o.column = false,
            "-i" | "--ignore-case" => o.icase = true,
            "--no-ignore-case" => o.icase = false,
            "-c" | "--count" => o.count = true,
            "--no-count" => o.count = false,
            "-l" | "--files-with-matches" | "--name-only" => o.files_with = true,
            "--no-files-with-matches" | "--no-name-only" => o.files_with = false,
            "-L" | "--files-without-match" => o.files_without = true,
            "--no-files-without-match" => o.files_without = false,
            "-w" | "--word-regexp" => o.word = true,
            "--no-word-regexp" => o.word = false,
            "-v" | "--invert-match" => o.invert = true,
            "--no-invert-match" => o.invert = false,
            "-o" | "--only-matching" => o.only = true,
            "--no-only-matching" => o.only = false,
            "-h" => o.no_filename = true,
            "-H" => o.no_filename = false,
            "-q" | "--quiet" => o.quiet = true,
            "--no-quiet" => o.quiet = false,
            "-E" | "--extended-regexp" => o.syntax = Syntax::Extended,
            "-F" | "--fixed-strings" => o.syntax = Syntax::Fixed,
            "-P" | "--perl-regexp" => o.syntax = Syntax::Perl,
            "-G" | "--basic-regexp" => o.syntax = Syntax::Basic,
            "--no-extended-regexp"
            | "--no-fixed-strings"
            | "--no-perl-regexp"
            | "--no-basic-regexp" => o.syntax = Syntax::Basic,
            "-I" => o.skip_binary = true,
            "-a" | "--text" => o.text = true,
            "--no-text" => o.text = false,
            "--textconv" | "--no-textconv" => {}
            "--cached" => o.cached = true,
            "--no-cached" => o.cached = false,
            "--no-index" => o.no_index = true,
            "--index" => o.no_index = false,
            "--untracked" => o.untracked = true,
            "--no-untracked" => o.untracked = false,
            "--exclude-standard" => o.exclude_standard = Some(true),
            "--no-exclude-standard" => o.exclude_standard = Some(false),
            "--recurse-submodules" | "--no-recurse-submodules" => {}
            "-r" | "--recursive" => o.max_depth = None,
            "--no-recursive" => o.max_depth = Some(0),
            "--full-name" => o.full_name = true,
            "--no-full-name" => o.full_name = false,
            "-z" | "--null" => o.null = true,
            "--no-null" => o.null = false,
            "--color" => o.color = true,
            "--no-color" => o.color = false,
            "--break" => o.brk = true,
            "--no-break" => o.brk = false,
            "--heading" => o.heading = true,
            "--no-heading" => o.heading = false,
            "--no-context" => {
                o.before = 0;
                o.after = 0;
            }
            "--no-before-context" => o.before = 0,
            "--no-after-context" => o.after = 0,
            "--no-threads" => {}
            "-p" | "--show-function" => o.show_function = true,
            "--no-show-function" => o.show_function = false,
            "-W" | "--function-context" => o.function_context = true,
            "--no-function-context" => o.function_context = false,
            "--and" => toks.push(Tok::And),
            "--or" => toks.push(Tok::Or),
            "--not" => toks.push(Tok::Not),
            "(" => toks.push(Tok::Open),
            ")" => toks.push(Tok::Close),
            "--all-match" => o.all_match = true,
            "--no-all-match" => o.all_match = false,
            "-O" | "--open-files-in-pager" => o.pager = Some(None),
            "--no-open-files-in-pager" => o.pager = None,
            "--ext-grep" | "--no-ext-grep" => {}
            // git's negated integer option leaves a limit of zero.
            "--no-max-count" => o.max_count = Some(0),
            "-e" => {
                toks.push(Tok::Pat(need(i, "e")?));
                i += 1;
            }
            "-f" => {
                let file = need(i, "f")?;
                i += 1;
                let text = if file == "-" {
                    let mut s = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
                    s
                } else {
                    fs::read_to_string(&file)
                        .map_err(|e| CliError::fatal(format!("cannot open '{file}': {e}")))?
                };
                for l in text.lines() {
                    toks.push(Tok::Pat(l.to_string()));
                }
            }
            "-A" | "--after-context" => {
                o.after = parse_num(&need(i, "A")?, "A")?;
                i += 1;
            }
            "-B" | "--before-context" => {
                o.before = parse_num(&need(i, "B")?, "B")?;
                i += 1;
            }
            "-C" | "--context" => {
                let n = parse_num(&need(i, "C")?, "C")?;
                o.before = n;
                o.after = n;
                i += 1;
            }
            "-m" | "--max-count" => {
                o.max_count = Some(parse_num(&need(i, "m")?, "m")?);
                i += 1;
            }
            "--max-depth" => {
                let v = need(i, "max-depth")?;
                o.max_depth = Some(v.parse::<i64>().map_err(|_| {
                    CliError::Exit(
                        129,
                        "error: option `max-depth' expects a numerical value".into(),
                    )
                })?)
                .filter(|d| *d >= 0);
                i += 1;
            }
            "--threads" => {
                need(i, "threads")?;
                i += 1;
            }
            x => {
                let valued = [
                    ("--after-context=", 'A'),
                    ("--before-context=", 'B'),
                    ("--context=", 'C'),
                    ("--max-count=", 'm'),
                ];
                if let Some((p, c)) = valued.iter().find(|(p, _)| x.starts_with(p)) {
                    let n = parse_num(&x[p.len()..], &c.to_string())?;
                    match c {
                        'A' => o.after = n,
                        'B' => o.before = n,
                        'C' => {
                            o.before = n;
                            o.after = n;
                        }
                        _ => o.max_count = Some(n),
                    }
                } else if let Some(v) = x.strip_prefix("--max-depth=") {
                    o.max_depth = v.parse::<i64>().ok().filter(|d| *d >= 0);
                } else if x.starts_with("--threads=") {
                } else if let Some(v) = x.strip_prefix("--color=") {
                    o.color = match v {
                        "always" | "true" | "yes" => true,
                        "never" | "false" | "no" | "auto" => false,
                        _ => {
                            return Err(CliError::fatal(format!(
                                "unknown color value '{v}' for --color"
                            )));
                        }
                    };
                } else if let Some(v) = x
                    .strip_prefix("--open-files-in-pager=")
                    .or_else(|| x.strip_prefix("-O").filter(|v| !v.is_empty()))
                {
                    o.pager = Some(Some(v.to_string()));
                } else if let Some(v) = x.strip_prefix("-e").filter(|v| !v.is_empty()) {
                    toks.push(Tok::Pat(v.to_string()));
                } else if x.starts_with('-') && x.len() > 1 {
                    return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
                } else {
                    rest.push(a);
                }
            }
        }
    }
    Ok(Parsed {
        o,
        toks,
        rest,
        paths,
        dashdash,
    })
}

const C_FILE: &str = "\x1b[35m";
const C_SEP: &str = "\x1b[36m";
const C_LNO: &str = "\x1b[32m";
const C_MATCH: &str = "\x1b[1;31m";
const C_RESET: &str = "\x1b[m";

struct Out<'a> {
    o: &'a Opts,
    w: std::io::BufWriter<std::io::StdoutLock<'static>>,
}

impl Out<'_> {
    fn paint(&mut self, color: &str, text: &[u8]) -> CliResult<()> {
        if self.o.color && !text.is_empty() {
            self.w.write_all(color.as_bytes())?;
            self.w.write_all(text)?;
            self.w.write_all(C_RESET.as_bytes())?;
        } else {
            self.w.write_all(text)?;
        }
        Ok(())
    }

    fn sep(&mut self, s: u8) -> CliResult<()> {
        self.paint(C_SEP, &[s])
    }

    /// File name followed by the separator (or NUL with `-z`).
    fn name(&mut self, name: &str, sep: u8) -> CliResult<()> {
        self.paint(C_FILE, name.as_bytes())?;
        if self.o.null {
            self.w.write_all(b"\0")?;
        } else {
            self.sep(sep)?;
        }
        Ok(())
    }
}

/// One searched file: display name and bytes.
struct Source {
    name: String,
    bytes: Vec<u8>,
}

fn depth_ok(path: &str, specs: &[String], max: Option<i64>) -> bool {
    let Some(max) = max else { return true };
    let rel_depth = |rel: &str| rel.matches('/').count() as i64;
    if specs.is_empty() {
        return rel_depth(path) <= max;
    }
    specs.iter().any(|s| {
        if s.is_empty() {
            rel_depth(path) <= max
        } else if path == s {
            true
        } else if let Some(r) = path.strip_prefix(&format!("{s}/")) {
            rel_depth(r) <= max
        } else {
            false
        }
    })
}

/// Files below `dir` for `--no-index` (paths relative to `dir`).
fn walk_dir(
    dir: &std::path::Path,
    rel: &str,
    ignore: &mut Option<crate::cmd::ignore::Ignore>,
    ig_base: &str,
    out: &mut Vec<String>,
) -> CliResult<()> {
    let mut ents: Vec<_> = fs::read_dir(dir)?.flatten().collect();
    ents.sort_by_key(|e| e.file_name());
    for e in ents {
        let name = e.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        let r = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let is_dir = e.path().is_dir();
        if let Some(ig) = ignore.as_mut() {
            let full = if ig_base.is_empty() {
                r.clone()
            } else {
                format!("{ig_base}/{r}")
            };
            if ig.is_ignored(&full, is_dir) {
                continue;
            }
        }
        if is_dir {
            walk_dir(&e.path(), &r, ignore, ig_base, out)?;
        } else {
            out.push(r);
        }
    }
    Ok(())
}

pub fn cmd_grep(args: &[String]) -> CliResult<()> {
    let Parsed {
        o,
        mut toks,
        rest,
        paths,
        dashdash,
    } = parse(args)?;
    let mut rest = rest.into_iter();
    if !toks.iter().any(|t| matches!(t, Tok::Pat(_))) {
        toks.push(Tok::Pat(
            rest.next()
                .ok_or_else(|| CliError::fatal("no pattern given"))?,
        ));
    }
    let mut ep = ExprParser {
        toks: &toks,
        i: 0,
        pats: Vec::new(),
    };
    let expr = ep.or()?;
    if ep.i < toks.len() {
        return Err(CliError::fatal("incomplete pattern expression group"));
    }
    let patterns = ep.pats;
    let pats: Vec<Bre> = patterns
        .iter()
        .map(|p| match o.syntax {
            Syntax::Fixed => Bre::new(&escape_fixed(p), o.icase),
            Syntax::Basic => Bre::with_mode(p, o.icase, false),
            Syntax::Extended => Bre::with_mode(p, o.icase, true),
            Syntax::Perl => Bre::with_mode(&perl_to_ere(p), o.icase, true),
        })
        .collect();
    let mut positive = Vec::new();
    expr.positive(false, &mut positive);

    let cwd = crate::util::cwd()?;
    let mut sources: Vec<Source> = Vec::new();
    if o.no_index {
        if o.cached || o.untracked {
            return Err(CliError::fatal(
                "--cached or --untracked cannot be used with --no-index",
            ));
        }
        let mut all: Vec<String> = rest.chain(paths).collect();
        let specs: Vec<String> = std::mem::take(&mut all)
            .into_iter()
            .map(|p| p.trim_end_matches('/').to_string())
            .collect();
        let (mut ignore, base) = match (o.exclude_standard, open_repo().ok()) {
            (Some(true), Some(repo)) => {
                let work = work_tree(&repo)?;
                let canon =
                    |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                let base = canon(&cwd)
                    .strip_prefix(canon(&work))
                    .map(|r| r.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                (Some(crate::cmd::ignore::Ignore::new(&work)), base)
            }
            _ => (None, String::new()),
        };
        let mut files = Vec::new();
        walk_dir(&cwd, "", &mut ignore, &base, &mut files)?;
        for f in files {
            let in_spec = specs.is_empty()
                || specs
                    .iter()
                    .any(|s| s == "." || f == *s || f.starts_with(&format!("{s}/")));
            if !in_spec || !depth_ok(&f, &specs, o.max_depth) {
                continue;
            }
            if let Ok(bytes) = fs::read(cwd.join(&f)) {
                sources.push(Source { name: f, bytes });
            }
        }
    } else {
        let repo = open_repo()?;
        let work = work_tree(&repo)?;
        let mut revs: Vec<(String, arks_oid::Oid)> = Vec::new();
        // Arguments before `--` are revisions while they resolve.
        let mut in_paths = false;
        let mut extra_paths = Vec::new();
        for r in rest {
            if !in_paths {
                if let Ok(oid) = crate::cmd::shared::resolve_rev(&repo, &r) {
                    revs.push((r, oid));
                    continue;
                }
                if dashdash {
                    return Err(CliError::fatal(format!("unable to resolve revision: {r}")));
                }
                in_paths = true;
            }
            extra_paths.push(r);
        }
        extra_paths.extend(paths);
        if !revs.is_empty() && (o.cached || o.untracked) {
            return Err(CliError::fatal(
                "--cached or --untracked cannot be used with revisions",
            ));
        }
        let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
        let prefix = canon(&cwd)
            .strip_prefix(canon(&work))
            .map(|r| r.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let mut specs: Vec<String> = Vec::new();
        for p in &extra_paths {
            specs.push(crate::cmd::core::pathspec_rel(&work, p)?);
        }
        if specs.is_empty() && !prefix.is_empty() {
            specs.push(prefix.clone());
        }
        let in_spec = |p: &str| {
            (specs.is_empty()
                || specs
                    .iter()
                    .any(|s| s.is_empty() || p == s || p.starts_with(&format!("{s}/"))))
                && depth_ok(p, &specs, o.max_depth)
        };
        let show = |p: &str| -> String {
            if prefix.is_empty() || o.full_name {
                return p.to_string();
            }
            let pre: Vec<&str> = prefix.split('/').collect();
            let comps: Vec<&str> = p.split('/').collect();
            let mut common = 0;
            while common < pre.len() && common + 1 < comps.len() && pre[common] == comps[common] {
                common += 1;
            }
            format!(
                "{}{}",
                "../".repeat(pre.len() - common),
                comps[common..].join("/")
            )
        };
        if revs.is_empty() {
            let index = read_index(&repo)?;
            let mut last = String::new();
            let mut names: Vec<(String, Option<arks_oid::Oid>)> = Vec::new();
            for e in &index.entries {
                if e.path == last || !in_spec(&e.path) {
                    continue;
                }
                last = e.path.clone();
                names.push((e.path.clone(), Some(e.oid)));
            }
            if o.untracked {
                let indexed: HashSet<String> =
                    index.entries.iter().map(|e| e.path.clone()).collect();
                let mut un = Vec::new();
                if o.exclude_standard == Some(false) {
                    crate::cmd::shared::collect_untracked_with(
                        &work, &work, &indexed, &mut un, None,
                    )?;
                } else {
                    crate::cmd::shared::collect_untracked(&work, &work, &indexed, &mut un)?;
                }
                names.extend(un.into_iter().filter(|p| in_spec(p)).map(|p| (p, None)));
                names.sort_by(|a, b| a.0.cmp(&b.0));
            }
            for (path, oid) in names {
                let bytes = match (o.cached, oid) {
                    (true, Some(oid)) => repo.get_object(&oid)?.payload,
                    _ => match fs::read(work.join(&path)) {
                        Ok(b) => b,
                        Err(_) => continue,
                    },
                };
                sources.push(Source {
                    name: show(&path),
                    bytes,
                });
            }
        } else {
            for (name, oid) in &revs {
                let map = crate::cmd::shared::tree_blob_map(&repo, oid)?;
                let mut keys: Vec<&String> = map.keys().filter(|k| in_spec(k)).collect();
                keys.sort();
                for k in keys {
                    let bytes = repo.get_object(&map[k])?.payload;
                    sources.push(Source {
                        name: format!("{name}:{}", show(k)),
                        bytes,
                    });
                }
            }
        }
    }
    search(&o, &expr, &pats, &positive, &sources)
}

fn search(
    o: &Opts,
    expr: &Expr,
    pats: &[Bre],
    positive: &[usize],
    sources: &[Source],
) -> CliResult<()> {
    let stdout: &'static std::io::Stdout = Box::leak(Box::new(std::io::stdout()));
    let mut out = Out {
        o,
        w: std::io::BufWriter::new(stdout.lock()),
    };
    let mut any = false;
    let mut listed_without = false;
    let mut first_file = true;
    let mut pager_files: Vec<String> = Vec::new();
    // `-O` without a pager command only applies when the output is a
    // terminal (no default pager otherwise).
    let pager_on = match &o.pager {
        Some(Some(_)) => true,
        Some(None) => std::io::IsTerminal::is_terminal(&std::io::stdout()),
        None => false,
    };
    let ctx_on = o.before > 0 || o.after > 0 || o.function_context;
    let mut printed_any_group = false;
    let alternatives = expr.alternatives();
    for src in sources {
        let bytes = &src.bytes;
        let name = &src.name;
        let binary = !o.text && bytes[..bytes.len().min(8000)].contains(&0);
        if binary && o.skip_binary {
            continue;
        }
        let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
        let lines: Vec<&[u8]> = if bytes.is_empty() {
            Vec::new()
        } else {
            body.split(|c| *c == b'\n').collect()
        };
        // Per line: which patterns hit, and whether the line is selected.
        let mut hits_per: Vec<Vec<Vec<(usize, usize)>>> = Vec::with_capacity(lines.len());
        let mut selected: Vec<bool> = Vec::with_capacity(lines.len());
        for line in &lines {
            let hits: Vec<Vec<(usize, usize)>> =
                pats.iter().map(|p| pat_matches(p, line, o.word)).collect();
            let m = expr.eval(&|i| !hits[i].is_empty());
            selected.push(m != o.invert);
            hits_per.push(hits);
        }
        if o.all_match && !o.invert {
            let all = alternatives
                .iter()
                .all(|alt| hits_per.iter().any(|h| alt.eval(&|i| !h[i].is_empty())));
            if !all {
                if o.files_without {
                    listed_without = true;
                    out.paint(C_FILE, name.as_bytes())?;
                    out.w.write_all(if o.null { b"\0" } else { b"\n" })?;
                }
                continue;
            }
        }
        // Apply --max-count.
        let mut count = 0usize;
        let mut chosen: Vec<usize> = Vec::new();
        for (n, s) in selected.iter().enumerate() {
            if *s {
                if o.max_count.is_some_and(|m| count >= m) {
                    break;
                }
                count += 1;
                chosen.push(n);
            }
        }
        if count > 0 {
            any = true;
        }
        if o.quiet {
            if any {
                break;
            }
            continue;
        }
        if pager_on {
            if count > 0 {
                pager_files.push(name.clone());
            }
            continue;
        }
        if o.files_with || o.files_without {
            let list = if o.files_with { count > 0 } else { count == 0 };
            if list {
                listed_without |= o.files_without;
                out.paint(C_FILE, name.as_bytes())?;
                out.w.write_all(if o.null { b"\0" } else { b"\n" })?;
            }
            continue;
        }
        if o.count {
            if count > 0 {
                if !o.no_filename {
                    out.name(name, b':')?;
                }
                writeln!(out.w, "{count}")?;
            }
            continue;
        }
        if count == 0 {
            continue;
        }
        if binary {
            writeln!(out.w, "Binary file {name} matches")?;
            continue;
        }
        if o.brk && !first_file {
            writeln!(out.w)?;
        } else if ctx_on && printed_any_group {
            out.paint(C_SEP, b"--")?;
            writeln!(out.w)?;
        }
        first_file = false;
        if o.heading && !o.no_filename {
            out.paint(C_FILE, name.as_bytes())?;
            writeln!(out.w)?;
        }
        // Lines to print: matches plus context / function context.
        let n = lines.len();
        let mut show: Vec<Option<u8>> = vec![None; n];
        for &m in &chosen {
            show[m] = Some(b':');
        }
        // Walk matches in order as git prints them: the function line
        // (`-p`) before the pre-context, the pre-context (function lines in
        // it marked `=` with `-p`), the match, then the after-context.
        let mut printed_to: Option<usize> = None;
        for &m in &chosen {
            let floor = printed_to.map_or(0, |p| p + 1);
            let pre_start = m.saturating_sub(o.before).max(floor);
            if o.show_function && !o.function_context && m >= floor {
                let found = (floor..pre_start.min(m))
                    .rev()
                    .find(|&y| is_funcname(lines[y]));
                let in_pre = (pre_start..m).any(|y| is_funcname(lines[y]));
                if let (Some(f), false) = (found, in_pre) {
                    show[f] = Some(b'=');
                }
            }
            // The nearest function line in the pre-context is the `=` one.
            let pre_func = if o.show_function {
                (pre_start..m).rev().find(|&y| is_funcname(lines[y]))
            } else {
                None
            };
            for (x, slot) in show.iter_mut().enumerate().take(m).skip(pre_start) {
                if slot.is_none() {
                    *slot = Some(if Some(x) == pre_func { b'=' } else { b'-' });
                }
            }
            let hi = (m + o.after).min(n.saturating_sub(1));
            let mut reach = hi.max(m);
            for slot in show.iter_mut().take(reach + 1).skip(m + 1) {
                if slot.is_none() {
                    *slot = Some(b'-');
                }
            }
            if o.function_context {
                let start = (0..=m).rev().find(|&x| is_funcname(lines[x])).unwrap_or(0);
                let end = (m + 1..n).find(|&x| is_funcname(lines[x])).unwrap_or(n);
                for x in start..end {
                    if show[x].is_none() {
                        show[x] = Some(if x == start && is_funcname(lines[x]) {
                            b'='
                        } else {
                            b'-'
                        });
                    }
                }
                reach = reach.max(end.saturating_sub(1));
            }
            printed_to = Some(printed_to.map_or(reach, |p| p.max(reach)));
        }
        let mut prev: Option<usize> = None;
        for (x, s) in show.iter().enumerate() {
            let Some(sep) = *s else { continue };
            if ctx_on {
                let gap = prev.is_some_and(|p| x > p + 1);
                if gap {
                    out.paint(C_SEP, b"--")?;
                    writeln!(out.w)?;
                }
            }
            prev = Some(x);
            printed_any_group = true;
            let line = lines[x];
            let hits = union_hits(
                positive
                    .iter()
                    .flat_map(|i| hits_per[x][*i].iter().copied())
                    .collect(),
            );
            if o.only && sep == b':' {
                if o.invert {
                    continue;
                }
                for (s0, e0) in hits {
                    write_lead(&mut out, o, name, x, Some(s0), sep)?;
                    out.paint(C_MATCH, &line[s0..e0])?;
                    writeln!(out.w)?;
                }
                continue;
            }
            let col = (sep == b':').then(|| hits.first().map_or(0, |h| h.0));
            write_lead(&mut out, o, name, x, col, sep)?;
            if o.color && sep == b':' && !o.invert {
                let mut at = 0;
                for (s0, e0) in &hits {
                    out.w.write_all(&line[at..*s0])?;
                    out.paint(C_MATCH, &line[*s0..*e0])?;
                    at = *e0;
                }
                out.w.write_all(&line[at..])?;
            } else {
                out.w.write_all(line)?;
            }
            writeln!(out.w)?;
        }
    }
    out.w.flush()?;
    if let (Some(pager), true) = (&o.pager, pager_on) {
        if !pager_files.is_empty() {
            let cmd = pager
                .clone()
                .or_else(|| std::env::var("GIT_PAGER").ok())
                .or_else(|| std::env::var("PAGER").ok())
                .unwrap_or_else(|| "less".into());
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(format!("{cmd} \"$@\""))
                .arg(&cmd)
                .args(&pager_files)
                .status()?;
            if !status.success() {
                return Err(CliError::silent(status.code().unwrap_or(1) as u8));
            }
        }
    }
    // git-grep -L: success when at least one file was listed.
    let ok = if o.files_without { listed_without } else { any };
    if ok { Ok(()) } else { Err(CliError::silent(1)) }
}

/// `name:lineno:col:` prefix of one output line.
fn write_lead(
    out: &mut Out<'_>,
    o: &Opts,
    name: &str,
    x: usize,
    col: Option<usize>,
    sep: u8,
) -> CliResult<()> {
    if !o.no_filename && !o.heading {
        out.name(name, sep)?;
    }
    // With -z every field separator is a NUL.
    let field_sep = |out: &mut Out<'_>| -> CliResult<()> {
        if o.null {
            out.w.write_all(b"\0")?;
            Ok(())
        } else {
            out.sep(sep)
        }
    };
    if o.line_numbers {
        out.paint(C_LNO, (x + 1).to_string().as_bytes())?;
        field_sep(out)?;
    }
    if o.column {
        if let Some(c) = col {
            out.paint(C_LNO, (c + 1).to_string().as_bytes())?;
            field_sep(out)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expr_of(toks: Vec<Tok>) -> (Expr, Vec<String>) {
        let mut p = ExprParser {
            toks: &toks,
            i: 0,
            pats: Vec::new(),
        };
        let e = p.or().unwrap();
        (e, p.pats)
    }

    #[test]
    fn expression_precedence() {
        // a --or b --and --not c  ==  a OR (b AND NOT c)
        let (e, pats) = expr_of(vec![
            Tok::Pat("a".into()),
            Tok::Or,
            Tok::Pat("b".into()),
            Tok::And,
            Tok::Not,
            Tok::Pat("c".into()),
        ]);
        assert_eq!(pats, ["a", "b", "c"]);
        assert!(e.eval(&|i| i == 0));
        assert!(e.eval(&|i| i == 1));
        assert!(!e.eval(&|i| i == 1 || i == 2));
        assert_eq!(e.alternatives().len(), 2);
    }

    #[test]
    fn depth_limits() {
        assert!(depth_ok("a.txt", &[], Some(0)));
        assert!(!depth_ok("src/a.txt", &[], Some(0)));
        assert!(depth_ok("src/a.txt", &["src".into()], Some(0)));
        assert!(!depth_ok("src/x/a.txt", &["src".into()], Some(0)));
    }
}
