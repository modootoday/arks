//! `grep`: search tracked files in the working tree, the index (`--cached`)
//! or trees (`<rev>...`).
//!
//! Output per git-grep: `<path>:<line>` (`-n` adds the line number, `<rev>:`
//! prefixes tree hits), `-c` counts, `-l`/`-L` names, `-o` matched parts,
//! `Binary file <path> matches` for binary blobs; paths are shown relative
//! to (and limited to) the current directory. Exit status 1 when nothing
//! matched. Patterns are basic regexes (`-G`, default), extended (`-E`) or
//! fixed strings (`-F`).
//! Primary: <https://git-scm.com/docs/git-grep>.
//!
//! Partial: `-P`, `--and`/`--or`/`--not`, context lines (`-A`/`-B`/`-C`),
//! `--untracked` and `--no-index` are not claimed.

use crate::cmd::bre::Bre;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use std::fs;
use std::io::Write;

#[derive(Default)]
struct Opts {
    line_numbers: bool,
    icase: bool,
    count: bool,
    files_with: bool,
    files_without: bool,
    word: bool,
    invert: bool,
    only: bool,
    no_filename: bool,
    quiet: bool,
    extended: bool,
    fixed: bool,
    cached: bool,
    text: bool,
    skip_binary: bool,
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

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Matches of the pattern set in `line` (`-w` enforces word boundaries).
fn line_matches(pats: &[Bre], line: &[u8], word: bool) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut pos = 0;
    while pos <= line.len() {
        let mut best: Option<(usize, usize)> = None;
        for p in pats {
            let mut from = pos;
            while let Some((s, e)) = p.find_from(line, from) {
                let ok = !word
                    || ((s == 0 || !is_word(line[s - 1]))
                        && (e >= line.len() || !is_word(line[e]))
                        && e > s);
                if ok {
                    if best.is_none_or(|(bs, be)| s < bs || (s == bs && e > be)) {
                        best = Some((s, e));
                    }
                    break;
                }
                from = s + 1;
                if from > line.len() {
                    break;
                }
            }
        }
        let Some((s, e)) = best else { break };
        out.push((s, e));
        pos = if e > s { e } else { e + 1 };
    }
    out
}

