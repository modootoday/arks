//! `arks replay` — worktree-free history replay (E3).
//!
//! Takes a range of commits and replays them onto a new base purely in the
//! object store (using in-memory 3-way tree merges), leaving the working tree
//! and `.git/index` completely untouched.
//!
//! Primary source: <https://git-scm.com/docs/git-replay>
//! Spec: `sot:root/v0-2-0-scope-expansion` (E3), `research:root/20261128092500` §3.

use crate::cmd::merge_engine::merge_trees_in_memory;
use crate::cmd::shared::{committer_signature, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, RawObject};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::Write;

const USAGE: &str = "\
usage: arks replay (--onto <newbase> | --advance <branch>) [--contained] [--ref-action=(update|print)] <revision-range>...

Replays commits onto a new base without touching the working tree or index.

OPTIONS:
  --onto <newbase>              Starting point at which to create the new commits
  --advance <branch>            Advance <branch> to the tip of the replayed commits
  --contained                   Update all branches contained in the revision range (with --onto)
  --ref-action=<update|print>   Apply ref updates atomically (default: update) or print update-ref --stdin commands
  -h, --help                    Show this help message
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefAction {
    Update,
    Print,
}

#[derive(Debug, Clone)]
enum Mode {
    Onto(String),
    Advance(String),
}

#[derive(Debug, Clone)]
struct ReplayOpts {
    mode: Mode,
    contained: bool,
    ref_action: RefAction,
    revs: Vec<String>,
}

fn parse_args(args: &[String]) -> CliResult<Option<ReplayOpts>> {
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return Ok(None);
    }

    let mut onto: Option<String> = None;
    let mut advance: Option<String> = None;
    let mut contained = false;
    let mut ref_action = RefAction::Update;
    let mut revs = Vec::new();

    let mut i = 0usize;
    let mut after_dd = false;
    while i < args.len() {
        let a = &args[i];
        if !after_dd && a == "--" {
            after_dd = true;
            i += 1;
            continue;
        }
        if !after_dd {
            if a == "--onto" {
                let val = args
                    .get(i + 1)
                    .ok_or_else(|| CliError::Usage("replay: --onto requires a value".into()))?;
                onto = Some(val.clone());
                i += 2;
                continue;
            }
            if let Some(val) = a.strip_prefix("--onto=") {
                onto = Some(val.to_string());
                i += 1;
                continue;
            }
            if a == "--advance" {
                let val = args
                    .get(i + 1)
                    .ok_or_else(|| CliError::Usage("replay: --advance requires a value".into()))?;
                advance = Some(val.clone());
                i += 2;
                continue;
            }
            if let Some(val) = a.strip_prefix("--advance=") {
                advance = Some(val.to_string());
                i += 1;
                continue;
            }
            if a == "--contained" {
                contained = true;
                i += 1;
                continue;
            }
            if a == "--ref-action" {
                let val = args.get(i + 1).ok_or_else(|| {
                    CliError::Usage("replay: --ref-action requires update or print".into())
                })?;
                ref_action = parse_ref_action(val)?;
                i += 2;
                continue;
            }
            if let Some(val) = a.strip_prefix("--ref-action=") {
                ref_action = parse_ref_action(val)?;
                i += 1;
                continue;
            }
            if a.starts_with('-') && a != "-" {
                return Err(CliError::Usage(format!("replay: unknown option {a:?}")));
            }
        }
        revs.push(a.clone());
        i += 1;
    }

    let mode = match (onto, advance) {
        (Some(_), Some(_)) => {
            return Err(CliError::Usage(
                "replay: --onto and --advance are mutually exclusive".into(),
            ));
        }
        (None, None) => {
            return Err(CliError::Usage(
                "replay: exactly one of --onto <newbase> or --advance <branch> is required".into(),
            ));
        }
        (Some(o), None) => Mode::Onto(o),
        (None, Some(b)) => {
            if contained {
                return Err(CliError::Usage(
                    "replay: --contained requires --onto (cannot combine with --advance)".into(),
                ));
            }
            Mode::Advance(b)
        }
    };

    if revs.is_empty() {
        return Err(CliError::Usage(
            "replay: at least one revision range is required".into(),
        ));
    }

    Ok(Some(ReplayOpts {
        mode,
        contained,
        ref_action,
        revs,
    }))
}

fn parse_ref_action(val: &str) -> CliResult<RefAction> {
    match val {
        "update" => Ok(RefAction::Update),
        "print" => Ok(RefAction::Print),
        other => Err(CliError::Usage(format!(
            "replay: invalid --ref-action {other:?} (expected 'update' or 'print')"
        ))),
    }
}

