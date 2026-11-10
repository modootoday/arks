//! Host cloud-sync / placeholder / LSM honesty (FS-007 / WIN-003 / FS-014).
//!
//! Cite: research 20261110222000; 20261111001816; evidence 20261110221919.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags, work_tree};
use std::env;
use std::path::PathBuf;

/// `arks host-env [path] [--refuse-sync]` — advisory detect; optional refuse.
pub fn cmd_host_env(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--refuse-sync"])?;
    let refuse_flag = args.iter().any(|a| a == "--refuse-sync");
    let refuse_requested = refuse_flag || arks_hostfs::refuse_sync_env_requested();

    let path = if let Some(p) = args.iter().find(|a| !a.starts_with('-')) {
        PathBuf::from(p)
    } else if let Ok(repo) = open_repo() {
        work_tree(&repo)
            .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    } else {
        env::current_dir().map_err(|e| CliError::Msg(e.to_string()))?
    };

    let report = arks_hostfs::probe_repo_path(&path);
    println!("path: {}", report.path.display());
    println!("claimed.full_sync: false");
    println!("claimed.hydration: false");
    println!("claimed.lsm_bypass: false");
    println!(
        "claimed.detect_refuse: {}",
        if refuse_requested { "opt-in" } else { "false" }
    );
    println!(
        "hazard: {}",
        if report.is_advisory_hazard() {
            "advisory"
        } else {
            "none"
        }
    );
    if let Some(c) = &report.cloud {
        println!("cloud.kind: {}", c.kind.as_str());
        println!("cloud.catalog: {}", c.kind.catalog_id());
        println!("cloud.root: {}", c.root.display());
        println!("cloud.detail: {}", c.detail);
        println!("cloud.claimed: {}", c.claimed);
    } else {
        println!("cloud.kind: (none)");
    }
    println!("onedrive.placeholder: {}", report.onedrive_placeholder);
    if report.lsm.is_empty() {
        println!("lsm.kind: (none)");
    } else {
        let kinds: Vec<&str> = report.lsm.iter().map(|l| l.as_str()).collect();
        println!("lsm.kind: {}", kinds.join(","));
        println!("lsm.catalog: FS-014");
    }
    println!("refuse.sync.requested: {refuse_requested}");
    println!(
        "refuse.sync.trigger: {}",
        report.should_refuse_sync(refuse_requested)
    );
    if let Some(hint) = report.errno_honesty_hint() {
        println!("errno.honesty: {hint}");
    }
    for note in report.notes() {
        eprintln!("{note}");
    }
    if !report.is_advisory_hazard() && report.lsm.is_empty() {
        eprintln!(
            "note: no cloud-sync / CfAPI / LSM hazard detected at this path (advisory probe only)"
        );
    }
    eprintln!(
        "note: FS-007 full multi-device sync safety remains Impossible/unclaimed; WIN-003 detect ≠ hydration; FS-014 detect ≠ LSM bypass"
    );
    eprintln!(
        "note: opt-in refuse: `arks host-env --refuse-sync` or ARKS_HOST_ENV_REFUSE_SYNC=1 when .git under sync root (Claimed detect+refuse slice only)"
    );

    if report.should_refuse_sync(refuse_requested) {
        let kind = report
            .cloud
            .as_ref()
            .map(|c| c.kind.as_str())
            .unwrap_or("cloud-sync");
        return Err(CliError::Msg(format!(
            "FS-007 refuse-sync: refusing writers under detected {kind} root at {} (advisory detect+refuse; not Claimed multi-device safety)",
            report.path.display()
        )));
    }
    Ok(())
}

/// Emit advisory notes for a path (used by status/lock); never fails the caller.
pub fn emit_cloud_advisory(path: &std::path::Path) {
    let report = arks_hostfs::probe_repo_path(path);
    for note in report.notes() {
        eprintln!("{note}");
    }
}

/// Opt-in hard refuse for writers when sync root detected (callers: status/add).
pub fn refuse_if_sync_writers(path: &std::path::Path) -> CliResult<()> {
    if !arks_hostfs::refuse_sync_env_requested() {
        return Ok(());
    }
    let report = arks_hostfs::probe_repo_path(path);
    if report.should_refuse_sync(true) {
        let kind = report
            .cloud
            .as_ref()
            .map(|c| c.kind.as_str())
            .unwrap_or("cloud-sync");
        return Err(CliError::Msg(format!(
            "FS-007 refuse-sync: ARKS_HOST_ENV_REFUSE_SYNC set and {kind} detected at {}",
            path.display()
        )));
    }
    Ok(())
}
