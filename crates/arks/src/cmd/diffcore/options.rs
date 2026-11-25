//! Diff option parsing shared by `diff`, `diff-tree`, `diff-index`,
//! `diff-files`, `log`, `show`, `whatchanged` and `range-diff`.
//!
//! Option names, value syntax and defaults follow the git documentation's
//! common diff options. Options that take a required value accept both
//! `--opt=<v>` and `--opt <v>` (short: `-X<v>` and `-X <v>`); options with
//! an optional value only take it attached.
//!
//! Primary: <https://git-scm.com/docs/diff-options>,
//! <https://git-scm.com/docs/git-diff>.

use super::xdiff::{Algo, LineOpts};
use crate::cmd::bre::Bre;
use crate::error::{CliError, CliResult};

/// `--stat[=<width>[,<name-width>[,<count>]]]` and friends.
#[derive(Clone, Debug, Default)]
pub struct StatOpts {
    pub width: Option<usize>,
    pub name_width: Option<usize>,
    pub graph_width: Option<usize>,
    pub count: Option<usize>,
}

/// `--dirstat` parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirstatMode {
    Changes,
    Lines,
    Files,
}

#[derive(Clone, Debug)]
pub struct DirstatOpts {
    pub mode: DirstatMode,
    pub cumulative: bool,
    /// Limit in tenths of a percent.
    pub limit: u32,
}

impl Default for DirstatOpts {
    fn default() -> Self {
        Self {
            mode: DirstatMode::Changes,
            cumulative: false,
            limit: 30,
        }
    }
}

/// Rename / copy detection level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Detect {
    #[default]
    Off,
    Renames,
    Copies,
}

/// Similarity scores are kept on git's 0..=60000 scale.
pub const MAX_SCORE: u32 = 60000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WordDiff {
    Plain,
    Color,
    Porcelain,
}

#[derive(Clone, Debug)]
pub enum Pickaxe {
    /// `-S<string>` (or a regex with `--pickaxe-regex`).
    Count(String),
    /// `-G<regex>`.
    Grep(String),
}

