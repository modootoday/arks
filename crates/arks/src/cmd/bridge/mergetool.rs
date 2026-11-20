//! `git mergetool`: run the configured merge tool on each conflicted path.
//!
//! Primary: <https://git-scm.com/docs/git-mergetool> (options, `TEMPORARY
//! FILES`, `mergetool.*` configuration). Observed peer behaviour (git
//! 2.39/2.43): the `Merging:` path list, the `Normal merge conflict` /
//! `Deleted merge conflict` descriptions, the `<name>_{BASE,LOCAL,REMOTE,
//! BACKUP}_<pid><ext>` temporary files beside the merged file, the
//! `seems unchanged` question for untrusted tools, `<path>.orig` backups and
//! the `Continue merging other unresolved paths [y/n]?` question.

use super::proc;
use super::tools::{Files, Mode, ToolConfig, launch};
use crate::cmd::opt_parse::{OptDef, Takes::*, o, parse};
use crate::cmd::plumbing_ext::{MergeFavor, MergeStyle, merge_three};
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, work_tree, write_index};
use arks_config::ConfigSet;
use arks_index::IndexEntry;
use arks_object::{ObjectKind, RawObject};
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const OPTS: &[OptDef] = &[
    o("tool", 't', Req, true),
    o("tool-help", ' ', No, true),
    o("no-prompt", 'y', No, true),
    o("gui", 'g', No, true),
    o("-O", 'O', Req, false),
];

const USAGE: &str = "usage: git mergetool [--tool=tool] [--tool-help] [-y|--no-prompt|--prompt] [-g|--gui|--no-gui] [-O<orderfile>] [file to merge] ...";

/// Stages of one conflicted path (1 base, 2 ours, 3 theirs).
#[derive(Default)]
struct Stages {
    base: Option<IndexEntry>,
    local: Option<IndexEntry>,
    remote: Option<IndexEntry>,
}

struct Ctx<'a> {
    repo: &'a mut Repository,
    top: PathBuf,
    tc: ToolConfig<'a>,
    tool: Option<String>,
    prompt: bool,
}

pub fn cmd_mergetool(args: &[String]) -> CliResult<()> {
    let p = parse("mergetool", args, OPTS, Default::default())
        .map_err(|_| CliError::Exit(1, USAGE.into()))?;
    let mut repo = open_repo()?;
    let cfg = ConfigSet::load(Some(repo.common_dir()));
    let tc = ToolConfig {
        cfg: &cfg,
        mode: Mode::Merge,
    };
    if p.on("tool-help") {
        tc.print_help();
        return Ok(());
    }
    let gui = p.on("gui")
        || (p.flag("gui").is_none() && cfg.get_bool("mergetool.guiDefault") == Some(true));
    let (tool, guessed) = match p.value("tool") {
        Some(t) => (t, false),
        None => match tc.configured(gui) {
            Some(t) => (t, false),
            // No installed default is not fatal (observed pin 2.39.5): the
            // message goes to stderr and each conflicted path stays unresolved.
            None => match tc.guess() {
                Ok(t) => (t, true),
                Err(CliError::Exit(1, m)) => {
                    eprintln!("{m}");
                    (String::new(), true)
                }
                Err(e) => return Err(e),
            },
        },
    };
    let prompt = match p.flag("no-prompt") {
        Some(v) => !v,
        None => cfg.get_bool("mergetool.prompt").unwrap_or(guessed),
    };
    let top = work_tree(&repo)?;
    let specs: Vec<String> = p
        .args()
        .into_iter()
        .chain(p.after_dashdash())
        .map(|a| {
            crate::cmd::diff_cmd::cwd_prefix(&top)
                .trim_end_matches('/')
                .to_string()
                + "/"
                + &a
        })
        .map(|a| normalize(a.trim_start_matches('/')))
        .collect();
    let order = p.value("-O").or_else(|| cfg.get("diff.orderFile"));
    let files = conflicted(&repo, &specs, order.as_deref())?;
    if files.is_empty() {
        println!("No files need merging");
        return Ok(());
    }
    println!("Merging:");
    for (path, _) in &files {
        println!("{path}");
    }
    let valid = tc.valid(&tool);
    let mut ctx = Ctx {
        repo: &mut repo,
        top,
        tc: ToolConfig {
            cfg: &cfg,
            mode: Mode::Merge,
        },
        tool: valid.then_some(tool),
        prompt,
    };
    let mut failed = false;
    let n = files.len();
    for (i, (path, st)) in files.into_iter().enumerate() {
        let ok = merge_one(&mut ctx, &path, &st)?;
        if ok {
            continue;
        }
        failed = true;
        if i + 1 < n {
            match proc::ask("Continue merging other unresolved paths [y/n]? ") {
                Some(a) if a.trim().eq_ignore_ascii_case("y") => {}
                _ => return Err(CliError::silent(1)),
            }
        }
    }
    if failed {
        return Err(CliError::silent(1));
    }
    Ok(())
}

