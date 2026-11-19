//! `apply`: apply a unified diff (git diff / format-patch output) to the work
//! tree, the index (`--cached`) or both (`--index`); `--stat`, `--numstat`
//! and `--check` inspect without applying; `-R` reverses. All files are
//! checked before anything is written.
//!
//! Primary: <https://git-scm.com/docs/git-apply>.
//!
//! Partial: `--3way`, `-p<n>` other than 1, whitespace options, binary
//! patches and `--reject` are not claimed.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree, write_index};
use arks_index::sort_entries;
use arks_object::{FilePatch, ObjectKind, RawObject, apply_hunks, parse_unified_diff};
use arks_store::ObjectStore;
use std::collections::HashMap;
use std::fs;

#[derive(Default)]
struct Opts {
    stat: bool,
    numstat: bool,
    check: bool,
    reverse: bool,
    index: bool,
    cached: bool,
    apply: bool,
    files: Vec<String>,
}

fn parse_opts(args: &[String]) -> CliResult<Opts> {
    let mut o = Opts::default();
    for a in args {
        match a.as_str() {
            "--stat" => o.stat = true,
            "--numstat" => o.numstat = true,
            "--check" => o.check = true,
            "-R" | "--reverse" => o.reverse = true,
            "--index" => o.index = true,
            "--cached" => o.cached = true,
            "--apply" => o.apply = true,
            "-p1" | "-v" | "--verbose" | "-q" | "--quiet" => {}
            _ if a.starts_with('-') && a != "-" => {
                return Err(CliError::NotImplemented(format!("unsupported option: {a}")));
            }
            _ => o.files.push(a.clone()),
        }
    }
    Ok(o)
}

pub fn cmd_apply(args: &[String]) -> CliResult<()> {
    let o = parse_opts(args)?;
    let mut text = Vec::new();
    if o.files.is_empty() || o.files.iter().any(|f| f == "-") {
        std::io::Read::read_to_end(&mut std::io::stdin(), &mut text)?;
    }
    for f in o.files.iter().filter(|f| *f != "-") {
        let bytes =
            fs::read(f).map_err(|e| CliError::fatal(format!("can't open patch '{f}': {e}")))?;
        text.extend_from_slice(&bytes);
    }
    let mut patches = parse_unified_diff(&text).map_err(|e| CliError::fatal(e.to_string()))?;
    if o.reverse {
        patches = patches.iter().map(FilePatch::reversed).collect();
    }
    if o.numstat {
        for p in &patches {
            let (i, d) = p.line_counts();
            if p.binary {
                println!("-\t-\t{}", p.display_path());
            } else {
                println!("{i}\t{d}\t{}", p.display_path());
            }
        }
    }
    if o.stat {
        print_stat(&patches);
    }
    // `--stat`/`--numstat` alone only describe the patch (no applicability check).
    if (o.stat || o.numstat) && !o.check && !o.apply {
        return Ok(());
    }
    let inspect_only = (o.stat || o.numstat || o.check) && !o.apply;
    let repo = open_repo()?;
    let use_work = !o.cached;
    let use_index = o.cached || o.index;
    let work = work_tree(&repo).ok();
    if use_work && work.is_none() {
        return Err(CliError::fatal(
            "apply without --cached requires a work tree",
        ));
    }
    let mut index = read_index(&repo)?;
    let idx_blob = |index: &arks_index::Index, path: &str| {
        index
            .entries
            .iter()
            .find(|e| e.path == path && e.stage() == 0)
            .map(|e| (e.oid, e.mode))
    };
    let mut errors = Vec::new();
    // Planned results: path -> new content (None = delete), plus mode.
    let mut results: Vec<(String, Option<Vec<u8>>, u32)> = Vec::new();
    // Content as already patched in this run (several patches may touch one path).
    let mut staged: HashMap<String, Option<Vec<u8>>> = HashMap::new();
    for p in &patches {
        let path = p.display_path().to_string();
        if p.binary {
            errors.push(format!(
                "error: cannot apply binary patch to '{path}' without full index line"
            ));
            continue;
        }
        let read_current = |path: &str| -> CliResult<Option<Vec<u8>>> {
            if let Some(v) = staged.get(path) {
                return Ok(v.clone());
            }
            if use_work {
                let abs = work.as_ref().expect("work tree").join(path);
                return Ok(fs::read(&abs).ok());
            }
            match idx_blob(&index, path) {
                Some((oid, _)) => Ok(Some(repo.get_object(&oid)?.payload)),
                None => Ok(None),
            }
        };
        let current = match &p.old_path {
            Some(op) => {
                let cur = read_current(op)?;
                if cur.is_none() {
                    let what = if use_work {
                        "No such file or directory"
                    } else {
                        "does not exist in index"
                    };
                    errors.push(format!("error: {op}: {what}"));
                    continue;
                }
                if o.index && use_work && !staged.contains_key(op.as_str()) {
                    let in_index = idx_blob(&index, op).map(|(oid, _)| oid);
                    let on_disk = cur
                        .as_ref()
                        .map(|b| {
                            RawObject::new(ObjectKind::Blob, b.clone()).oid(repo.object_format())
                        })
                        .transpose()?;
                    if in_index.is_none() || in_index != on_disk {
                        errors.push(format!("error: {op}: does not match index"));
                        continue;
                    }
                }
                cur.unwrap_or_default()
            }
            None => {
                if read_current(&path)?.is_some() {
                    let where_ = if use_work {
                        "already exists in working directory"
                    } else {
                        "already exists in index"
                    };
                    errors.push(format!("error: {path}: {where_}"));
                    continue;
                }
                Vec::new()
            }
        };
        match apply_hunks(&current, p) {
            Ok(new) => {
                let mode = p
                    .new_mode
                    .or_else(|| {
                        p.old_path
                            .as_deref()
                            .and_then(|op| idx_blob(&index, op))
                            .map(|(_, m)| m)
                    })
                    .unwrap_or(0o100644);
                if let (Some(op), Some(np)) = (&p.old_path, &p.new_path) {
                    if op != np {
                        staged.insert(op.clone(), None);
                        results.push((op.clone(), None, mode));
                    }
                }
                match &p.new_path {
                    Some(np) => {
                        staged.insert(np.clone(), Some(new.clone()));
                        results.push((np.clone(), Some(new), mode));
                    }
                    None => {
                        let op = p.old_path.clone().unwrap_or_default();
                        if !new.is_empty() {
                            errors.push(format!(
                                "error: removal patch leaves file contents\nerror: {op}: patch does not apply"
                            ));
                            continue;
                        }
                        staged.insert(op.clone(), None);
                        results.push((op, None, mode));
                    }
                }
            }
            Err(line) => {
                let op = p.old_path.as_deref().unwrap_or(&path);
                errors.push(format!(
                    "error: patch failed: {op}:{line}\nerror: {op}: patch does not apply"
                ));
            }
        }
    }
    if !errors.is_empty() {
        return Err(CliError::Exit(1, errors.join("\n")));
    }
    if inspect_only {
        return Ok(());
    }
    let mut repo = repo;
    for (path, content, mode) in &results {
        if use_work {
            let abs = work.as_ref().expect("work tree").join(path);
            match content {
                Some(bytes) => {
                    if let Some(parent) = abs.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&abs, bytes)?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let perm = if *mode == 0o100755 { 0o755 } else { 0o644 };
                        let _ = fs::set_permissions(&abs, fs::Permissions::from_mode(perm));
                    }
                }
                None => {
                    let _ = fs::remove_file(&abs);
                }
            }
        }
        if use_index {
            index.entries.retain(|e| &e.path != path);
            if let Some(bytes) = content {
                let format = repo.object_format();
                let oid = repo
                    .store_mut()
                    .put(format, &RawObject::new(ObjectKind::Blob, bytes.clone()))?;
                let mut entry = match (use_work, &work) {
                    (true, Some(w)) => {
                        let meta = fs::metadata(w.join(path))?;
                        crate::cmd::working_tree::stat_entry(path, oid, &meta)
                    }
                    _ => arks_index::IndexEntry {
                        ctime_sec: 0,
                        ctime_nsec: 0,
                        mtime_sec: 0,
                        mtime_nsec: 0,
                        dev: 0,
                        ino: 0,
                        mode: *mode,
                        uid: 0,
                        gid: 0,
                        size: bytes.len().min(u32::MAX as usize) as u32,
                        oid,
                        flags: (path.len().min(0x0fff)) as u16,
                        flags_extended: None,
                        path: path.clone(),
                    },
                };
                entry.mode = *mode;
                index.entries.push(entry);
            }
        }
    }
    if use_index {
        sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
    }
    Ok(())
}