#[derive(Clone, Debug)]
pub struct DiffOptions {
    // Output formats.
    pub patch: bool,
    pub raw: bool,
    pub name_only: bool,
    pub name_status: bool,
    pub numstat: bool,
    pub stat: bool,
    pub shortstat: bool,
    pub dirstat: Option<DirstatOpts>,
    pub summary: bool,
    pub check: bool,
    /// `-s` / `--no-patch`.
    pub no_output: bool,
    pub stat_opts: StatOpts,
    pub compact_summary: bool,
    /// Explicit `-U<n>` (else 3).
    pub context: Option<usize>,
    pub inter_hunk: usize,
    pub function_context: bool,
    pub binary: bool,
    pub full_index: bool,
    /// `--abbrev[=<n>]` (`Some(0)` = `--no-abbrev`: full ids).
    pub abbrev: Option<usize>,
    pub abbrev_given: bool,
    pub nul: bool,
    pub src_prefix: String,
    pub dst_prefix: String,
    pub line_prefix: String,
    pub ind_new: String,
    pub ind_old: String,
    pub ind_ctx: String,
    pub color: Option<bool>,
    pub color_moved: bool,
    pub word_diff: Option<WordDiff>,
    pub word_regex: Option<String>,
    // Renames.
    pub detect: Detect,
    pub rename_score: u32,
    pub copy_score: u32,
    pub find_copies_harder: bool,
    pub break_rewrites: Option<(u32, u32)>,
    pub irreversible_delete: bool,
    pub rename_empty: bool,
    pub rename_limit: Option<usize>,
    pub no_renames: bool,
    pub follow: bool,
    // Algorithm.
    pub line: LineOpts,
    pub ignore_regex_src: Vec<String>,
    // Other.
    pub relative: Option<String>,
    pub text: bool,
    pub reverse: bool,
    pub exit_code: bool,
    pub quiet: bool,
    pub pickaxe: Option<Pickaxe>,
    pub pickaxe_all: bool,
    pub pickaxe_regex: bool,
    pub find_object: Option<String>,
    pub orderfile: Option<String>,
    pub rotate_to: Option<String>,
    pub skip_to: Option<String>,
    pub diff_filter: Option<String>,
    pub output: Option<String>,
    pub ita_invisible: bool,
    /// `--ws-error-highlight` covers context lines (`context` / `all`).
    pub ws_highlight_context: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            patch: false,
            raw: false,
            name_only: false,
            name_status: false,
            numstat: false,
            stat: false,
            shortstat: false,
            dirstat: None,
            summary: false,
            check: false,
            no_output: false,
            stat_opts: StatOpts::default(),
            compact_summary: false,
            context: None,
            inter_hunk: 0,
            function_context: false,
            binary: false,
            full_index: false,
            abbrev: None,
            abbrev_given: false,
            nul: false,
            src_prefix: "a/".into(),
            dst_prefix: "b/".into(),
            line_prefix: String::new(),
            ind_new: "+".into(),
            ind_old: "-".into(),
            ind_ctx: " ".into(),
            color: None,
            color_moved: false,
            word_diff: None,
            word_regex: None,
            detect: Detect::Off,
            rename_score: MAX_SCORE / 2,
            copy_score: MAX_SCORE / 2,
            find_copies_harder: false,
            break_rewrites: None,
            irreversible_delete: false,
            rename_empty: true,
            rename_limit: None,
            no_renames: false,
            follow: false,
            line: LineOpts::default(),
            ignore_regex_src: Vec::new(),
            relative: None,
            text: false,
            reverse: false,
            exit_code: false,
            quiet: false,
            pickaxe: None,
            pickaxe_all: false,
            pickaxe_regex: false,
            find_object: None,
            orderfile: None,
            rotate_to: None,
            skip_to: None,
            diff_filter: None,
            output: None,
            ita_invisible: false,
            ws_highlight_context: false,
        }
    }
}

/// git `parse_rename_score`: `<n>` is a fraction digit string (`5` =
/// 50%, `05` = 5%), `<n>%` a percentage, `<n>.<m>` a decimal fraction.
pub fn parse_score(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    if let Some(p) = s.strip_suffix('%') {
        let v: f64 = p.parse().ok()?;
        return Some(((v / 100.0) * MAX_SCORE as f64).min(MAX_SCORE as f64) as u32);
    }
    if s.contains('.') {
        let v: f64 = s.parse().ok()?;
        if v >= 1.0 {
            return Some(MAX_SCORE);
        }
        return Some((v * MAX_SCORE as f64) as u32);
    }
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Digits are the fraction after a decimal point.
    let v: f64 = format!("0.{s}").parse().ok()?;
    Some((v * MAX_SCORE as f64) as u32)
}

/// Argument cursor used by the option parsers.
pub struct Args<'a> {
    pub v: &'a [String],
    pub i: usize,
}

impl<'a> Args<'a> {
    pub fn new(v: &'a [String]) -> Self {
        Self { v, i: 0 }
    }

