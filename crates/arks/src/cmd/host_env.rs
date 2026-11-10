//! Host cloud-sync / placeholder honesty (FS-007 / WIN-003 advisory).
//!
//! Cite: research 20261110222000; evidence 20261110221919.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags, work_tree};
use std::env;
use std::path::PathBuf;

/// `arks host-env [path]` — print advisory cloud-sync / CfAPI detections.
pub fn cmd_host_env(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let path = if let Some(p) = args.first() {
        PathBuf::from(p)
    } else if let Ok(repo) = open_repo() {
        work_tree(&repo)
            .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    } else {
        env::current_dir().map_err(|e| CliError::Msg(e.to_string()))?
    };

    let report = arks_hostfs::probe_repo_path(&path);
    println!("path: {}", report.path.display());
    println!("claimed: false");
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
    for note in report.notes() {
        eprintln!("{note}");
    }
    if !report.is_advisory_hazard() {
        eprintln!("note: no cloud-sync / CfAPI hazard detected at this path (advisory probe only)");
    }
    eprintln!(
        "note: FS-007 full multi-device sync safety remains Impossible/unclaimed; WIN-003 detect ≠ hydration product"
    );
    Ok(())
}

/// Emit advisory notes for a path (used by status/lock); never fails the caller.
pub fn emit_cloud_advisory(path: &std::path::Path) {
    let report = arks_hostfs::probe_repo_path(path);
    for note in report.notes() {
        eprintln!("{note}");
    }
}
