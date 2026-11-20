//! `arks backfill [--min-batch-size=<n>] [--[no-]sparse]`.
//!
//! Cite: <https://git-scm.com/docs/git-backfill> (git 2.49; pin 2.39.5 has
//! no such command, so arks follows the later documented contract): in a
//! partial clone, download the blobs that are missing for paths of commits
//! reachable from `HEAD`, in batches of at least `--min-batch-size` objects
//! (default 50000) from the promisor remote. `--sparse` (the default when
//! sparse-checkout is enabled) limits the paths to the sparse-checkout
//! definition. A repository without a promisor remote has nothing to fetch.

use crate::cmd::opt_parse::{Mode, Takes, o, parse};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{ObjectKind, Tree, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashSet;

const DEFAULT_BATCH: usize = 50_000;

pub fn cmd_backfill(args: &[String]) -> CliResult<()> {
    let defs = [
        o("min-batch-size", ' ', Takes::Req, false),
        o("sparse", ' ', Takes::No, true),
    ];
    let p = parse("backfill", args, &defs, Mode::default())?;
    if !p.args().is_empty() {
        return Err(CliError::Exit(
            129,
            "usage: git backfill [--min-batch-size=<n>] [--[no-]sparse]".into(),
        ));
    }
    let batch = match p.value("min-batch-size") {
        Some(v) => v
            .parse::<usize>()
            .ok()
            .filter(|n| *n > 0)
            .ok_or_else(|| CliError::fatal(format!("invalid --min-batch-size value: {v}")))?,
        None => DEFAULT_BATCH,
    };
    let mut repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let sparse = p
        .flag("sparse")
        .unwrap_or_else(|| cfg.get("core.sparseCheckout").as_deref() == Some("true"));
    let Some(url) = promisor_url(&cfg) else {
        return Ok(());
    };
    let cone = if sparse { sparse_dirs(&repo) } else { None };
    let head = repo.head_oid()?;
    let commits = arks_repo::ancestor_set(&repo, head)?;
    // Each round can reveal more (missing trees hide their entries).
    for _ in 0..64 {
        let mut missing = Vec::new();
        let mut seen = HashSet::new();
        for c in &commits {
            let tree = crate::cmd::log_format::read_commit(&repo, c)?.tree;
            collect_missing(&repo, &tree, "", cone.as_deref(), &mut seen, &mut missing)?;
        }
        if missing.is_empty() {
            return Ok(());
        }
        for chunk in missing.chunks(batch) {
            let opts = arks_remote::FetchOptions {
                want_oids: Some(chunk.to_vec()),
                mark_promisor: true,
                ..Default::default()
            };
            arks_remote::fetch_remote_with(&mut repo, &url, "origin", &opts)
                .map_err(|e| CliError::fatal(e.to_string()))?;
        }
    }
    Err(CliError::fatal("backfill did not converge"))
}

/// URL of the promisor remote (`extensions.partialClone`, else the first
/// remote with `remote.<name>.promisor = true`).
fn promisor_url(cfg: &arks_config::ConfigSet) -> Option<String> {
    let name = cfg.get("extensions.partialClone").or_else(|| {
        cfg.entries()
            .iter()
            .filter(|e| {
                let k = e.key.to_ascii_lowercase();
                k.starts_with("remote.")
                    && k.ends_with(".promisor")
                    && e.value
                        .as_deref()
                        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
            })
            .map(|e| e.key["remote.".len()..e.key.len() - ".promisor".len()].to_string())
            .next()
    })?;
    cfg.get(&format!("remote.{name}.url"))
}

/// Directories of a cone-mode sparse-checkout definition (`None` = all paths).
fn sparse_dirs(repo: &Repository) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(repo.git_dir().join("info/sparse-checkout")).ok()?;
    let dirs: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('/') && l.ends_with('/') && !l.starts_with("!"))
        .map(|l| l.trim_matches('/').to_string())
        .filter(|l| !l.is_empty())
        .collect();
    Some(dirs)
}

fn wanted(path: &str, cone: Option<&[String]>) -> bool {
    match cone {
        None => true,
        // Top-level files are always in a cone; otherwise under a listed directory.
        Some(dirs) => {
            !path.contains('/')
                || dirs.iter().any(|d| {
                    path.starts_with(&format!("{d}/"))
                        || d.starts_with(&format!("{path}/"))
                        || d == path
                })
        }
    }
}

fn collect_missing(
    repo: &Repository,
    tree: &Oid,
    prefix: &str,
    cone: Option<&[String]>,
    seen: &mut HashSet<Oid>,
    missing: &mut Vec<Oid>,
) -> CliResult<()> {
    if !seen.insert(*tree) {
        return Ok(());
    }
    if !repo.store().exists(tree)? {
        missing.push(*tree);
        return Ok(());
    }
    let obj = repo.get_object(tree)?;
    if obj.kind != ObjectKind::Tree {
        return Ok(());
    }
    for e in Tree::decode(&obj.payload, repo.oid_kind())?.entries {
        let path = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        if !wanted(&path, cone) {
            continue;
        }
        match e.mode {
            TreeMode::Tree => collect_missing(repo, &e.oid, &path, cone, seen, missing)?,
            TreeMode::Gitlink => {}
            _ => {
                if seen.insert(e.oid) && !repo.store().exists(&e.oid)? {
                    missing.push(e.oid);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::wanted;

    #[test]
    fn cone_paths() {
        let cone = vec!["src".to_string()];
        assert!(wanted("README", Some(&cone)));
        assert!(wanted("src/a.rs", Some(&cone)));
        assert!(!wanted("docs/x.md", Some(&cone)));
        assert!(wanted("docs/x.md", None));
    }
}
