//! `arks refs` — `list`, `verify [--strict]`, and `migrate --ref-format=reftable|files` (E5).
//!
//! Primary reference: Git `Documentation/git-refs.txt` and `Documentation/technical/reftable.txt`.
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.6 (E5).

use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_refs::RefTarget;
use arks_refs::reftable::{
    RefValue, ReftableStack, detect_oid_kind, is_reftable_dir, write_reftable_stubs,
};
use std::fs;
use std::path::Path;

const REFS_USAGE: &str = "\
usage: arks refs list
   or: arks refs verify [--strict]
   or: arks refs migrate --ref-format=<reftable|files> [--no-reflog] [--dry-run]
";

pub fn cmd_refs(args: &[String]) -> CliResult<()> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{REFS_USAGE}");
        return Ok(());
    }
    let sub = args[0].as_str();
    let rest = &args[1..];
    match sub {
        "list" => crate::cmd::cmd_show_ref(rest),
        "verify" => cmd_refs_verify(rest),
        "migrate" => cmd_refs_migrate(rest),
        other => Err(CliError::Usage(format!(
            "refs: unknown subcommand {other:?} (expected list|verify|migrate)"
        ))),
    }
}

fn cmd_refs_verify(args: &[String]) -> CliResult<()> {
    let mut strict = false;
    for a in args {
        match a.as_str() {
            "--strict" => strict = true,
            other => {
                return Err(CliError::Usage(format!(
                    "refs verify: unknown option {other:?}"
                )));
            }
        }
    }
    let repo = open_repo()?;
    let common = repo.common_dir();
    if is_reftable_dir(common) {
        let stack = repo.refs().reftable_stack();
        let report = stack.verify()?;
        if strict {
            // Also verify backward-compat stubs exist per Git reftable spec.
            let head_text = fs::read_to_string(common.join("HEAD")).unwrap_or_default();
            if head_text.trim() != "ref: refs/heads/.invalid" {
                return Err(CliError::Msg(
                    "refs verify --strict: missing reftable HEAD stub (ref: refs/heads/.invalid)"
                        .into(),
                ));
            }
        }
        println!(
            "ok\tformat=reftable\ttables={}\trefs={}\tmax_update_index={}",
            report.table_count, report.total_refs, report.max_update_index
        );
        return Ok(());
    }

    // Files backend verification.
    let packed = common.join("packed-refs");
    let heads = common.join("refs/heads");
    if strict {
        let _ = repo.refs().read("HEAD")?;
        let _ = repo.refs().list_refs("")?;
    }
    println!(
        "ok\tformat=files\tpacked-refs={}\trefs/heads={}",
        packed.is_file(),
        heads.is_dir()
    );
    Ok(())
}

fn cmd_refs_migrate(args: &[String]) -> CliResult<()> {
    let mut target_format: Option<String> = None;
    let mut dry_run = false;
    let mut _no_reflog = false;
    let mut i = 0usize;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--ref-format" => {
                let v = args.get(i + 1).ok_or_else(|| {
                    CliError::Usage("refs migrate: --ref-format requires a value".into())
                })?;
                target_format = Some(v.clone());
                i += 2;
            }
            _ if a.starts_with("--ref-format=") => {
                target_format = Some(a["--ref-format=".len()..].to_string());
                i += 1;
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            "--no-reflog" => {
                _no_reflog = true;
                i += 1;
            }
            other => {
                return Err(CliError::Usage(format!(
                    "refs migrate: unknown option {other:?}"
                )));
            }
        }
    }
    let fmt = target_format.as_deref().ok_or_else(|| {
        CliError::Usage("refs migrate: --ref-format=<reftable|files> is required".into())
    })?;
    match fmt {
        "reftable" => migrate_to_reftable(dry_run),
        "files" => migrate_to_files(dry_run),
        other => Err(CliError::Usage(format!(
            "refs migrate: unsupported ref format {other:?} (expected reftable|files)"
        ))),
    }
}

fn migrate_to_reftable(dry_run: bool) -> CliResult<()> {
    let repo = open_repo()?;
    let common = repo.common_dir().to_path_buf();
    if is_reftable_dir(&common) {
        return Err(CliError::Msg(
            "refs migrate: repository already uses the reftable format".into(),
        ));
    }

    // Collect HEAD (preserving symbolic target if symbolic) + all direct refs under refs/.
    let mut updates: Vec<(String, RefValue)> = Vec::new();
    if let Ok(head) = repo.refs().read("HEAD") {
        let val = match head {
            RefTarget::Direct(oid) => RefValue::Direct(oid),
            RefTarget::Symbolic(sym) => RefValue::Symbolic(sym),
        };
        updates.push(("HEAD".into(), val));
    }
    for (name, oid) in repo.refs().list_refs("refs/")? {
        // Preserve symbolic refs if any exist under refs/.
        let val = match repo.refs().read(&name) {
            Ok(RefTarget::Symbolic(sym)) => RefValue::Symbolic(sym),
            _ => RefValue::Direct(oid),
        };
        updates.push((name, val));
    }

    if dry_run {
        println!(
            "dry-run: would migrate {} refs from files to reftable",
            updates.len()
        );
        return Ok(());
    }

    let oid_kind = detect_oid_kind(&common);
    let stack = ReftableStack::open(common.join("reftable"), oid_kind);
    stack.init()?;
    let count = updates.len();
    if !updates.is_empty() {
        stack.append_transaction(updates, vec![])?;
    }
    write_reftable_stubs(&common)?;
    set_repo_ref_storage_config(&common, "reftable")?;
    println!("migrated {count} refs from files to reftable");
    Ok(())
}

