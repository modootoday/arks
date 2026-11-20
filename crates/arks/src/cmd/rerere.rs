//! `rerere` — reuse recorded resolutions of conflicted merges.
//!
//! A conflict is normalised (marker labels dropped, the two sides of each
//! hunk in byte order) and named by the hash of its sides
//! (`<side1> NUL <side2> NUL` per hunk). `$GIT_DIR/rr-cache/<id>/preimage`
//! holds the normalised conflict, `postimage` the recorded resolution and
//! `$GIT_DIR/MERGE_RR` the `<id> TAB <path> NUL` list of the merge in progress.
//! Subcommands: (none), `clear`, `forget <pathspec>`, `diff`, `status`,
//! `remaining`, `gc`; `--[no-]rerere-autoupdate` stages replayed resolutions.
//! Cite: <https://git-scm.com/docs/git-rerere> (behaviour, file layout and
//! messages observed from stock git).

use crate::cmd::opt_parse::{Mode, Takes::*, o, parse};
use crate::cmd::unified_diff::{HunkStyle, write_hunks};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree};
use arks_hash::Hasher;
use arks_repo::Repository;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const USAGE: &str =
    "usage: git rerere [clear | forget <pathspec>... | diff | status | remaining | gc]

    --rerere-autoupdate   register clean resolutions in index
";

fn config_bool(repo: &Repository, key: &str) -> Option<bool> {
    let v = arks_config::ConfigSet::load(Some(repo.common_dir())).get(key)?;
    match v.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" | "" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// rerere runs when `rerere.enabled` is true, or unset with an `rr-cache`.
pub(crate) fn enabled(repo: &Repository) -> bool {
    config_bool(repo, "rerere.enabled").unwrap_or_else(|| rr_cache(repo).is_dir())
}

fn rr_cache(repo: &Repository) -> PathBuf {
    repo.common_dir().join("rr-cache")
}

fn merge_rr_path(repo: &Repository) -> PathBuf {
    repo.git_dir().join("MERGE_RR")
}

fn read_merge_rr(repo: &Repository) -> Vec<(String, String)> {
    let Ok(bytes) = fs::read(merge_rr_path(repo)) else {
        return Vec::new();
    };
    bytes
        .split(|b| *b == 0)
        .filter(|r| !r.is_empty())
        .filter_map(|r| {
            let s = String::from_utf8_lossy(r);
            let (id, path) = s.split_once('\t')?;
            Some((id.to_string(), path.to_string()))
        })
        .collect()
}

fn write_merge_rr(repo: &Repository, rr: &[(String, String)]) -> CliResult<()> {
    let mut out = Vec::new();
    for (id, path) in rr {
        out.extend_from_slice(format!("{id}\t{path}").as_bytes());
        out.push(0);
    }
    fs::write(merge_rr_path(repo), out)?;
    Ok(())
}

/// Normalise the conflict hunks of `text`; returns the normalised bytes, the
/// conflict id and the number of hunks (`None` id when there is none).
fn normalize(repo: &Repository, text: &[u8]) -> (Vec<u8>, Option<String>, usize) {
    let lines: Vec<&[u8]> = text.split_inclusive(|c| *c == b'\n').collect();
    let is = |l: &[u8], m: &[u8]| {
        l.starts_with(m) && matches!(l.get(m.len()), None | Some(b' ' | b'\n' | b'\r'))
    };
    let mut out = Vec::new();
    let mut hasher = Hasher::new(repo.object_format());
    let mut hunks = 0;
    let mut i = 0;
    while i < lines.len() {
        if !is(lines[i], b"<<<<<<<") {
            out.extend_from_slice(lines[i]);
            i += 1;
            continue;
        }
        // <<<<<<< ours [||||||| base] ======= theirs >>>>>>>
        let start = i;
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let mut part = 0;
        let mut j = i + 1;
        let mut closed = false;
        while j < lines.len() {
            let l = lines[j];
            if is(l, b"|||||||") && part == 0 {
                part = 1;
            } else if is(l, b"=======") && part < 2 {
                part = 2;
            } else if is(l, b">>>>>>>") && part == 2 {
                closed = true;
                break;
            } else if part == 0 {
                a.extend_from_slice(l);
            } else if part == 2 {
                b.extend_from_slice(l);
            }
            j += 1;
        }
        if !closed {
            for l in &lines[start..] {
                out.extend_from_slice(l);
            }
            break;
        }
        if b < a {
            std::mem::swap(&mut a, &mut b);
        }
        out.extend_from_slice(b"<<<<<<<\n");
        out.extend_from_slice(&a);
        out.extend_from_slice(b"=======\n");
        out.extend_from_slice(&b);
        out.extend_from_slice(b">>>>>>>\n");
        hasher.update(&a);
        hasher.update(b"\0");
        hasher.update(&b);
        hasher.update(b"\0");
        hunks += 1;
        i = j + 1;
    }
    let id = (hunks > 0)
        .then(|| hasher.finalize().ok())
        .flatten()
        .map(|d| d.as_bytes().iter().map(|b| format!("{b:02x}")).collect());
    (out, id, hunks)
}

/// Unmerged index paths, in index order.
fn conflicted_paths(repo: &Repository) -> CliResult<Vec<String>> {
    let index = read_index(repo)?;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for e in &index.entries {
        if e.stage() != 0 && seen.insert(e.path.clone()) {
            out.push(e.path.clone());
        }
    }
    Ok(out)
}

fn stage_path(path: &str) -> CliResult<()> {
    crate::cmd::core::cmd_add(&["--".to_string(), path.to_string()])
}

/// The plain `git rerere` step: record preimages of new conflicts, reuse
/// recorded resolutions, and record resolutions of fixed conflicts.
pub(crate) fn do_rerere(repo: &Repository, autoupdate: bool) -> CliResult<()> {
    let work = work_tree(repo)?;
    let cache = rr_cache(repo);
    fs::create_dir_all(&cache)?;
    let mut rr = read_merge_rr(repo);
    for path in conflicted_paths(repo)? {
        if rr.iter().any(|(_, p)| p == &path) {
            continue;
        }
        let Ok(text) = fs::read(work.join(&path)) else {
            continue;
        };
        let (norm, Some(id), _) = normalize(repo, &text) else {
            continue;
        };
        let dir = cache.join(&id);
        let post = dir.join("postimage");
        if post.is_file() {
            let pre = fs::read(dir.join("preimage")).unwrap_or_default();
            let postimage = fs::read(&post)?;
            let resolved = if pre == norm {
                Some(postimage)
            } else {
                let labels = crate::cmd::merge_engine::MergeLabels::default();
                let (m, conflict) =
                    crate::cmd::merge_engine::merge_text(&pre, &norm, &postimage, &labels);
                (!conflict).then_some(m)
            };
            if let Some(bytes) = resolved {
                fs::write(work.join(&path), bytes)?;
                if autoupdate {
                    stage_path(&path)?;
                    eprintln!("Staged '{path}' using previous resolution.");
                } else {
                    eprintln!("Resolved '{path}' using previous resolution.");
                }
                continue;
            }
        }
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("preimage"), &norm)?;
        eprintln!("Recorded preimage for '{path}'");
        rr.push((id, path));
    }
    // Conflicts tracked in MERGE_RR whose markers are gone are resolved.
    let mut keep = Vec::new();
    for (id, path) in rr {
        let text = fs::read(work.join(&path)).unwrap_or_default();
        let (_, now, _) = normalize(repo, &text);
        let dir = cache.join(&id);
        if now.is_none() && dir.join("preimage").is_file() {
            fs::write(dir.join("postimage"), &text)?;
            eprintln!("Recorded resolution for '{path}'.");
            continue;
        }
        keep.push((id, path));
    }
    if keep.is_empty() {
        let _ = fs::remove_file(merge_rr_path(repo));
    } else {
        write_merge_rr(repo, &keep)?;
    }
    Ok(())
}