    pub fn cur(&self) -> &'a str {
        &self.v[self.i]
    }

    /// Value of a required-argument long option `name` (`--x=v` or `--x v`).
    pub fn long_req(&mut self, name: &str) -> CliResult<Option<String>> {
        let a = self.cur();
        if let Some(v) = a.strip_prefix(name).and_then(|r| r.strip_prefix('=')) {
            return Ok(Some(v.to_string()));
        }
        if a == name {
            let v = self.v.get(self.i + 1).cloned().ok_or_else(|| {
                CliError::fatal(format!(
                    "option '{}' requires a value",
                    name.trim_start_matches('-')
                ))
            })?;
            self.i += 1;
            return Ok(Some(v));
        }
        Ok(None)
    }

    /// Value of a required-argument short option `-X` (`-Xv` or `-X v`).
    pub fn short_req(&mut self, flag: char) -> CliResult<Option<String>> {
        let a = self.cur();
        let p = format!("-{flag}");
        if a == p {
            let v = self
                .v
                .get(self.i + 1)
                .cloned()
                .ok_or_else(|| CliError::fatal(format!("switch `{flag}' requires a value")))?;
            self.i += 1;
            return Ok(Some(v));
        }
        if let Some(v) = a.strip_prefix(&p) {
            if !a.starts_with("--") {
                return Ok(Some(v.to_string()));
            }
        }
        Ok(None)
    }

    /// Optional-argument long option: `Some(None)` bare, `Some(Some(v))` with `=v`.
    pub fn long_opt(&self, name: &str) -> Option<Option<String>> {
        let a = self.cur();
        if a == name {
            return Some(None);
        }
        a.strip_prefix(name)
            .and_then(|r| r.strip_prefix('='))
            .map(|v| Some(v.to_string()))
    }

    /// Optional-argument short option: `-X` or `-X<v>`.
    pub fn short_opt(&self, flag: char) -> Option<Option<String>> {
        let a = self.cur();
        let p = format!("-{flag}");
        if a == p {
            return Some(None);
        }
        if a.starts_with("--") {
            return None;
        }
        a.strip_prefix(&p).map(|v| Some(v.to_string()))
    }
}

fn num(v: &str, opt: &str) -> CliResult<usize> {
    v.parse()
        .map_err(|_| CliError::fatal(format!("{opt} expects a numerical value")))
}

fn parse_dirstat(d: &mut DirstatOpts, params: &str) -> CliResult<()> {
    for p in params.split(',').filter(|p| !p.is_empty()) {
        match p {
            "changes" => d.mode = DirstatMode::Changes,
            "lines" => d.mode = DirstatMode::Lines,
            "files" => d.mode = DirstatMode::Files,
            "cumulative" => d.cumulative = true,
            "noncumulative" => d.cumulative = false,
            _ => {
                let (whole, frac) = p.split_once('.').unwrap_or((p, "0"));
                let ok = !whole.is_empty()
                    && whole.bytes().all(|b| b.is_ascii_digit())
                    && frac.bytes().all(|b| b.is_ascii_digit());
                if !ok {
                    return Err(CliError::fatal(format!(
                        "Failed to parse --dirstat/-X option parameter:\nUnknown dirstat parameter '{p}'"
                    )));
                }
                let w: u32 = whole.parse().unwrap_or(0);
                let f: u32 = frac
                    .chars()
                    .next()
                    .and_then(|c| c.to_digit(10))
                    .unwrap_or(0);
                d.limit = w * 10 + f;
            }
        }
    }
    Ok(())
}

fn parse_color(v: Option<&str>) -> CliResult<bool> {
    match v {
        None | Some("always") | Some("true") | Some("yes") | Some("on") => Ok(true),
        Some("never") | Some("false") | Some("no") | Some("off") => Ok(false),
        // Output is never a terminal in this mode of operation.
        Some("auto") => Ok(false),
        Some(x) => Err(CliError::fatal(format!(
            "option `color' expects \"always\", \"auto\", or \"never\" (got '{x}')"
        ))),
    }
}

impl DiffOptions {
    pub fn context(&self) -> usize {
        self.context.unwrap_or(3)
    }

    /// Any output format requested (`-s` not applied).
    pub fn any_format(&self) -> bool {
        self.patch
            || self.raw
            || self.name_only
            || self.name_status
            || self.numstat
            || self.stat
            || self.shortstat
            || self.dirstat.is_some()
            || self.summary
            || self.check
    }

    fn set_patch(&mut self) {
        self.patch = true;
        self.no_output = false;
    }

