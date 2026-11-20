//! `apply`: apply a unified diff (git diff / format-patch output) to the work
//! tree, the index (`--cached`) or both (`--index`); `--stat`, `--numstat`
//! and `--check` inspect without applying; `-R` reverses. All files are
//! checked before anything is written.
//!
//! Primary: <https://git-scm.com/docs/git-apply>.
//!
//! `-p<n>`, `--directory`, `--include` / `--exclude`, `--no-add`,
//! `--summary`, `--whitespace`, `--reject` (`<path>.rej`), `--3way` (merge
//! with the pre-image blob, conflicts as index stages) and
//! `--build-fake-ancestor` follow git-apply(1).
//!
//! Partial: binary patches are refused; `-C<n>`, `--ignore-space-change`,
//! `--ignore-whitespace`, `--unidiff-zero`, `--recount`, `--inaccurate-eof`
//! and `--allow-overlap` are accepted but hunks still need exact context.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree, write_index};
use arks_index::sort_entries;
use arks_object::{FilePatch, HunkLine, ObjectKind, RawObject, apply_hunks, parse_unified_diff};
use arks_store::ObjectStore;
use std::collections::HashMap;
use std::fs;

#[derive(Default)]
struct Opts {
    stat: bool,
    numstat: bool,
    summary: bool,
    check: bool,
    reverse: bool,
    index: bool,
    cached: bool,
    apply: bool,
    no_add: bool,
    intent_to_add: bool,
    three_way: bool,
    nul: bool,
    reject: bool,
    verbose: bool,
    quiet: bool,
    allow_empty: bool,
    strip: usize,
    directory: Option<String>,
    /// `--include` / `--exclude` rules in order: (glob, include).
    rules: Vec<(String, bool)>,
    whitespace: Whitespace,
    fake_ancestor: Option<String>,
    files: Vec<String>,
}

/// `--whitespace=<action>` for added lines with whitespace errors.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Whitespace {
    #[default]
    Warn,
    NoWarn,
    Fix,
    Error,
    ErrorAll,
}

fn parse_opts(args: &[String]) -> CliResult<Opts> {
    // Every option of `git apply -h`. Cite: https://git-scm.com/docs/git-apply
    use crate::cmd::opt_parse::{Item, Mode, parse};
    let p = parse(
        "apply",
        args,
        crate::cmd::porcelain_opts::APPLY,
        Mode::default(),
    )?;
    let mut o = Opts {
        stat: p.on("stat"),
        numstat: p.on("numstat"),
        summary: p.on("summary"),
        check: p.on("check"),
        reverse: p.on("reverse"),
        index: p.on("index"),
        cached: p.on("cached"),
        apply: p.on("apply"),
        no_add: p.on("no-add"),
        intent_to_add: p.on("intent-to-add"),
        three_way: p.on("3way"),
        nul: p.on("-z"),
        reject: p.on("reject"),
        verbose: p.on("verbose"),
        quiet: p.on("quiet"),
        allow_empty: p.on("allow-empty"),
        strip: 1,
        directory: p.value("directory"),
        rules: Vec::new(),
        whitespace: Whitespace::Warn,
        fake_ancestor: p.value("build-fake-ancestor"),
        files: p.args(),
    };
    o.files.extend(p.after_dashdash());
    if let Some(n) = p.value("-p") {
        o.strip = n.parse().map_err(|_| {
            CliError::Exit(129, format!("error: switch `p' expects a numerical value"))
        })?;
    }
    if let Some(c) = p.value("-C") {
        c.parse::<usize>().map_err(|_| {
            CliError::Exit(129, format!("error: switch `C' expects a numerical value"))
        })?;
    }
    for it in &p.items {
        if let Item::Opt {
            key: k @ ("include" | "exclude"),
            val: Some(v),
            ..
        } = it
        {
            o.rules.push((v.clone(), *k == "include"));
        }
    }
    if let Some(w) = p.value("whitespace") {
        o.whitespace = match w.as_str() {
            "warn" => Whitespace::Warn,
            "nowarn" => Whitespace::NoWarn,
            "fix" | "strip" => Whitespace::Fix,
            "error" => Whitespace::Error,
            "error-all" => Whitespace::ErrorAll,
            other => {
                return Err(CliError::Exit(
                    129,
                    format!("error: unrecognized whitespace option '{other}'"),
                ));
            }
        };
    }
    if o.three_way {
        if o.reject {
            return Err(CliError::fatal(
                "options '--reject' and '--3way' cannot be used together",
            ));
        }
        if o.cached {
            o.cached = true;
        } else {
            o.index = true;
        }
    }
    // `--no-apply` turns applying off even with --index / --cached.
    if p.flag("apply") == Some(false) {
        o.check = true;
    }
    Ok(o)
}