/// Entry point for `arks replay`.
pub fn cmd_git_replay(args: &[String]) -> CliResult<()> {
    let Some(opts) = parse_args(args)? else {
        return Ok(());
    };
    let mut repo = open_repo()?;
    let mut out = std::io::stdout();
    run_replay(&mut repo, &opts, &mut out)
}

fn run_replay(repo: &mut Repository, opts: &ReplayOpts, out: &mut dyn Write) -> CliResult<()> {
    let (onto_oid, advance_ref) = match &opts.mode {
        Mode::Onto(rev) => (resolve_rev(repo, rev)?, None),
        Mode::Advance(branch) => {
            let full = resolve_branch_ref(repo, branch)?;
            let oid = repo.refs().resolve(&full)?;
            (oid, Some(full))
        }
    };
    let _ = load_commit(repo, &onto_oid)?;

    // Parse revision ranges into negative (excluded) and positive (included) commit OIDs,
    // recording which positive specs refer to local branches for `--onto` ref updates.
    let mut positives: Vec<Oid> = Vec::new();
    let mut negatives: Vec<Oid> = Vec::new();
    let mut positive_refs: Vec<(String, Oid)> = Vec::new();

    for spec in &opts.revs {
        if let Some((left, right)) = spec.split_once("..") {
            let l_rev = if left.is_empty() { "HEAD" } else { left };
            let r_rev = if right.is_empty() { "HEAD" } else { right };
            negatives.push(resolve_rev(repo, l_rev)?);
            let r_oid = resolve_rev(repo, r_rev)?;
            positives.push(r_oid);
            if let Some(full) = try_branch_ref(repo, r_rev) {
                positive_refs.push((full, r_oid));
            }
        } else if let Some(neg) = spec.strip_prefix('^') {
            negatives.push(resolve_rev(repo, neg)?);
        } else {
            let oid = resolve_rev(repo, spec)?;
            positives.push(oid);
            if let Some(full) = try_branch_ref(repo, spec) {
                positive_refs.push((full, oid));
            }
        }
    }

    let commits = collect_commits_topo(repo, &positives, &negatives)?;
    if commits.is_empty() {
        return Ok(());
    }

    let empty_tree = {
        let format = repo.object_format();
        repo.store_mut()
            .put(format, &RawObject::new(ObjectKind::Tree, Vec::new()))?
    };

    let committer = committer_signature()?;
    let mut commit_map: HashMap<Oid, Oid> = HashMap::new();
    let mut last_oid = onto_oid;

    for orig_oid in &commits {
        let orig = load_commit(repo, orig_oid)?;
        if orig.parents.len() > 1 {
            return Err(CliError::fatal(format!(
                "replay: merge commit {} is not supported in linear replay",
                orig_oid.to_hex()
            )));
        }
        let parent_oid = orig.parents.first().copied();
        let base_tree = match parent_oid {
            Some(p) => load_commit(repo, &p)?.tree,
            None => empty_tree,
        };
        let ours_commit = parent_oid
            .and_then(|p| commit_map.get(&p).copied())
            .unwrap_or(onto_oid);
        let ours_tree = load_commit(repo, &ours_commit)?.tree;

        let merged = merge_trees_in_memory(repo, &base_tree, &ours_tree, &orig.tree)?;
        if !merged.conflicts.is_empty() || merged.tree.is_none() {
            for path in &merged.conflicts {
                eprintln!("CONFLICT (content): Merge conflict in {path}");
            }
            return Err(CliError::Exit(
                1,
                format!(
                    "replay: merge conflict while replaying {} (no refs, index, or worktree modified)",
                    orig_oid.to_hex()
                ),
            ));
        }
        let new_tree = merged.tree.unwrap();

        // Drop commits that become empty when the original commit was non-empty
        // (matching Git 2.54 git-replay semantics).
        if new_tree == ours_tree && orig.tree != base_tree {
            commit_map.insert(*orig_oid, ours_commit);
            last_oid = ours_commit;
            continue;
        }

        let new_commit = Commit {
            tree: new_tree,
            parents: vec![ours_commit],
            author: orig.author.clone(),
            committer: committer.clone(),
            gpgsig: None,
            message: orig.message.clone(),
        };
        let payload = new_commit.encode()?;
        let format = repo.object_format();
        let new_oid = repo
            .store_mut()
            .put(format, &RawObject::new(ObjectKind::Commit, payload))?;
        commit_map.insert(*orig_oid, new_oid);
        last_oid = new_oid;
    }

    // Build deterministic ordered map of refname -> (new_oid, old_oid).
    let mut updates: BTreeMap<String, (Oid, Oid)> = BTreeMap::new();
    if let Some(branch_ref) = advance_ref {
        updates.insert(branch_ref, (last_oid, onto_oid));
    } else {
        for (refname, old_oid) in &positive_refs {
            if let Some(&new_oid) = commit_map.get(old_oid) {
                updates.insert(refname.clone(), (new_oid, *old_oid));
            }
        }
        if opts.contained {
            for (refname, tip_oid) in repo.refs().list_refs("refs/heads/")? {
                if let Some(&new_oid) = commit_map.get(&tip_oid) {
                    updates.entry(refname).or_insert((new_oid, tip_oid));
                }
            }
        }
    }

    match opts.ref_action {
        RefAction::Print => {
            for (refname, (new_oid, old_oid)) in &updates {
                writeln!(
                    out,
                    "update {refname} {} {}",
                    new_oid.to_hex(),
                    old_oid.to_hex()
                )
                .map_err(|e| CliError::Msg(e.to_string()))?;
            }
        }
        RefAction::Update => {
            // Pre-verify all expected old OIDs before mutating any ref (all-or-nothing check).
            for (refname, (_, old_oid)) in &updates {
                let current = repo.refs().resolve(refname)?;
                if &current != old_oid {
                    return Err(CliError::fatal(format!(
                        "replay: ref {refname} moved concurrently (expected {}, found {})",
                        old_oid.to_hex(),
                        current.to_hex()
                    )));
                }
            }
            for (refname, (new_oid, old_oid)) in &updates {
                repo.refs_mut().update(refname, new_oid, Some(old_oid))?;
            }
        }
    }

    Ok(())
}