pub fn cmd_grep(args: &[String]) -> CliResult<()> {
    let args = crate::util::split_short_bundles(args, "nicLlwvohHqEFGIa", "e");
    let mut o = Opts::default();
    let mut patterns: Vec<String> = Vec::new();
    let mut rest: Vec<String> = Vec::new();
    let mut paths: Vec<String> = Vec::new();
    let mut dashdash = false;
    let mut i = 0;
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
            "-i" | "--ignore-case" => o.icase = true,
            "-c" | "--count" => o.count = true,
            "-l" | "--files-with-matches" | "--name-only" => o.files_with = true,
            "-L" | "--files-without-match" => o.files_without = true,
            "-w" | "--word-regexp" => o.word = true,
            "-v" | "--invert-match" => o.invert = true,
            "-o" | "--only-matching" => o.only = true,
            "-h" => o.no_filename = true,
            "-H" => o.no_filename = false,
            "-q" | "--quiet" => o.quiet = true,
            "-E" | "--extended-regexp" => o.extended = true,
            "-F" | "--fixed-strings" => o.fixed = true,
            "-G" | "--basic-regexp" => {
                o.extended = false;
                o.fixed = false;
            }
            "-I" => o.skip_binary = true,
            "-a" | "--text" => o.text = true,
            "--cached" => o.cached = true,
            "--no-color" | "--color=never" => {}
            "-e" => patterns.push(
                args.get(i)
                    .cloned()
                    .ok_or_else(|| CliError::fatal("switch `e' requires a value"))?,
            ),
            "-P" | "--perl-regexp" => {
                return Err(CliError::NotImplemented("grep -P is not claimed".into()));
            }
            x if x.starts_with('-') && x.len() > 1 => {
                return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
            }
            _ => rest.push(a),
        }
        if args[i - 1] == "-e" {
            i += 1;
        }
    }
    let mut rest = rest.into_iter();
    if patterns.is_empty() {
        patterns.push(
            rest.next()
                .ok_or_else(|| CliError::fatal("no pattern given"))?,
        );
    }
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let mut revs: Vec<(String, arks_oid::Oid)> = Vec::new();
    // Arguments before `--` are revisions while they resolve; the rest are paths.
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
    let paths = extra_paths;
    let pats: Vec<Bre> = patterns
        .iter()
        .map(|p| {
            if o.fixed {
                Bre::new(&escape_fixed(p), o.icase)
            } else {
                Bre::with_mode(p, o.icase, o.extended)
            }
        })
        .collect();
    let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let prefix = canon(&crate::util::cwd()?)
        .strip_prefix(canon(&work))
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let mut specs: Vec<String> = Vec::new();
    for p in &paths {
        specs.push(crate::cmd::core::pathspec_rel(&work, p)?);
    }
    if specs.is_empty() && !prefix.is_empty() {
        specs.push(prefix.clone());
    }
    let in_spec = |p: &str| {
        specs.is_empty()
            || specs
                .iter()
                .any(|s| s.is_empty() || p == s || p.starts_with(&format!("{s}/")))
    };
    let show = |p: &str| -> String {
        if prefix.is_empty() {
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
    // (display name, contents) in search order.
    let mut sources: Vec<(String, Vec<u8>)> = Vec::new();
    if revs.is_empty() {
        let index = read_index(&repo)?;
        let mut last = String::new();
        for e in &index.entries {
            if e.path == last || !in_spec(&e.path) {
                continue;
            }
            last = e.path.clone();
            let bytes = if o.cached {
                repo.get_object(&e.oid)?.payload
            } else {
                match fs::read(work.join(&e.path)) {
                    Ok(b) => b,
                    Err(_) => continue,
                }
            };
            sources.push((show(&e.path), bytes));
        }
    } else {
        for (name, oid) in &revs {
            let map = crate::cmd::shared::tree_blob_map(&repo, oid)?;
            let mut keys: Vec<&String> = map.keys().filter(|k| in_spec(k)).collect();
            keys.sort();
            for k in keys {
                let bytes = repo.get_object(&map[k])?.payload;
                sources.push((format!("{name}:{}", show(k)), bytes));
            }
        }
    }
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    let mut any = false;
    let mut listed_without = false;
    for (name, bytes) in &sources {
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
        let mut count = 0usize;
        let mut printed_binary = false;
        for (n, line) in lines.iter().enumerate() {
            let hits = line_matches(&pats, line, o.word);
            let selected = hits.is_empty() == o.invert;
            if !selected {
                continue;
            }
            count += 1;
            if o.quiet {
                break;
            }
            if o.count || o.files_with || o.files_without {
                continue;
            }
            if binary {
                if !printed_binary {
                    writeln!(out, "Binary file {name} matches")?;
                    printed_binary = true;
                }
                continue;
            }
            let mut lead = String::new();
            if !o.no_filename {
                lead.push_str(name);
                lead.push(':');
            }
            if o.line_numbers {
                lead.push_str(&format!("{}:", n + 1));
            }
            if o.only && !o.invert {
                for (s, e) in hits {
                    out.write_all(lead.as_bytes())?;
                    out.write_all(&line[s..e])?;
                    out.write_all(b"\n")?;
                }
            } else {
                out.write_all(lead.as_bytes())?;
                out.write_all(line)?;
                out.write_all(b"\n")?;
            }
        }
        if count > 0 {
            any = true;
        }
        if o.quiet && any {
            break;
        }
        if o.count && count > 0 {
            if o.no_filename {
                writeln!(out, "{count}")?;
            } else {
                writeln!(out, "{name}:{count}")?;
            }
        } else if o.files_with && count > 0 {
            writeln!(out, "{name}")?;
        } else if o.files_without && count == 0 {
            listed_without = true;
            writeln!(out, "{name}")?;
        }
    }
    out.flush()?;
    // git-grep -L: success when at least one file was listed.
    let ok = if o.files_without { listed_without } else { any };
    if ok { Ok(()) } else { Err(CliError::silent(1)) }
}