/// Hook for merging commands: run rerere when it is enabled.
pub(crate) fn auto_rerere(repo: &Repository, autoupdate: Option<bool>) -> CliResult<()> {
    if !enabled(repo) {
        return Ok(());
    }
    let au = autoupdate.unwrap_or_else(|| config_bool(repo, "rerere.autoUpdate").unwrap_or(false));
    do_rerere(repo, au)
}

fn in_pathspec(specs: &[String], path: &str) -> bool {
    specs.is_empty()
        || specs.iter().any(|s| {
            let s = s.trim_end_matches('/');
            s == "." || path == s || path.starts_with(&format!("{s}/"))
        })
}

/// `rerere forget`: regenerate the conflict from the index stages and drop
/// its recorded resolution.
fn forget(repo: &Repository, specs: &[String]) -> CliResult<()> {
    if specs.is_empty() {
        eprintln!("warning: 'git rerere forget' without paths is deprecated");
    }
    let index = read_index(repo)?;
    let cache = rr_cache(repo);
    let mut rr = read_merge_rr(repo);
    for path in conflicted_paths(repo)? {
        if !in_pathspec(specs, &path) {
            continue;
        }
        let side = |stage: u16| -> Vec<u8> {
            index
                .entries
                .iter()
                .find(|e| e.path == path && e.stage() == stage)
                .and_then(|e| repo.get_object(&e.oid).ok())
                .map(|o| o.payload)
                .unwrap_or_default()
        };
        let labels = crate::cmd::merge_engine::MergeLabels::default();
        let (merged, _) =
            crate::cmd::merge_engine::merge_text(&side(1), &side(2), &side(3), &labels);
        let (norm, Some(id), _) = normalize(repo, &merged) else {
            continue;
        };
        let dir = cache.join(&id);
        let post = dir.join("postimage");
        if !post.is_file() {
            eprintln!("error: no remembered resolution for '{path}'");
            continue;
        }
        fs::write(dir.join("preimage"), &norm)?;
        eprintln!("Updated preimage for '{path}'");
        fs::remove_file(&post)?;
        eprintln!("Forgot resolution for '{path}'");
        rr.retain(|(_, p)| p != &path);
        rr.push((id, path));
    }
    write_merge_rr(repo, &rr)
}