fn load_commit(repo: &Repository, oid: &Oid) -> CliResult<Commit> {
    let obj = repo.get_object(oid)?;
    if obj.kind != ObjectKind::Commit {
        return Err(CliError::fatal(format!("{} is not a commit", oid.to_hex())));
    }
    Ok(Commit::decode(&obj.payload)?)
}

fn try_branch_ref(repo: &Repository, spec: &str) -> Option<String> {
    if spec.starts_with("refs/heads/") && repo.refs().resolve(spec).is_ok() {
        return Some(spec.to_string());
    }
    let full = format!("refs/heads/{spec}");
    if repo.refs().resolve(&full).is_ok() {
        return Some(full);
    }
    None
}

fn resolve_branch_ref(repo: &Repository, branch: &str) -> CliResult<String> {
    try_branch_ref(repo, branch).ok_or_else(|| {
        CliError::fatal(format!(
            "replay: --advance target {branch:?} is not an existing local branch"
        ))
    })
}

/// Collect commits reachable from `positives` and not reachable from `negatives`,
/// returning them in oldest-first topological order (parents before children).
fn collect_commits_topo(
    repo: &Repository,
    positives: &[Oid],
    negatives: &[Oid],
) -> CliResult<Vec<Oid>> {
    // 1. Mark all commits reachable from negatives as excluded.
    let mut excluded: HashSet<Oid> = HashSet::new();
    let mut queue: VecDeque<Oid> = negatives.iter().copied().collect();
    while let Some(oid) = queue.pop_front() {
        if !excluded.insert(oid) {
            continue;
        }
        let c = load_commit(repo, &oid)?;
        for p in c.parents {
            if !excluded.contains(&p) {
                queue.push_back(p);
            }
        }
    }

    // 2. Collect all commits reachable from positives that are not excluded.
    let mut included: HashMap<Oid, Vec<Oid>> = HashMap::new();
    let mut pqueue: VecDeque<Oid> = positives.iter().copied().collect();
    while let Some(oid) = pqueue.pop_front() {
        if excluded.contains(&oid) || included.contains_key(&oid) {
            continue;
        }
        let c = load_commit(repo, &oid)?;
        let parents: Vec<Oid> = c
            .parents
            .into_iter()
            .filter(|p| !excluded.contains(p))
            .collect();
        for &p in &parents {
            pqueue.push_back(p);
        }
        included.insert(oid, parents);
    }

    // 3. Kahn's topological sort (parents before children).
    let mut in_degree: HashMap<Oid, usize> = HashMap::new();
    let mut children: HashMap<Oid, Vec<Oid>> = HashMap::new();
    for (&oid, parents) in &included {
        in_degree.insert(oid, parents.len());
        for &p in parents {
            children.entry(p).or_default().push(oid);
        }
    }
    let mut ready: VecDeque<Oid> = in_degree
        .iter()
        .filter_map(|(&oid, &deg)| (deg == 0).then_some(oid))
        .collect();
    let mut order = Vec::with_capacity(included.len());
    while let Some(oid) = ready.pop_front() {
        order.push(oid);
        if let Some(kids) = children.get(&oid) {
            for &kid in kids {
                if let Some(d) = in_degree.get_mut(&kid) {
                    *d -= 1;
                    if *d == 0 {
                        ready.push_back(kid);
                    }
                }
            }
        }
    }
    Ok(order)
}