    /// Try to consume the option at `a.cur()` (and its value).
    /// Returns `Ok(true)` when it is a diff option.
    pub fn take(&mut self, a: &mut Args<'_>) -> CliResult<bool> {
        let cur = a.cur();
        match cur {
            "-p" | "-u" | "--patch" => self.set_patch(),
            "-s" | "--no-patch" => self.no_output = true,
            // Only the patch options lift an earlier `-s` (pin 2.39.5).
            "--raw" => self.raw = true,
            "--patch-with-raw" => {
                self.set_patch();
                self.raw = true;
            }
            "--patch-with-stat" => {
                self.set_patch();
                self.stat = true;
            }
            "--numstat" => self.numstat = true,
            "--shortstat" => self.shortstat = true,
            "--name-only" => self.name_only = true,
            "--name-status" => self.name_status = true,
            "--summary" => self.summary = true,
            "--check" => self.check = true,
            "--cumulative" => {
                self.dirstat.get_or_insert_with(Default::default).cumulative = true;
            }
            "--compact-summary" => {
                self.compact_summary = true;
                self.stat = true;
            }
            "--no-compact-summary" => self.compact_summary = false,
            "-W" | "--function-context" => self.function_context = true,
            "--no-function-context" => self.function_context = false,
            "--binary" => {
                self.binary = true;
                self.set_patch();
            }
            "--full-index" => self.full_index = true,
            "--no-full-index" => self.full_index = false,
            "-z" => self.nul = true,
            "--no-abbrev" => {
                self.abbrev = Some(0);
                self.abbrev_given = true;
            }
            "--no-prefix" => {
                self.src_prefix.clear();
                self.dst_prefix.clear();
            }
            "--default-prefix" => {
                self.src_prefix = "a/".into();
                self.dst_prefix = "b/".into();
            }
            "--no-color" => self.color = Some(false),
            "--no-color-moved" => self.color_moved = false,
            "--no-color-moved-ws" => {}
            "-D" | "--irreversible-delete" => self.irreversible_delete = true,
            "--find-copies-harder" => {
                self.find_copies_harder = true;
                self.detect = Detect::Copies;
            }
            "--no-find-copies-harder" => self.find_copies_harder = false,
            "--no-renames" => {
                self.no_renames = true;
                self.detect = Detect::Off;
            }
            "--rename-empty" => self.rename_empty = true,
            "--no-rename-empty" => self.rename_empty = false,
            "--follow" => self.follow = true,
            "--no-follow" => self.follow = false,
            "--minimal" => self.line.algo = Algo::Minimal,
            "--patience" => self.line.algo = Algo::Patience,
            "--histogram" => self.line.algo = Algo::Histogram,
            "-w" | "--ignore-all-space" => self.line.ignore_all_space = true,
            "-b" | "--ignore-space-change" => self.line.ignore_space_change = true,
            "--ignore-space-at-eol" => self.line.ignore_space_at_eol = true,
            "--ignore-cr-at-eol" => self.line.ignore_cr_at_eol = true,
            "--ignore-blank-lines" => self.line.ignore_blank_lines = true,
            "--no-ignore-matching-lines" => {
                self.line.ignore_regex.clear();
                self.ignore_regex_src.clear();
            }
            // The indent heuristic only moves ambiguous change groups; see
            // xdiff's module note.
            "--indent-heuristic" | "--no-indent-heuristic" => {}
            "--no-relative" => self.relative = None,
            "-a" | "--text" => self.text = true,
            "--no-text" => self.text = false,
            "-R" => self.reverse = true,
            "--exit-code" => self.exit_code = true,
            "--no-exit-code" => self.exit_code = false,
            "--quiet" => {
                self.quiet = true;
                self.exit_code = true;
            }
            "--no-quiet" => self.quiet = false,
            // No external diff driver or textconv filter is ever run by
            // arks; without configured drivers git's output is the same.
            "--ext-diff" | "--no-ext-diff" | "--textconv" | "--no-textconv" => {}
            "--ita-invisible-in-index" => self.ita_invisible = true,
            "--ita-visible-in-index" => self.ita_invisible = false,
            "--pickaxe-all" => self.pickaxe_all = true,
            "--pickaxe-regex" => self.pickaxe_regex = true,
            _ => return self.take_valued(a),
        }
        Ok(true)
    }