/// `apply --stat` (git apply.c `show_stats`): names padded to the longest
/// (max 50, longer ones shortened with `...`), a 5-wide change count and a
/// `+`/`-` bar scaled to fit 70 columns, then the diffstat summary line.
fn print_stat(patches: &[FilePatch]) {
    let names: Vec<&str> = patches.iter().map(FilePatch::display_path).collect();
    let max_len = names.iter().map(|n| n.chars().count()).max().unwrap_or(0);
    let max_change = patches
        .iter()
        .map(|p| {
            let (i, d) = p.line_counts();
            i + d
        })
        .max()
        .unwrap_or(0);
    let (mut adds, mut dels) = (0, 0);
    for (p, name) in patches.iter().zip(&names) {
        let width = max_len.min(50);
        let len = name.chars().count();
        let shown = if len > width {
            let cut = len + 3 - width;
            let tail: String = name.chars().skip(cut).collect();
            let tail = match tail.find('/') {
                Some(i) => tail[i..].to_string(),
                None => tail,
            };
            format!("...{tail}")
        } else {
            name.to_string()
        };
        if p.binary {
            println!(" {shown:<width$} |  Bin");
            continue;
        }
        let (add, del) = p.line_counts();
        adds += add;
        dels += del;
        let scale = if width + max_change > 70 {
            70 - width
        } else {
            max_change
        };
        let half = max_change / 2;
        let (a, d) = match (
            ((add + del) * scale + half).checked_div(max_change),
            (add * scale + half).checked_div(max_change),
        ) {
            (Some(total), Some(a)) => (a, total - a),
            _ => (add, del),
        };
        println!(
            " {shown:<width$} |{:5} {}{}",
            add + del,
            "+".repeat(a),
            "-".repeat(d)
        );
    }
    let files = patches.len();
    let mut line = format!(" {files} file{} changed", if files == 1 { "" } else { "s" });
    if adds > 0 || dels == 0 {
        line.push_str(&format!(
            ", {adds} insertion{}(+)",
            if adds == 1 { "" } else { "s" }
        ));
    }
    if dels > 0 || adds == 0 {
        line.push_str(&format!(
            ", {dels} deletion{}(-)",
            if dels == 1 { "" } else { "s" }
        ));
    }
    println!("{line}");
}