fn clear(repo: &Repository) -> CliResult<()> {
    let cache = rr_cache(repo);
    for (id, _) in read_merge_rr(repo) {
        let dir = cache.join(&id);
        if !dir.join("postimage").is_file() {
            let _ = fs::remove_dir_all(&dir);
        }
    }
    let _ = fs::remove_file(merge_rr_path(repo));
    Ok(())
}

fn mtime_age_days(p: &Path, now: std::time::SystemTime) -> Option<u64> {
    let m = fs::metadata(p).ok()?.modified().ok()?;
    Some(now.duration_since(m).ok()?.as_secs() / 86_400)
}

fn gc(repo: &Repository) -> CliResult<()> {
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let days =
        |k: &str, d: u64| -> u64 { cfg.get(k).and_then(|v| v.trim().parse().ok()).unwrap_or(d) };
    let (resolved, unresolved) = (
        days("gc.rerereResolved", 60),
        days("gc.rerereUnresolved", 15),
    );
    let now = std::time::SystemTime::now();
    fs::create_dir_all(rr_cache(repo))?;
    let Ok(rd) = fs::read_dir(rr_cache(repo)) else {
        return Ok(());
    };
    for ent in rd.flatten() {
        let dir = ent.path();
        if !dir.is_dir() {
            continue;
        }
        let post = dir.join("postimage");
        let (file, limit) = if post.is_file() {
            (post, resolved)
        } else {
            (dir.join("preimage"), unresolved)
        };
        if mtime_age_days(&file, now).is_some_and(|age| age > limit) {
            let _ = fs::remove_dir_all(&dir);
        }
    }
    Ok(())
}

fn diff(repo: &Repository) -> CliResult<()> {
    let work = work_tree(repo)?;
    let cache = rr_cache(repo);
    let mut out = io::stdout().lock();
    for (id, path) in read_merge_rr(repo) {
        let pre = fs::read(cache.join(&id).join("preimage")).unwrap_or_default();
        let cur = fs::read(work.join(&path)).unwrap_or_default();
        if pre == cur {
            continue;
        }
        writeln!(out, "--- a/{path}\n+++ b/{path}")?;
        write_hunks(&mut out, &pre, &cur, 3, HunkStyle::default())?;
    }
    Ok(())
}

pub fn cmd_rerere(args: &[String]) -> CliResult<()> {
    if args.len() == 1 && args[0] == "-h" {
        println!("{USAGE}");
        return Err(CliError::silent(129));
    }
    let p = parse(
        "rerere",
        args,
        &[o("rerere-autoupdate", ' ', No, true)],
        Mode::default(),
    )?;
    let argv = p.args();
    let sub = argv.first().map(String::as_str);
    if !matches!(
        sub,
        None | Some("clear" | "forget" | "diff" | "status" | "remaining" | "gc")
    ) {
        return Err(CliError::Exit(129, USAGE.to_string()));
    }
    let repo = open_repo()?;
    // SEQ-004 (M20 Cx-6): refuse while another writer holds the rr-cache lock.
    let lock = repo.common_dir().join("rr-cache.lock");
    if lock.is_file() {
        return Err(CliError::Msg(format!(
            "{} (SEQ-004 — concurrent rerere rr-cache writes refuse; serialize conflict resolution)",
            lock.display()
        )));
    }
    if !enabled(&repo) {
        return Ok(());
    }
    match sub {
        None => {
            let au = p
                .flag("rerere-autoupdate")
                .unwrap_or_else(|| config_bool(&repo, "rerere.autoUpdate").unwrap_or(false));
            do_rerere(&repo, au)
        }
        Some("clear") => clear(&repo),
        Some("forget") => forget(&repo, &argv[1..]),
        Some("diff") => diff(&repo),
        Some("gc") => gc(&repo),
        Some("status") => {
            let mut out = io::stdout().lock();
            for (_, path) in read_merge_rr(&repo) {
                writeln!(out, "{path}")?;
            }
            Ok(())
        }
        _ => {
            // remaining: tracked conflicts still unmerged, plus conflicts
            // rerere cannot track (a side is missing, e.g. modify/delete).
            let index = read_index(&repo)?;
            let rr = read_merge_rr(&repo);
            let mut out = io::stdout().lock();
            for path in conflicted_paths(&repo)? {
                let stages = |n: u16| {
                    index
                        .entries
                        .iter()
                        .any(|e| e.path == path && e.stage() == n)
                };
                let tracked = rr.iter().any(|(_, p)| p == &path);
                if tracked || !(stages(2) && stages(3)) {
                    writeln!(out, "{path}")?;
                }
            }
            Ok(())
        }
    }
}