/// `./a/b` → `a/b`, `a/./b/` → `a/b`.
fn normalize(p: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in p.split('/') {
        match c {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

/// Conflicted paths under the pathspecs, sorted (or in orderfile order).
fn conflicted(
    repo: &Repository,
    specs: &[String],
    order: Option<&str>,
) -> CliResult<Vec<(String, Stages)>> {
    let mut map: BTreeMap<String, Stages> = BTreeMap::new();
    for e in read_index(repo)?.entries {
        let slot = map.entry(e.path.clone()).or_default();
        match e.stage() {
            1 => slot.base = Some(e),
            2 => slot.local = Some(e),
            3 => slot.remote = Some(e),
            _ => {}
        }
    }
    let mut out: Vec<(String, Stages)> = map
        .into_iter()
        .filter(|(_, s)| s.base.is_some() || s.local.is_some() || s.remote.is_some())
        .filter(|(p, _)| {
            specs.is_empty()
                || specs
                    .iter()
                    .any(|s| s.is_empty() || p == s || p.starts_with(&format!("{s}/")))
        })
        .collect();
    if let Some(file) = order {
        let text = fs::read_to_string(file).unwrap_or_default();
        let pats: Vec<&str> = text
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        let rank = |path: &str| -> usize {
            pats.iter()
                .position(|p| crate::cmd::branch_cmd::fnmatch(p.as_bytes(), path.as_bytes()))
                .unwrap_or(pats.len())
        };
        out.sort_by_key(|(p, _)| rank(p));
    }
    Ok(out)
}

/// How one side reads in the conflict description.
fn describe(repo: &Repository, e: Option<&IndexEntry>, base: bool) -> String {
    let Some(e) = e else {
        return "deleted".into();
    };
    match e.mode {
        0o120000 => {
            let target = repo
                .get_object(&e.oid)
                .map(|o| String::from_utf8_lossy(&o.payload).into_owned())
                .unwrap_or_default();
            format!("a symbolic link -> '{target}'")
        }
        0o160000 => format!("submodule commit {}", e.oid.to_hex()),
        _ if !base => "created file".into(),
        _ => "modified file".into(),
    }
}

/// Resolve one path; `Ok(false)` when it stays unresolved.
fn merge_one(ctx: &mut Ctx<'_>, path: &str, st: &Stages) -> CliResult<bool> {
    println!();
    let Some(tool) = ctx.tool.clone() else {
        return Ok(false);
    };
    let has_base = st.base.is_some();
    let local = describe(ctx.repo, st.local.as_ref(), has_base);
    let remote = describe(ctx.repo, st.remote.as_ref(), has_base);
    if st.local.is_none() || st.remote.is_none() {
        println!(
            "Deleted merge conflict for '{path}':\n  {{local}}: {local}\n  {{remote}}: {remote}"
        );
        return resolve_deleted(ctx, path, st);
    }
    let link = |e: &Option<IndexEntry>| e.as_ref().is_some_and(|e| e.mode == 0o120000);
    let sub = |e: &Option<IndexEntry>| e.as_ref().is_some_and(|e| e.mode == 0o160000);
    if link(&st.local) || link(&st.remote) || sub(&st.local) || sub(&st.remote) {
        let kind = if sub(&st.local) || sub(&st.remote) {
            "Submodule"
        } else {
            "Symbolic link"
        };
        println!(
            "{kind} merge conflict for '{path}':\n  {{local}}: {local}\n  {{remote}}: {remote}"
        );
        return resolve_sides(ctx, path, st);
    }
    println!("Normal merge conflict for '{path}':\n  {{local}}: {local}\n  {{remote}}: {remote}");
    if ctx.prompt
        && proc::ask(&format!(
            "Hit return to start merge resolution tool ({tool}): "
        ))
        .is_none()
    {
        return Ok(false);
    }
    run_tool(ctx, &tool, path, st)
}

/// Temporary file beside the merged path: `dir/name_LABEL_<pid>.ext`
/// (`mergetool.writeToTemp`: inside a private temporary directory).
fn temp_name(path: &str, label: &str, tmpdir: Option<&Path>) -> String {
    let (dir, file) = match path.rsplit_once('/') {
        Some((d, f)) => (format!("./{d}/"), f),
        None => ("./".to_string(), path),
    };
    let (stem, ext) = match file.rfind('.') {
        Some(i) if i > 0 => (&file[..i], &file[i..]),
        _ => (file, ""),
    };
    let name = format!("{stem}_{label}_{}{ext}", std::process::id());
    match tmpdir {
        Some(t) => t.join(name).to_string_lossy().into_owned(),
        None => format!("{dir}{name}"),
    }
}

fn blob(repo: &Repository, e: Option<&IndexEntry>) -> CliResult<Vec<u8>> {
    Ok(match e {
        Some(e) => repo.get_object(&e.oid)?.payload.to_vec(),
        None => Vec::new(),
    })
}

fn bool_cfg(ctx: &Ctx<'_>, tool: &str, var: &str, default: bool) -> bool {
    ctx.tc
        .cfg
        .get_bool(&format!("mergetool.{tool}.{var}"))
        .or_else(|| ctx.tc.cfg.get_bool(&format!("mergetool.{var}")))
        .unwrap_or(default)
}

/// Write the stage files, run the tool and judge the result.
fn run_tool(ctx: &mut Ctx<'_>, tool: &str, path: &str, st: &Stages) -> CliResult<bool> {
    let top = ctx.top.clone();
    let cfg = ctx.tc.cfg;
    let tmpdir = if cfg.get_bool("mergetool.writeToTemp") == Some(true) {
        Some(proc::temp_dir("git-mergetool-")?)
    } else {
        None
    };
    let base_n = temp_name(path, "BASE", tmpdir.as_deref());
    let local_n = temp_name(path, "LOCAL", tmpdir.as_deref());
    let remote_n = temp_name(path, "REMOTE", tmpdir.as_deref());
    let backup_n = temp_name(path, "BACKUP", tmpdir.as_deref());
    let at = |n: &str| top.join(n);
    let merged_abs = top.join(path);
    let original = fs::read(&merged_abs).unwrap_or_default();
    let mut local_b = blob(ctx.repo, st.local.as_ref())?;
    let mut remote_b = blob(ctx.repo, st.remote.as_ref())?;
    let base_b = blob(ctx.repo, st.base.as_ref())?;
    if bool_cfg(ctx, tool, "hideResolved", false) {
        let labels = ["", "", ""];
        local_b = merge_three(
            &base_b,
            &local_b,
            &remote_b,
            labels,
            MergeStyle::Merge,
            MergeFavor::Ours,
            7,
        )
        .0;
        remote_b = merge_three(
            &base_b,
            &local_b,
            &remote_b,
            labels,
            MergeStyle::Merge,
            MergeFavor::Theirs,
            7,
        )
        .0;
    }
    fs::write(at(&backup_n), &original)?;
    fs::write(at(&local_n), &local_b)?;
    fs::write(at(&remote_n), &remote_b)?;
    fs::write(at(&base_n), &base_b)?;
    let script = ctx.tc.command(tool, st.base.is_some())?;
    let files = Files {
        local: &local_n,
        remote: &remote_n,
        merged: path,
        base: &base_n,
    };
    let code = launch(&script, &files, &top)?;
    let trust = ctx.tc.trust_exit_code(tool);
    let ok = if trust {
        code == 0
    } else {
        let now = fs::read(&merged_abs).unwrap_or_default();
        if now != original {
            true
        } else {
            println!("{path} seems unchanged.");
            matches!(
                proc::ask("Was the merge successful [y/n]? "),
                Some(a) if a.trim().eq_ignore_ascii_case("y")
            )
        }
    };
    if ok {
        stage_worktree(ctx.repo, &top, path)?;
        if cfg.get_bool("mergetool.keepBackup").unwrap_or(true) {
            move_file(&at(&backup_n), &top.join(format!("{path}.orig")))?;
        } else {
            let _ = fs::remove_file(at(&backup_n));
        }
    } else {
        // Put the conflicted content back.
        let _ = move_file(&at(&backup_n), &merged_abs);
    }
    let keep_temps = !ok && cfg.get_bool("mergetool.keepTemporaries") == Some(true);
    if !keep_temps {
        for n in [&base_n, &local_n, &remote_n] {
            let _ = fs::remove_file(at(n));
        }
        if let Some(t) = &tmpdir {
            let _ = fs::remove_dir_all(t);
        }
    }
    if !ok {
        eprintln!("merge of {path} failed");
        return Ok(false);
    }
    Ok(true)
}

/// `(m)odified or (d)eleted`: keep the surviving side or delete the path.
fn resolve_deleted(ctx: &mut Ctx<'_>, path: &str, st: &Stages) -> CliResult<bool> {
    loop {
        let Some(a) = proc::ask("Use (m)odified or (d)eleted file, or (a)bort? ") else {
            return Ok(false);
        };
        match a.trim() {
            "m" | "M" => {
                let side = st.local.as_ref().or(st.remote.as_ref());
                if let Some(e) = side {
                    let abs = ctx.top.join(path);
                    crate::cmd::index_plumbing::write_worktree_file(
                        ctx.repo, &abs, &e.oid, e.mode,
                    )?;
                }
                stage_worktree(ctx.repo, &ctx.top.clone(), path)?;
                return Ok(true);
            }
            "d" | "D" => {
                let _ = fs::remove_file(ctx.top.join(path));
                unstage(ctx.repo, path)?;
                return Ok(true);
            }
            "a" | "A" => return Ok(false),
            _ => {}
        }
    }
}

/// `(l)ocal or (r)emote` for symbolic links and submodules.
fn resolve_sides(ctx: &mut Ctx<'_>, path: &str, st: &Stages) -> CliResult<bool> {
    loop {
        let Some(a) = proc::ask("Use (l)ocal or (r)emote, or (a)bort? ") else {
            return Ok(false);
        };
        let pick = match a.trim() {
            "l" | "L" => st.local.as_ref(),
            "r" | "R" => st.remote.as_ref(),
            "a" | "A" => return Ok(false),
            _ => continue,
        };
        let Some(e) = pick else { return Ok(false) };
        if e.mode != 0o160000 {
            let abs = ctx.top.join(path);
            crate::cmd::index_plumbing::write_worktree_file(ctx.repo, &abs, &e.oid, e.mode)?;
        }
        let mut idx = read_index(ctx.repo)?;
        idx.entries.retain(|x| x.path != path);
        let mut ne = e.clone();
        ne.flags &= !0x3000;
        idx.entries.push(ne);
        arks_index::sort_entries(&mut idx.entries);
        write_index(ctx.repo, &idx)?;
        return Ok(true);
    }
}

/// `git add <path>`: the work-tree file becomes the stage-0 entry.
fn stage_worktree(repo: &mut Repository, top: &Path, path: &str) -> CliResult<()> {
    let abs = top.join(path);
    let meta = fs::symlink_metadata(&abs)?;
    let bytes = if meta.file_type().is_symlink() {
        fs::read_link(&abs)?
            .to_string_lossy()
            .into_owned()
            .into_bytes()
    } else {
        fs::read(&abs)?
    };
    let format = repo.object_format();
    let oid = repo
        .store_mut()
        .put(format, &RawObject::new(ObjectKind::Blob, bytes))?;
    let mut e = crate::cmd::working_tree::stat_entry(path, oid, &meta);
    if meta.file_type().is_symlink() {
        e.mode = 0o120000;
    }
    let mut idx = read_index(repo)?;
    idx.entries.retain(|x| x.path != path);
    idx.entries.push(e);
    arks_index::sort_entries(&mut idx.entries);
    write_index(repo, &idx)
}

/// `git rm --cached <path>` for every stage of the path.
fn unstage(repo: &Repository, path: &str) -> CliResult<()> {
    let mut idx = read_index(repo)?;
    idx.entries.retain(|x| x.path != path);
    write_index(repo, &idx)
}

/// Rename, falling back to copy and remove across file systems.
fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to)?;
    fs::remove_file(from)
}