/// Rewrite header paths for `-p<n>` and `--directory` so the parser's
/// `a/` / `b/` stripping yields the paths git would use.
fn rewrite_paths(text: &[u8], o: &Opts) -> CliResult<Vec<u8>> {
    rewrite_patch_paths(text, o.strip, o.directory.as_deref())
}

/// [`rewrite_paths`] for callers outside `apply` (`am -p` / `--directory`).
pub(crate) fn rewrite_patch_paths(
    text: &[u8],
    strip: usize,
    directory: Option<&str>,
) -> CliResult<Vec<u8>> {
    if strip == 1 && directory.is_none() {
        return Ok(text.to_vec());
    }
    let dir = directory
        .map(|d| format!("{}/", d.trim_end_matches('/')))
        .unwrap_or_default();
    let fix = |raw: &str, side: &str| -> CliResult<String> {
        let (path, tail) = match raw.split_once('\t') {
            Some((p, t)) => (p, format!("\t{t}")),
            None => (raw, String::new()),
        };
        if path == "/dev/null" {
            return Ok(raw.to_string());
        }
        let comps: Vec<&str> = path.split('/').collect();
        if comps.len() <= strip {
            return Err(CliError::fatal(format!(
                "git apply: bad git-diff - expected /dev/null, got {path}"
            )));
        }
        Ok(format!("{side}/{dir}{}{tail}", comps[strip..].join("/")))
    };
    let mut out = String::new();
    for line in String::from_utf8_lossy(text).split_inclusive('\n') {
        let body = line.trim_end_matches('\n');
        let nl = if line.ends_with('\n') { "\n" } else { "" };
        if let Some(rest) = body.strip_prefix("diff --git ") {
            let mut parts = rest.splitn(2, ' ');
            let (a, b) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
            out.push_str(&format!("diff --git {} {}{nl}", fix(a, "a")?, fix(b, "b")?));
        } else if let Some(rest) = body.strip_prefix("--- ") {
            out.push_str(&format!("--- {}{nl}", fix(rest, "a")?));
        } else if let Some(rest) = body.strip_prefix("+++ ") {
            out.push_str(&format!("+++ {}{nl}", fix(rest, "b")?));
        } else if let Some(rest) = body
            .strip_prefix("rename from ")
            .map(|r| ("rename from ", r))
            .or_else(|| body.strip_prefix("rename to ").map(|r| ("rename to ", r)))
            .map(|(k, r)| format!("{k}{dir}{r}"))
        {
            out.push_str(&rest);
            out.push_str(nl);
        } else {
            out.push_str(line);
        }
    }
    Ok(out.into_bytes())
}

/// Whether `path` passes the `--include` / `--exclude` rules (first match wins;
/// with only include rules, unmatched paths are left out).
pub(crate) fn path_selected(path: &str, rules: &[(String, bool)]) -> bool {
    for (glob, include) in rules {
        if crate::cmd::log_format::path_in_specs(path, std::slice::from_ref(glob))
            || glob_match(glob, path)
        {
            return *include;
        }
    }
    !rules.iter().any(|(_, inc)| *inc)
}