    fn take_valued(&mut self, a: &mut Args<'_>) -> CliResult<bool> {
        let cur = a.cur();
        if let Some(v) = a.long_opt("--unified").or_else(|| a.short_opt('U')) {
            if let Some(v) = v {
                self.context = Some(num(&v, "-U")?);
            }
            self.set_patch();
        } else if let Some(v) = a.long_opt("--stat") {
            self.stat = true;
            if let Some(v) = v {
                let mut it = v.split(',');
                if let Some(w) = it.next().filter(|s| !s.is_empty()) {
                    self.stat_opts.width = Some(num(w, "--stat")?);
                }
                if let Some(n) = it.next().filter(|s| !s.is_empty()) {
                    self.stat_opts.name_width = Some(num(n, "--stat")?);
                }
                if let Some(c) = it.next().filter(|s| !s.is_empty()) {
                    self.stat_opts.count = Some(num(c, "--stat")?);
                }
            }
        } else if let Some(v) = a.long_req("--stat-width")? {
            self.stat = true;
            self.stat_opts.width = Some(num(&v, "--stat-width")?);
        } else if let Some(v) = a.long_req("--stat-name-width")? {
            self.stat = true;
            self.stat_opts.name_width = Some(num(&v, "--stat-name-width")?);
        } else if let Some(v) = a.long_req("--stat-graph-width")? {
            self.stat = true;
            self.stat_opts.graph_width = Some(num(&v, "--stat-graph-width")?);
        } else if let Some(v) = a.long_req("--stat-count")? {
            self.stat = true;
            self.stat_opts.count = Some(num(&v, "--stat-count")?);
        } else if let Some(v) = a.long_opt("--dirstat").or_else(|| a.short_opt('X')) {
            let d = self.dirstat.get_or_insert_with(Default::default);
            if let Some(v) = v {
                parse_dirstat(d, &v)?;
            }
        } else if let Some(v) = a.long_opt("--dirstat-by-file") {
            let d = self.dirstat.get_or_insert_with(Default::default);
            d.mode = DirstatMode::Files;
            if let Some(v) = v {
                parse_dirstat(d, &v)?;
            }
        } else if let Some(v) = a.long_opt("--abbrev") {
            self.abbrev_given = true;
            self.abbrev = Some(match v {
                Some(n) => {
                    let n = num(&n, "--abbrev")?;
                    if n == 0 { 0 } else { n.clamp(4, 40) }
                }
                None => 7,
            });
        } else if let Some(v) = a.long_req("--src-prefix")? {
            self.src_prefix = v;
        } else if let Some(v) = a.long_req("--dst-prefix")? {
            self.dst_prefix = v;
        } else if let Some(v) = a.long_req("--line-prefix")? {
            self.line_prefix = v;
        } else if let Some(v) = a.long_req("--inter-hunk-context")? {
            self.inter_hunk = num(&v, "--inter-hunk-context")?;
        } else if let Some(v) = a.long_req("--output-indicator-new")? {
            self.ind_new = v;
        } else if let Some(v) = a.long_req("--output-indicator-old")? {
            self.ind_old = v;
        } else if let Some(v) = a.long_req("--output-indicator-context")? {
            self.ind_ctx = v;
        } else if let Some(v) = a.long_opt("--color") {
            self.color = Some(parse_color(v.as_deref())?);
        } else if let Some(v) = a.long_req("--ws-error-highlight")? {
            for k in v.split(',') {
                if !matches!(k, "context" | "old" | "new" | "all" | "none" | "default") {
                    return Err(CliError::fatal(format!(
                        "unknown value after ws-error-highlight={k}"
                    )));
                }
            }
            self.ws_highlight_context = v.split(',').fold(false, |on, k| match k {
                "context" | "all" => true,
                "none" | "default" => false,
                _ => on,
            });
        } else if let Some(v) = a.long_opt("--color-moved") {
            if let Some(m) = v.as_deref() {
                if !matches!(
                    m,
                    "no" | "default"
                        | "plain"
                        | "blocks"
                        | "zebra"
                        | "dimmed-zebra"
                        | "dimmed_zebra"
                ) && parse_color(Some(m)).is_err()
                {
                    return Err(CliError::fatal(format!("bad --color-moved argument: {m}")));
                }
                self.color_moved = m != "no" && parse_color(Some(m)).unwrap_or(true);
            } else {
                self.color_moved = true;
            }
        } else if a.long_req("--color-moved-ws")?.is_some() {
            // Only changes which moved lines are coloured as moved.
        } else if let Some(v) = a.long_opt("--break-rewrites").or_else(|| a.short_opt('B')) {
            let (mut b, mut m) = (MAX_SCORE / 2, MAX_SCORE * 3 / 5);
            if let Some(v) = v {
                let (x, y) = v.split_once('/').unwrap_or((&v, ""));
                if !x.is_empty() {
                    b = parse_score(x).ok_or_else(|| bad_score(cur))?;
                }
                if !y.is_empty() {
                    m = parse_score(y).ok_or_else(|| bad_score(cur))?;
                }
            }
            self.break_rewrites = Some((b, m));
        } else if let Some(v) = a.long_opt("--find-renames").or_else(|| a.short_opt('M')) {
            if let Some(v) = v {
                self.rename_score = parse_score(&v).ok_or_else(|| bad_score(cur))?;
            }
            if self.detect == Detect::Off {
                self.detect = Detect::Renames;
            }
            self.no_renames = false;
        } else if let Some(v) = a.long_opt("--find-copies").or_else(|| a.short_opt('C')) {
            if let Some(v) = v {
                self.copy_score = parse_score(&v).ok_or_else(|| bad_score(cur))?;
            }
            if self.detect == Detect::Copies {
                self.find_copies_harder = true;
            }
            self.detect = Detect::Copies;
            self.no_renames = false;
        } else if let Some(v) = a.short_req('l')? {
            self.rename_limit = Some(num(&v, "-l")?);
        } else if let Some(v) = a.long_req("--diff-algorithm")? {
            self.line.algo = Algo::parse(&v).ok_or_else(|| {
                CliError::fatal(
                    "option diff-algorithm accepts \"myers\", \"minimal\", \"patience\" and \"histogram\"",
                )
            })?;
        } else if let Some(v) = a.long_req("--anchored")? {
            self.line.anchors.push(v);
            self.line.algo = Algo::Patience;
        } else if let Some(v) = a.long_req("--ignore-matching-lines")?.or(a.short_req('I')?) {
            self.line.ignore_regex.push(Bre::with_mode(&v, false, true));
            self.ignore_regex_src.push(v);
        } else if let Some(v) = a.long_opt("--word-diff") {
            self.word_diff = Some(match v.as_deref() {
                None | Some("plain") => WordDiff::Plain,
                Some("color") => WordDiff::Color,
                Some("porcelain") => WordDiff::Porcelain,
                Some("none") => {
                    self.word_diff = None;
                    self.set_patch();
                    return Ok(true);
                }
                Some(x) => {
                    return Err(CliError::fatal(format!("bad --word-diff argument: {x}")));
                }
            });
            self.set_patch();
        } else if let Some(v) = a.long_req("--word-diff-regex")? {
            self.word_regex = Some(v);
            if self.word_diff.is_none() {
                self.word_diff = Some(WordDiff::Plain);
            }
            self.set_patch();
        } else if let Some(v) = a.long_opt("--color-words") {
            self.word_diff = Some(WordDiff::Color);
            self.color = Some(true);
            if let Some(v) = v {
                self.word_regex = Some(v);
            }
            self.set_patch();
        } else if let Some(v) = a.long_opt("--relative") {
            self.relative = Some(v.unwrap_or_default());
        } else if let Some(v) = a.long_opt("--ignore-submodules") {
            if let Some(v) = v {
                if !matches!(v.as_str(), "none" | "untracked" | "dirty" | "all") {
                    return Err(CliError::fatal(format!(
                        "bad --ignore-submodules argument: {v}"
                    )));
                }
            }
        } else if let Some(v) = a.long_opt("--submodule") {
            if let Some(v) = v {
                if !matches!(v.as_str(), "short" | "log" | "diff") {
                    return Err(CliError::fatal(format!(
                        "Failed to parse --submodule option parameter: '{v}'"
                    )));
                }
            }
        } else if let Some(v) = a.short_req('S')? {
            self.pickaxe = Some(Pickaxe::Count(v));
        } else if let Some(v) = a.short_req('G')? {
            self.pickaxe = Some(Pickaxe::Grep(v));
        } else if let Some(v) = a.short_req('O')? {
            self.orderfile = Some(v);
        } else if let Some(v) = a.long_req("--rotate-to")? {
            self.rotate_to = Some(v);
        } else if let Some(v) = a.long_req("--skip-to")? {
            self.skip_to = Some(v);
        } else if let Some(v) = a.long_req("--find-object")? {
            self.find_object = Some(v);
        } else if let Some(v) = a.long_req("--diff-filter")? {
            for c in v.chars() {
                if !"ACDMRTUXBacdmrtuxb*".contains(c) {
                    return Err(CliError::fatal(format!(
                        "unknown change class '{c}' in --diff-filter={v}"
                    )));
                }
            }
            self.diff_filter = Some(v);
        } else if let Some(v) = a.long_req("--output")? {
            self.output = Some(v);
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    /// Parse every diff option in `args`, returning the other arguments in
    /// order. Arguments after `--` are passed through untouched.
    pub fn parse_all(&mut self, args: &[String]) -> CliResult<Vec<String>> {
        let mut rest = Vec::new();
        let mut a = Args::new(args);
        while a.i < args.len() {
            if a.cur() == "--" {
                rest.extend(args[a.i..].iter().cloned());
                break;
            }
            if !self.take(&mut a)? {
                rest.push(a.cur().to_string());
            }
            a.i += 1;
        }
        Ok(rest)
    }

    /// Settings implied by others once parsing is done; rejects output formats
    /// that cannot be combined (`--name-only`, `--name-status`, `--check`, `-s`).
    pub fn finish(&mut self) -> CliResult<()> {
        let exclusive = [self.name_only, self.name_status, self.check, self.no_output];
        if exclusive.iter().filter(|set| **set).count() > 1 {
            return Err(CliError::fatal(
                "options '--name-only', '--name-status', '--check', and '-s' cannot be used together",
            ));
        }
        if self.reverse {
            std::mem::swap(&mut self.src_prefix, &mut self.dst_prefix);
        }
        if self.word_diff == Some(WordDiff::Color) && self.color.is_none() {
            self.color = Some(true);
        }
        if self.find_copies_harder && self.detect == Detect::Off {
            self.detect = Detect::Copies;
        }
        Ok(())
    }

    pub fn use_color(&self) -> bool {
        self.color.unwrap_or(false)
    }
}

fn bad_score(opt: &str) -> CliError {
    CliError::fatal(format!("invalid argument to {opt}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores() {
        assert_eq!(parse_score("5"), Some(30000));
        assert_eq!(parse_score("50%"), Some(30000));
        assert_eq!(parse_score("05"), Some(3000));
        assert_eq!(parse_score("0.9"), Some(54000));
    }

    #[test]
    fn values() {
        let args: Vec<String> = ["--stat=60,10,2", "-U1", "-S", "x", "--src-prefix=L/", "rev"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut o = DiffOptions::default();
        let rest = o.parse_all(&args).unwrap();
        assert_eq!(rest, vec!["rev".to_string()]);
        assert_eq!(o.stat_opts.width, Some(60));
        assert_eq!(o.context, Some(1));
        assert!(matches!(o.pickaxe, Some(Pickaxe::Count(ref s)) if s == "x"));
        assert_eq!(o.src_prefix, "L/");
    }
}