fn migrate_to_files(dry_run: bool) -> CliResult<()> {
    let repo = open_repo()?;
    let common = repo.common_dir().to_path_buf();
    if !is_reftable_dir(&common) {
        return Err(CliError::Msg(
            "refs migrate: repository already uses the files format".into(),
        ));
    }

    let stack = repo.refs().reftable_stack();
    let merged = stack.merged_refs()?;
    if dry_run {
        println!(
            "dry-run: would migrate {} refs from reftable to files",
            merged.len()
        );
        return Ok(());
    }

    // Replace refs/heads sentinel file with standard directories.
    let refs_dir = common.join("refs");
    let heads_stub = refs_dir.join("heads");
    if heads_stub.is_file() {
        fs::remove_file(&heads_stub)?;
    }
    fs::create_dir_all(refs_dir.join("heads"))?;
    fs::create_dir_all(refs_dir.join("tags"))?;

    let count = merged.len();
    for (name, val) in &merged {
        let target_path = common.join(name);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        match val {
            RefValue::Direct(oid) | RefValue::Peeled { value: oid, .. } => {
                fs::write(&target_path, format!("{}\n", oid.to_hex()))?;
            }
            RefValue::Symbolic(sym) => {
                fs::write(&target_path, format!("ref: {sym}\n"))?;
            }
            RefValue::Deletion => {}
        }
    }

    // Remove reftable directory and update config.
    let reftable_dir = common.join("reftable");
    if reftable_dir.is_dir() {
        fs::remove_dir_all(&reftable_dir)?;
    }
    set_repo_ref_storage_config(&common, "files")?;
    println!("migrated {count} refs from reftable to files");
    Ok(())
}

/// Update `core.repositoryformatversion = 1` and `extensions.refStorage = <fmt>` in `<git_dir>/config`.
fn set_repo_ref_storage_config(common_dir: &Path, ref_storage: &str) -> CliResult<()> {
    let cfg_path = common_dir.join("config");
    let text = fs::read_to_string(&cfg_path).unwrap_or_default();
    let mut out = String::new();
    let mut section = String::new();
    let mut wrote_fmt_ver = false;
    let mut wrote_ref_storage = false;
    let mut has_extensions = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if section.eq_ignore_ascii_case("core") && !wrote_fmt_ver {
                out.push_str("\trepositoryformatversion = 1\n");
                wrote_fmt_ver = true;
            }
            if section.eq_ignore_ascii_case("extensions") && !wrote_ref_storage {
                out.push_str(&format!("\trefStorage = {ref_storage}\n"));
                wrote_ref_storage = true;
            }
            section = trimmed[1..trimmed.len() - 1].trim().to_string();
            if section.eq_ignore_ascii_case("extensions") {
                has_extensions = true;
            }
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if let Some((k, _)) = trimmed.split_once('=') {
            let key = k.trim();
            if section.eq_ignore_ascii_case("core")
                && key.eq_ignore_ascii_case("repositoryformatversion")
            {
                out.push_str("\trepositoryformatversion = 1\n");
                wrote_fmt_ver = true;
                continue;
            }
            if section.eq_ignore_ascii_case("extensions") && key.eq_ignore_ascii_case("refstorage")
            {
                out.push_str(&format!("\trefStorage = {ref_storage}\n"));
                wrote_ref_storage = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }

    if section.eq_ignore_ascii_case("core") && !wrote_fmt_ver {
        out.push_str("\trepositoryformatversion = 1\n");
        wrote_fmt_ver = true;
    }
    if section.eq_ignore_ascii_case("extensions") && !wrote_ref_storage {
        out.push_str(&format!("\trefStorage = {ref_storage}\n"));
        wrote_ref_storage = true;
    }
    if !wrote_fmt_ver {
        out.push_str("[core]\n\trepositoryformatversion = 1\n");
    }
    if !has_extensions || !wrote_ref_storage {
        out.push_str(&format!("[extensions]\n\trefStorage = {ref_storage}\n"));
    }
    fs::write(&cfg_path, out)?;
    Ok(())
}