/// fnmatch(3) without FNM_PATHNAME: `*` and `?` also match `/`.
fn glob_match(pat: &str, s: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = s.chars().collect();
    fn rec(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') => (0..=t.len()).any(|i| rec(&p[1..], &t[i..])),
            Some('?') => !t.is_empty() && rec(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && rec(&p[1..], &t[1..]),
        }
    }
    rec(&p, &t)
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
    let text = rewrite_paths(&text, &o)?;
    let mut patches = match parse_unified_diff(&text) {
        Ok(p) => p,
        Err(_) if o.allow_empty && !text.iter().any(|b| !b.is_ascii_whitespace()) => Vec::new(),
        Err(e) => return Err(CliError::fatal(e.to_string())),
    };
    if patches.is_empty() && !o.allow_empty {
        return Err(CliError::fatal(
            "No valid patches in input (allow with \"--allow-empty\")",
        ));
    }
    patches.retain(|p| path_selected(p.display_path(), &o.rules));
    if o.reverse {
        patches = patches.iter().map(FilePatch::reversed).collect();
    }
    if o.no_add {
        // `--no-add`: apply only the deletions.
        for p in &mut patches {
            for h in &mut p.hunks {
                h.lines.retain(|l| !matches!(l, HunkLine::Add(_)));
            }
        }
    }
    // Whitespace errors in added lines (trailing whitespace).
    let mut ws_errors = 0usize;
    for p in &mut patches {
        for h in &mut p.hunks {
            for l in &mut h.lines {
                if let HunkLine::Add(b) = l {
                    let body = b.strip_suffix(b"\n").unwrap_or(b);
                    if body.ends_with(b" ") || body.ends_with(b"\t") {
                        ws_errors += 1;
                        if o.whitespace == Whitespace::Fix {
                            let keep = body.iter().rposition(|c| *c != b' ' && *c != b'\t');
                            let mut fixed = keep.map(|k| body[..=k].to_vec()).unwrap_or_default();
                            if b.ends_with(b"\n") {
                                fixed.push(b'\n');
                            }
                            *b = fixed;
                        }
                    }
                }
            }
        }
    }
    if ws_errors > 0 && matches!(o.whitespace, Whitespace::Error | Whitespace::ErrorAll) {
        return Err(CliError::fatal(format!(
            "{ws_errors} line{} adds whitespace errors.",
            if ws_errors == 1 { "" } else { "s" }
        )));
    }
    if ws_errors > 0 && o.whitespace == Whitespace::Warn && !o.quiet {
        eprintln!(
            "warning: {ws_errors} line{} adds whitespace errors.",
            if ws_errors == 1 { "" } else { "s" }
        );
    }
    let eol = if o.nul { '\0' } else { '\n' };
    if o.numstat {
        for p in &patches {
            let (i, d) = p.line_counts();
            if p.binary {
                print!("-\t-\t{}{eol}", p.display_path());
            } else {
                print!("{i}\t{d}\t{}{eol}", p.display_path());
            }
        }
    }
    if o.stat {
        print_stat(&patches);
    }
    if o.summary {
        for p in &patches {
            match (&p.old_path, &p.new_path) {
                (None, Some(n)) => {
                    println!(" create mode {:o} {n}", p.new_mode.unwrap_or(0o100644))
                }
                (Some(old), None) => {
                    println!(" delete mode {:o} {old}", p.new_mode.unwrap_or(0o100644))
                }
                (Some(a), Some(b)) if a != b => println!(" rename {a} => {b} (100%)"),
                _ => {}
            }
        }
    }
    if let Some(path) = &o.fake_ancestor {
        return build_fake_ancestor(&patches, path);
    }
    // `--stat`/`--numstat`/`--summary` alone only describe the patch.
    if (o.stat || o.numstat || o.summary) && !o.check && !o.apply {
        return Ok(());
    }
    let inspect_only = (o.stat || o.numstat || o.summary || o.check) && !o.apply;
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
    // `--reject`: (path, rejected hunk numbers, hunk count, rejected hunks).
    let mut rejects: Vec<(String, Vec<usize>, usize, Vec<arks_object::Hunk>)> = Vec::new();
    // `--3way` conflicts: (path, base, ours, theirs) blobs for stages 1/2/3.
    let mut conflicts: Vec<(String, Option<Vec<u8>>, Vec<u8>, Vec<u8>)> = Vec::new();
    let mut three_way_msgs: Vec<String> = Vec::new();
    for p in &patches {
        if o.verbose && !o.quiet {
            eprintln!("Checking patch {}...", p.display_path());
        }
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
                let op = p.old_path.as_deref().unwrap_or(&path).to_string();
                if o.reject {
                    // Apply what applies; keep the rest for `<path>.rej`.
                    let mut content = current.clone();
                    let mut bad = Vec::new();
                    let mut bad_hunks = Vec::new();
                    for (k, h) in p.hunks.iter().enumerate() {
                        let single = FilePatch {
                            hunks: vec![h.clone()],
                            ..p.clone()
                        };
                        match apply_hunks(&content, &single) {
                            Ok(n) => content = n,
                            Err(_) => {
                                bad.push(k + 1);
                                bad_hunks.push(h.clone());
                            }
                        }
                    }
                    eprintln!("error: patch failed: {op}:{line}");
                    let np = p.new_path.clone().unwrap_or(op.clone());
                    let mode = idx_blob(&index, &op).map(|(_, m)| m).unwrap_or(0o100644);
                    staged.insert(np.clone(), Some(content.clone()));
                    results.push((np.clone(), Some(content), mode));
                    rejects.push((np, bad, p.hunks.len(), bad_hunks));
                    continue;
                }
                if o.three_way {
                    // Fall back to a three-way merge with the pre-image blob.
                    let base = p
                        .index_oids
                        .as_ref()
                        .and_then(|(a, _)| crate::cmd::shared::resolve_rev(&repo, a).ok())
                        .and_then(|oid| repo.get_object(&oid).ok())
                        .filter(|obj| obj.kind == ObjectKind::Blob)
                        .map(|obj| obj.payload);
                    let theirs = base.as_ref().and_then(|b| apply_hunks(b, p).ok());
                    if let (Some(base), Some(theirs)) = (base, theirs) {
                        three_way_msgs.push(format!(
                            "error: patch failed: {op}:{line}\nFalling back to three-way merge..."
                        ));
                        let labels = crate::cmd::merge_engine::MergeLabels {
                            ours: "ours".into(),
                            base: "base".into(),
                            theirs: "theirs".into(),
                            diff3: false,
                            favor: crate::cmd::merge_engine::Favor::None,
                        };
                        let (merged, conflicted) =
                            crate::cmd::merge_engine::merge_text(&base, &current, &theirs, &labels);
                        let np = p.new_path.clone().unwrap_or(op.clone());
                        let mode = idx_blob(&index, &op).map(|(_, m)| m).unwrap_or(0o100644);
                        staged.insert(np.clone(), Some(merged.clone()));
                        results.push((np.clone(), Some(merged), mode));
                        if conflicted {
                            three_way_msgs.push(format!("Applied patch to '{np}' with conflicts."));
                            conflicts.push((np, Some(base), current.clone(), theirs));
                        } else {
                            three_way_msgs.push(format!("Applied patch to '{np}' cleanly."));
                        }
                        continue;
                    }
                    errors.push(format!(
                        "error: patch failed: {op}:{line}\nerror: repository lacks the necessary blob to perform 3-way merge.\nerror: {op}: patch does not apply"
                    ));
                    continue;
                }
                errors.push(format!(
                    "error: patch failed: {op}:{line}\nerror: {op}: patch does not apply"
                ));
            }
        }
    }
    if !errors.is_empty() {
        return Err(CliError::Exit(1, errors.join("\n")));
    }
    for m in &three_way_msgs {
        eprintln!("{m}");
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
    // `--3way` conflicts: stages 1/2/3 in the index, markers in the file.
    for (path, base, ours, theirs) in &conflicts {
        index.entries.retain(|e| &e.path != path);
        let mode = 0o100644;
        for (stage, blob) in [(1u16, base.as_ref()), (2, Some(ours)), (3, Some(theirs))] {
            let Some(bytes) = blob else { continue };
            let format = repo.object_format();
            let oid = repo
                .store_mut()
                .put(format, &RawObject::new(ObjectKind::Blob, bytes.clone()))?;
            index.entries.push(arks_index::IndexEntry {
                ctime_sec: 0,
                ctime_nsec: 0,
                mtime_sec: 0,
                mtime_nsec: 0,
                dev: 0,
                ino: 0,
                mode,
                uid: 0,
                gid: 0,
                size: 0,
                oid,
                flags: (path.len().min(0x0fff)) as u16 | (stage << 12),
                flags_extended: None,
                path: path.clone(),
            });
        }
        eprintln!("U {path}");
    }
    // `-N` without `--index`: new files become intent-to-add entries.
    if o.intent_to_add && !use_index && use_work {
        for p in patches.iter().filter(|p| p.old_path.is_none()) {
            let Some(np) = &p.new_path else { continue };
            if index.entries.iter().any(|e| &e.path == np) {
                continue;
            }
            let meta = fs::metadata(work.as_ref().expect("work tree").join(np))?;
            let mut e = crate::cmd::working_tree::stat_entry(
                np,
                RawObject::new(ObjectKind::Blob, Vec::new()).oid(repo.object_format())?,
                &meta,
            );
            e.size = 0;
            e.flags |= arks_index::CE_EXTENDED;
            e.flags_extended = Some(crate::cmd::core::INTENT_TO_ADD);
            index.entries.push(e);
        }
    }
    if use_index || !conflicts.is_empty() || o.intent_to_add {
        sort_entries(&mut index.entries);
        write_index(&repo, &index)?;
    }
    if o.verbose && !o.quiet && rejects.is_empty() {
        for p in &patches {
            eprintln!("Applied patch {} cleanly.", p.display_path());
        }
    }
    if !rejects.is_empty() {
        for (path, bad, total, hunks) in &rejects {
            eprintln!(
                "Applying patch {path} with {} reject{}...",
                bad.len(),
                if bad.len() == 1 { "" } else { "s" }
            );
            for k in 1..=*total {
                if bad.contains(&k) {
                    eprintln!("Rejected hunk #{k}.");
                } else {
                    eprintln!("Hunk #{k} applied cleanly.");
                }
            }
            if let Some(w) = &work {
                let mut rej = format!("diff a/{path} b/{path}\t(rejected hunks)\n");
                for h in hunks {
                    rej.push_str(&format!(
                        "@@ -{},{} +{},{} @@\n",
                        h.old_start, h.old_count, h.new_start, h.new_count
                    ));
                    for l in &h.lines {
                        let (c, b) = match l {
                            HunkLine::Context(b) => (' ', b),
                            HunkLine::Del(b) => ('-', b),
                            HunkLine::Add(b) => ('+', b),
                        };
                        rej.push(c);
                        rej.push_str(&String::from_utf8_lossy(b));
                        if !b.ends_with(b"\n") {
                            rej.push('\n');
                        }
                    }
                }
                fs::write(w.join(format!("{path}.rej")), rej)?;
            }
        }
        return Err(CliError::silent(1));
    }
    if !conflicts.is_empty() {
        return Err(CliError::silent(1));
    }
    Ok(())
}

/// `--build-fake-ancestor=<file>`: an index of the patch's pre-image blobs.
fn build_fake_ancestor(patches: &[FilePatch], out: &str) -> CliResult<()> {
    let repo = open_repo()?;
    let mut entries = Vec::new();
    for p in patches {
        let path = p
            .old_path
            .clone()
            .unwrap_or_else(|| p.display_path().to_string());
        let oid = p
            .index_oids
            .as_ref()
            .and_then(|(a, _)| crate::cmd::shared::resolve_rev(&repo, a).ok())
            .ok_or_else(|| {
                CliError::fatal(format!("sha1 information is lacking or useless ({path})."))
            })?;
        entries.push(arks_index::IndexEntry {
            ctime_sec: 0,
            ctime_nsec: 0,
            mtime_sec: 0,
            mtime_nsec: 0,
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            size: 0,
            oid,
            flags: (path.len().min(0x0fff)) as u16,
            flags_extended: None,
            path,
        });
    }
    sort_entries(&mut entries);
    let index = arks_index::Index {
        version: 2,
        entries,
        extensions: Vec::new(),
    };
    index
        .write_path(std::path::Path::new(out), repo.oid_kind())
        .map_err(|e| CliError::Msg(e.to_string()))?;
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
