//! `commit-graph` — write and verify commit-graph files.
//!
//! `write` collects commits (`--reachable`, `--stdin-commits`,
//! `--stdin-packs`, default: every pack), closes the set under parents and
//! writes `objects/info/commit-graph`, or a `--split` layer under
//! `objects/info/commit-graphs/` with its `commit-graph-chain`. Chunks:
//! OIDF, OIDL, CDAT, GDA2 (+ GDO2), EDGE, BASE and, with
//! `--changed-paths`, BIDX/BDAT Bloom filters. `verify` checks the files
//! against the object database.
//! Cite: <https://git-scm.com/docs/git-commit-graph>,
//! <https://git-scm.com/docs/gitformat-commit-graph>.

use crate::cmd::opt_parse::{Mode, OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_hash::{Hasher, ObjectFormat};
use arks_object::{Commit, ObjectKind, Tree, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

const USAGE_VERIFY: &str =
    "usage: git commit-graph verify [--object-dir <dir>] [--shallow] [--[no-]progress]";
const USAGE_WRITE: &str = "usage: git commit-graph write [--object-dir <dir>] [--append]
                              [--split[=<strategy>]] [--reachable | --stdin-packs | --stdin-commits]
                              [--changed-paths] [--[no-]max-new-filters <n>] [--[no-]progress]
                              <split options>";

fn usage_all() -> String {
    format!(
        "{USAGE_VERIFY}\n   or: {}\n\n    --object-dir <dir>    the object directory to store the graph\n",
        &USAGE_WRITE["usage: ".len()..]
    )
}

const NO_PARENT: u32 = 0x7000_0000;
const EXTRA_EDGES: u32 = 0x8000_0000;
const LAST_EDGE: u32 = 0x8000_0000;
const MAX_CHANGED_PATHS: usize = 512;
const BLOOM_HASHES: u32 = 7;
const BLOOM_BITS_PER_ENTRY: u32 = 10;

fn hash_version(f: ObjectFormat) -> u8 {
    match f {
        ObjectFormat::Sha1 => 1,
        ObjectFormat::Sha256 => 2,
    }
}

fn repo_kind(f: ObjectFormat) -> arks_oid::OidKind {
    match f {
        ObjectFormat::Sha1 => arks_oid::OidKind::Sha1,
        ObjectFormat::Sha256 => arks_oid::OidKind::Sha256,
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The object directory named by `--object-dir`, or the repository's.
fn object_dir(repo: &Repository, dir: Option<&str>) -> CliResult<PathBuf> {
    match dir {
        None => Ok(repo.common_dir().join("objects")),
        Some(d) => {
            let p = PathBuf::from(d);
            let full = if p.is_absolute() {
                p
            } else {
                std::env::current_dir()?.join(p)
            };
            if !full.is_dir() {
                return Err(CliError::fatal(format!(
                    "could not find object directory matching {d}"
                )));
            }
            Ok(full)
        }
    }
}

pub fn cmd_commit_graph(args: &[String]) -> CliResult<()> {
    // Global options come before the subcommand.
    let mut obj_dir: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-h" {
            println!("{}", usage_all());
            return Err(CliError::silent(129));
        }
        if let Some(v) = a.strip_prefix("--object-dir=") {
            obj_dir = Some(v.to_string());
        } else if a == "--object-dir" {
            i += 1;
            obj_dir = Some(args.get(i).cloned().ok_or_else(|| {
                CliError::Exit(129, "error: option `object-dir' requires a value".into())
            })?);
        } else if a == "--no-object-dir" {
            obj_dir = None;
        } else if a.starts_with('-') {
            return Err(CliError::Exit(
                129,
                format!(
                    "error: unknown option `{}'\n{}",
                    a.trim_start_matches('-'),
                    usage_all()
                ),
            ));
        } else {
            break;
        }
        i += 1;
    }
    let Some(sub) = args.get(i) else {
        return Err(CliError::Exit(
            129,
            format!("error: need a subcommand\n{}", usage_all()),
        ));
    };
    let rest = &args[i + 1..];
    match sub.as_str() {
        "verify" => verify_cmd(rest, obj_dir),
        "write" => write_cmd(rest, obj_dir),
        other => Err(CliError::Exit(
            129,
            format!("error: unknown subcommand: `{other}'\n{}", usage_all()),
        )),
    }
}

const VERIFY_DEFS: &[OptDef] = &[
    o("object-dir", ' ', Req, true),
    o("shallow", ' ', No, true),
    o("progress", ' ', No, true),
];

const WRITE_DEFS: &[OptDef] = &[
    o("object-dir", ' ', Req, true),
    o("reachable", ' ', No, true),
    o("stdin-packs", ' ', No, true),
    o("stdin-commits", ' ', No, true),
    o("append", ' ', No, true),
    o("changed-paths", ' ', No, true),
    o("max-new-filters", ' ', Req, true),
    o("progress", ' ', No, true),
    o("split", ' ', Opt, true),
    o("max-commits", ' ', Req, false),
    o("size-multiple", ' ', Req, false),
    o("expire-time", ' ', Req, false),
];

const OBJDIR_HELP: &str = "    --object-dir <dir>    the object directory to store the graph
";

const VERIFY_HELP: &str =
    "    --shallow             if the commit-graph is split, only verify the tip file
    --progress            force progress reporting
";

const WRITE_HELP: &str = "    --reachable           start walk at all refs
    --stdin-packs         scan pack-indexes listed by stdin for commits
    --stdin-commits       start walk at commits listed by stdin
    --append              include all commits already in the commit-graph file
    --changed-paths       enable computation for changed paths
    --split[=...]         allow writing an incremental commit-graph file
    --max-commits <n>     maximum number of commits in a non-base split commit-graph
    --size-multiple <n>   maximum ratio between two levels of a split commit-graph
    --expire-time <expiry-date>
                          only expire files older than a given date-time
    --max-new-filters ...
                          maximum number of changed-path Bloom filters to compute
    --progress            force progress reporting
";

/// Usage line plus the option list of a subcommand.
fn sub_usage(usage: &str) -> String {
    let help = if usage == USAGE_VERIFY {
        VERIFY_HELP
    } else {
        WRITE_HELP
    };
    format!("{usage}\n\n{OBJDIR_HELP}{help}")
}

fn sub_help(args: &[String], usage: &str) -> CliResult<()> {
    if args.len() == 1 && args[0] == "-h" {
        println!("{}", sub_usage(usage));
        return Err(CliError::silent(129));
    }
    Ok(())
}

fn verify_cmd(args: &[String], obj_dir: Option<String>) -> CliResult<()> {
    sub_help(args, USAGE_VERIFY)?;
    let p = parse("commit-graph verify", args, VERIFY_DEFS, Mode::default())?;
    if !p.args().is_empty() {
        return Err(CliError::Exit(129, sub_usage(USAGE_VERIFY)));
    }
    let repo = open_repo()?;
    let dir = object_dir(&repo, p.value("object-dir").or(obj_dir).as_deref())?;
    let layers = existing_layers(&dir);
    if layers.is_empty() {
        return Ok(());
    }
    let shallow = p.on("shallow");
    let mut errors = Vec::new();
    let mut graphs: Vec<Graph> = Vec::new();
    for (path, _) in &layers {
        match Graph::read(path, repo.object_format()) {
            Ok(g) => graphs.push(g),
            Err(e) => errors.push(e),
        }
    }
    let total: Vec<&Graph> = graphs.iter().collect();
    let start = if shallow {
        total.len().saturating_sub(1)
    } else {
        0
    };
    let mut base_pos = 0usize;
    let all_oids: Vec<Oid> = total.iter().flat_map(|g| g.oids.iter().copied()).collect();
    for (li, g) in total.iter().enumerate() {
        if li >= start {
            errors.extend(g.verify(&repo, &all_oids, base_pos));
        }
        base_pos += g.oids.len();
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CliError::Exit(
            1,
            errors
                .iter()
                .map(|e| format!("error: {e}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ))
    }
}

/// Current layers: the chain (oldest first) or the single file.
fn existing_layers(dir: &Path) -> Vec<(PathBuf, String)> {
    let chain = dir.join("info/commit-graphs/commit-graph-chain");
    if let Ok(text) = fs::read_to_string(&chain) {
        return text
            .lines()
            .filter(|l| !l.is_empty())
            .map(|h| {
                (
                    dir.join(format!("info/commit-graphs/graph-{h}.graph")),
                    h.to_string(),
                )
            })
            .collect();
    }
    let single = dir.join("info/commit-graph");
    if single.is_file() {
        return vec![(single, String::new())];
    }
    Vec::new()
}

/// A parsed commit-graph file (one layer).
struct Graph {
    oids: Vec<Oid>,
    data: Vec<u8>,
    chunks: HashMap<[u8; 4], (usize, usize)>,
    hash_len: usize,
}

impl Graph {
    fn read(path: &Path, format: ObjectFormat) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
        let hl = format.digest_len();
        if data.len() < 8 + hl || &data[..4] != b"CGPH" {
            return Err(format!(
                "commit-graph signature {:X} does not match signature {:X}",
                u32::from_be_bytes(
                    data.get(..4)
                        .and_then(|s| s.try_into().ok())
                        .unwrap_or([0; 4])
                ),
                u32::from_be_bytes(*b"CGPH")
            ));
        }
        if data[4] != 1 {
            return Err(format!(
                "commit-graph version {:X} does not match version {:X}",
                data[4], 1
            ));
        }
        if data[5] != hash_version(format) {
            return Err(format!(
                "commit-graph hash version {:X} does not match version {:X}",
                data[5],
                hash_version(format)
            ));
        }
        let n_chunks = data[6] as usize;
        let mut chunks = HashMap::new();
        let mut entries = Vec::new();
        for i in 0..=n_chunks {
            let at = 8 + i * 12;
            let id: [u8; 4] = data
                .get(at..at + 4)
                .and_then(|s| s.try_into().ok())
                .ok_or("commit-graph chunk lookup table entry missing; file may be incomplete")?;
            let off = u64::from_be_bytes(data[at + 4..at + 12].try_into().map_err(|_| "truncated")?)
                as usize;
            entries.push((id, off));
        }
        for w in entries.windows(2) {
            chunks.insert(w[0].0, (w[0].1, w[1].1));
        }
        let (os, oe) = *chunks
            .get(b"OIDL")
            .ok_or("commit-graph required OID lookup chunk missing or corrupted")?;
        let oids = data
            .get(os..oe)
            .ok_or("commit-graph improper chunk offset")?
            .chunks(hl)
            .map(|c| Oid::from_bytes(repo_kind(format), c))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            oids,
            data,
            chunks,
            hash_len: hl,
        })
    }

    fn verify(&self, repo: &Repository, all: &[Oid], base: usize) -> Vec<String> {
        let mut errs = Vec::new();
        let hl = self.hash_len;
        if let Some((fs_, fe)) = self.chunks.get(b"OIDF") {
            let fan: Vec<u32> = self.data[*fs_..*fe]
                .chunks(4)
                .map(|c| u32::from_be_bytes(c.try_into().unwrap_or([0; 4])))
                .collect();
            if fan.windows(2).any(|w| w[0] > w[1]) {
                errs.push("commit-graph fanout values out of order".into());
                return errs;
            }
        } else {
            errs.push("commit-graph required OID fanout chunk missing or corrupted".into());
        }
        let body = &self.data[..self.data.len() - hl];
        let sum = &self.data[self.data.len() - hl..];
        if arks_hash::hash_bytes(repo.object_format(), body)
            .map(|d| d.as_bytes() != sum)
            .unwrap_or(true)
        {
            errs.push("the commit-graph file has incorrect checksum and is likely corrupt".into());
        }
        for w in self.oids.windows(2) {
            if w[0].as_bytes() >= w[1].as_bytes() {
                errs.push(format!(
                    "commit-graph has incorrect OID order: {} then {}",
                    w[0].to_hex(),
                    w[1].to_hex()
                ));
            }
        }
        let Some((cs, _)) = self.chunks.get(b"CDAT").copied() else {
            errs.push("commit-graph required commit data chunk missing or corrupted".into());
            return errs;
        };
        let rec = hl + 16;
        for (i, oid) in self.oids.iter().enumerate() {
            let r = &self.data[cs + i * rec..cs + (i + 1) * rec];
            let Ok(obj) = repo.get_object(oid) else {
                errs.push(format!(
                    "failed to parse commit {} from object database for commit-graph",
                    oid.to_hex()
                ));
                continue;
            };
            let Ok(c) = Commit::decode(&obj.payload) else {
                errs.push(format!(
                    "failed to parse commit {} from object database for commit-graph",
                    oid.to_hex()
                ));
                continue;
            };
            if c.tree.as_bytes() != &r[..hl] {
                errs.push(format!(
                    "root tree OID for commit {} in commit-graph is {} != {}",
                    oid.to_hex(),
                    hex(&r[..hl]),
                    c.tree.to_hex()
                ));
            }
            let p1 = u32::from_be_bytes(r[hl..hl + 4].try_into().unwrap_or([0; 4]));
            let pos = |p: u32| all.get(p as usize).copied();
            let want1 = c.parents.first().copied();
            let got1 = (p1 != NO_PARENT).then(|| pos(p1)).flatten();
            if want1 != got1 {
                errs.push(format!(
                    "commit-graph parent list for commit {} is too long",
                    oid.to_hex()
                ));
            }
            let time = (u64::from(u32::from_be_bytes(
                r[hl + 8..hl + 12].try_into().unwrap_or([0; 4]),
            )) & 3)
                << 32
                | u64::from(u32::from_be_bytes(
                    r[hl + 12..hl + 16].try_into().unwrap_or([0; 4]),
                ));
            if time as i64 != c.committer.timestamp {
                errs.push(format!(
                    "commit date for commit {} in commit-graph is {} != {}",
                    oid.to_hex(),
                    time,
                    c.committer.timestamp
                ));
            }
        }
        let _ = base;
        errs
    }
}

fn read_commit(repo: &Repository, oid: &Oid) -> Option<Commit> {
    let obj = repo.get_object(oid).ok()?;
    (obj.kind == ObjectKind::Commit)
        .then(|| Commit::decode(&obj.payload).ok())
        .flatten()
}

/// Commits in `packs` (idx paths) of the object directory.
fn pack_commits(repo: &Repository, idx_files: &[PathBuf]) -> Vec<Oid> {
    let mut out = Vec::new();
    for p in idx_files {
        let Ok(bytes) = fs::read(p) else { continue };
        let Ok(index) = arks_pack::PackIndex::decode(&bytes, repo.object_format()) else {
            continue;
        };
        for e in index.entries {
            if repo
                .get_object(&e.oid)
                .is_ok_and(|o| o.kind == ObjectKind::Commit)
            {
                out.push(e.oid);
            }
        }
    }
    out
}

fn all_idx(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir.join("pack"))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "idx"))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Commit metadata the writer needs.
struct Info {
    tree: Oid,
    parents: Vec<Oid>,
    time: u64,
}

fn write_cmd(args: &[String], obj_dir: Option<String>) -> CliResult<()> {
    sub_help(args, USAGE_WRITE)?;
    let p = parse("commit-graph write", args, WRITE_DEFS, Mode::default())?;
    if !p.args().is_empty() {
        return Err(CliError::Exit(129, sub_usage(USAGE_WRITE)));
    }
    let sources = ["reachable", "stdin-commits", "stdin-packs"]
        .iter()
        .filter(|k| p.on(k))
        .count();
    if sources > 1 {
        return Err(CliError::fatal(
            "use at most one of --reachable, --stdin-commits, or --stdin-packs",
        ));
    }
    let split = match p.last("split") {
        None => None,
        Some(None) => Some(String::new()),
        Some(Some(v)) => match v.as_str() {
            "no-merge" | "replace" => Some(v),
            other => {
                return Err(CliError::fatal(format!(
                    "unrecognized --split argument, {other}"
                )));
            }
        },
    };
    let num = |k: &str| -> CliResult<Option<u64>> {
        match p.value(k) {
            None => Ok(None),
            Some(v) => v.parse::<u64>().map(Some).map_err(|_| {
                CliError::Exit(
                    129,
                    format!("error: option `{k}' expects a numerical value"),
                )
            }),
        }
    };
    let size_multiple = num("size-multiple")?.unwrap_or(2).max(1);
    let max_commits = num("max-commits")?;
    let max_new_filters = match p.last("max-new-filters") {
        Some(Some(v)) => Some(v.parse::<i64>().map_err(|_| {
            CliError::Exit(
                129,
                "error: option `max-new-filters' expects a numerical value".into(),
            )
        })?),
        _ => None,
    };
    let repo = open_repo()?;
    let dir = object_dir(&repo, p.value("object-dir").or(obj_dir).as_deref())?;
    // Start set.
    let mut start: Vec<Oid> = Vec::new();
    if p.on("reachable") {
        for (_, oid) in repo.refs().list_refs("refs/")? {
            if let Ok(c) = crate::cmd::shared::peel_tags(&repo, &oid) {
                start.push(c);
            }
        }
        if let Ok(h) = repo.head_oid() {
            start.push(h);
        }
    } else if p.on("stdin-commits") {
        for line in io::stdin().lock().lines() {
            let line = line?;
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            let oid = Oid::from_hex(l).map_err(|_| {
                CliError::Exit(1, format!("error: unexpected non-hex object ID: {l}"))
            })?;
            if let Ok(c) = crate::cmd::shared::peel_tags(&repo, &oid) {
                start.push(c);
            }
        }
    } else if p.on("stdin-packs") {
        let mut files = Vec::new();
        for line in io::stdin().lock().lines() {
            let line = line?;
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            let f = dir.join("pack").join(l);
            if !f.is_file() {
                return Err(CliError::Exit(
                    1,
                    format!("error: error adding pack {}", f.display()),
                ));
            }
            files.push(f);
        }
        start = pack_commits(&repo, &files);
    } else {
        start = pack_commits(&repo, &all_idx(&dir));
    }
    let layers = existing_layers(&dir);
    let mut old_graphs: Vec<Graph> = Vec::new();
    for (path, _) in &layers {
        if let Ok(g) = Graph::read(path, repo.object_format()) {
            old_graphs.push(g);
        }
    }
    if p.on("append") || split.as_deref() == Some("") || split.as_deref() == Some("no-merge") {
        for g in &old_graphs {
            start.extend(g.oids.iter().copied());
        }
    }
    // Close under parents.
    let mut infos: HashMap<Oid, Info> = HashMap::new();
    let mut stack = start;
    while let Some(c) = stack.pop() {
        if infos.contains_key(&c) {
            continue;
        }
        let Some(commit) = read_commit(&repo, &c) else {
            continue;
        };
        stack.extend(commit.parents.iter().copied());
        infos.insert(
            c,
            Info {
                tree: commit.tree,
                parents: commit.parents,
                time: commit.committer.timestamp.max(0) as u64,
            },
        );
    }
    if infos.is_empty() {
        return Ok(());
    }
    let changed_paths = p.on("changed-paths")
        || (p.flag("changed-paths").is_none()
            && old_graphs.iter().any(|g| g.chunks.contains_key(b"BDAT")));
    let info_dir = dir.join("info");
    fs::create_dir_all(&info_dir)?;
    let all: BTreeSet<Vec<u8>> = infos.keys().map(|o| o.as_bytes().to_vec()).collect();
    let kind = repo.oid_kind();
    let mut sorted: Vec<Oid> = all
        .iter()
        .filter_map(|b| Oid::from_bytes(kind, b).ok())
        .collect();
    let filters = FilterBudget {
        enabled: changed_paths,
        max_new: max_new_filters,
    };
    match split {
        None => {
            let bytes = encode_graph(&repo, &sorted, &infos, &[], &[], filters)?;
            fs::write(info_dir.join("commit-graph"), bytes)?;
            let chain_dir = info_dir.join("commit-graphs");
            if chain_dir.is_dir() {
                let _ = fs::remove_dir_all(&chain_dir);
            }
        }
        Some(mode) => {
            let chain_dir = info_dir.join("commit-graphs");
            fs::create_dir_all(&chain_dir)?;
            // Lower layers kept as they are.
            let mut keep: Vec<(String, Vec<Oid>)> = Vec::new();
            if mode != "replace" {
                for ((_, h), g) in layers.iter().zip(&old_graphs) {
                    if !h.is_empty() {
                        keep.push((h.clone(), g.oids.clone()));
                    }
                }
            }
            let in_lower = |keep: &[(String, Vec<Oid>)]| -> HashSet<Oid> {
                keep.iter().flat_map(|(_, v)| v.iter().copied()).collect()
            };
            if mode.is_empty() {
                // Merge the top layers while they are not much larger than the new one.
                loop {
                    let lower = in_lower(&keep);
                    let new_n = sorted.iter().filter(|o| !lower.contains(o)).count() as u64;
                    let Some((_, top)) = keep.last() else { break };
                    let too_many = max_commits.is_some_and(|m| new_n > m);
                    if (top.len() as u64) <= new_n * size_multiple || too_many {
                        keep.pop();
                    } else {
                        break;
                    }
                }
            }
            let lower = in_lower(&keep);
            sorted.retain(|o| !lower.contains(o));
            let base_oids: Vec<Oid> = keep.iter().flat_map(|(_, v)| v.iter().copied()).collect();
            let base_hashes: Vec<String> = keep.iter().map(|(h, _)| h.clone()).collect();
            let mut chain: Vec<String> = base_hashes.clone();
            if !sorted.is_empty() || chain.is_empty() {
                let bytes =
                    encode_graph(&repo, &sorted, &infos, &base_oids, &base_hashes, filters)?;
                let h = hex(&bytes[bytes.len() - repo.object_format().digest_len()..]);
                fs::write(chain_dir.join(format!("graph-{h}.graph")), bytes)?;
                chain.push(h);
            }
            let text: String = chain.iter().map(|h| format!("{h}\n")).collect();
            fs::write(chain_dir.join("commit-graph-chain"), text)?;
            let _ = fs::remove_file(info_dir.join("commit-graph"));
            // Drop layer files no longer in the chain (subject to --expire-time).
            let expire = p
                .value("expire-time")
                .and_then(|v| crate::cmd::config::expiry_date(&v));
            if let Ok(rd) = fs::read_dir(&chain_dir) {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let Some(h) = name
                        .strip_prefix("graph-")
                        .and_then(|n| n.strip_suffix(".graph"))
                    else {
                        continue;
                    };
                    if chain.iter().any(|c| c == h) {
                        continue;
                    }
                    let old_enough = match expire {
                        None => true,
                        Some(t) => e
                            .metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                            .is_some_and(|d| (d.as_secs() as i64) <= t),
                    };
                    if old_enough {
                        let _ = fs::remove_file(e.path());
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct FilterBudget {
    enabled: bool,
    max_new: Option<i64>,
}

/// Topological levels and corrected commit dates over `infos`.
fn generations(infos: &HashMap<Oid, Info>) -> HashMap<Oid, (u32, u64)> {
    let mut levels: HashMap<Oid, (u32, u64)> = HashMap::new();
    let keys: Vec<Oid> = infos.keys().copied().collect();
    for k in keys {
        let mut stack = vec![(k, false)];
        while let Some((c, expanded)) = stack.pop() {
            if levels.contains_key(&c) {
                continue;
            }
            let Some(info) = infos.get(&c) else {
                levels.insert(c, (0, 0));
                continue;
            };
            if !expanded {
                stack.push((c, true));
                for p in &info.parents {
                    if !levels.contains_key(p) {
                        stack.push((*p, false));
                    }
                }
                continue;
            }
            let mut level = 0u32;
            let mut corrected = info.time;
            for p in &info.parents {
                let (pl, pc) = levels.get(p).copied().unwrap_or((0, 0));
                level = level.max(pl);
                corrected = corrected.max(pc + 1);
            }
            levels.insert(c, ((level + 1).min(0x3fff_ffff), corrected));
        }
    }
    levels
}

fn murmur3(seed: u32, data: &[u8]) -> u32 {
    let (c1, c2) = (0xcc9e_2d51u32, 0x1b87_3593u32);
    let mut h = seed;
    let chunks = data.len() / 4;
    for i in 0..chunks {
        let mut k = u32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().unwrap_or([0; 4]));
        k = k.wrapping_mul(c1).rotate_left(15).wrapping_mul(c2);
        h ^= k;
        h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xe654_6b64);
    }
    // Version 1 filters hash the tail bytes as signed chars.
    let tail = &data[chunks * 4..];
    let b = |i: usize| (tail[i] as i8) as i32 as u32;
    let mut k1: u32 = 0;
    if tail.len() >= 3 {
        k1 ^= b(2) << 16;
    }
    if tail.len() >= 2 {
        k1 ^= b(1) << 8;
    }
    if !tail.is_empty() {
        k1 ^= b(0);
        k1 = k1.wrapping_mul(c1).rotate_left(15).wrapping_mul(c2);
        h ^= k1;
    }
    h ^= data.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

fn flat(repo: &Repository, tree: &Oid, prefix: &str, out: &mut HashMap<String, Oid>) {
    let Ok(obj) = repo.get_object(tree) else {
        return;
    };
    let Ok(t) = Tree::decode(&obj.payload, repo.oid_kind()) else {
        return;
    };
    for e in t.entries {
        let p = if prefix.is_empty() {
            e.name.clone()
        } else {
            format!("{prefix}/{}", e.name)
        };
        if e.mode == TreeMode::Tree {
            out.insert(format!("{p}/"), e.oid);
            flat(repo, &e.oid, &p, out);
        } else {
            out.insert(p, e.oid);
        }
    }
}

/// Bloom filter bytes of the paths changed against the first parent.
fn bloom_filter(repo: &Repository, info: &Info, infos: &HashMap<Oid, Info>) -> Vec<u8> {
    let mut new = HashMap::new();
    flat(repo, &info.tree, "", &mut new);
    let mut old = HashMap::new();
    if let Some(p) = info.parents.first() {
        let ptree = infos
            .get(p)
            .map(|i| i.tree)
            .or_else(|| read_commit(repo, p).map(|c| c.tree));
        if let Some(t) = ptree {
            flat(repo, &t, "", &mut old);
        }
    }
    let mut changed: BTreeSet<String> = BTreeSet::new();
    for (k, v) in &new {
        if !k.ends_with('/') && old.get(k) != Some(v) {
            changed.insert(k.clone());
        }
    }
    for k in old.keys() {
        if !k.ends_with('/') && !new.contains_key(k) {
            changed.insert(k.clone());
        }
    }
    // Every leading directory of a changed path counts too.
    let mut paths: BTreeSet<String> = BTreeSet::new();
    for c in &changed {
        let mut p = c.as_str();
        paths.insert(p.to_string());
        while let Some(i) = p.rfind('/') {
            p = &p[..i];
            paths.insert(p.to_string());
        }
    }
    if paths.len() > MAX_CHANGED_PATHS {
        return vec![0xff];
    }
    if paths.is_empty() {
        return vec![0];
    }
    // ceil(paths * bits-per-entry / 8) bytes; bit `pos` is bit `pos % 8`
    // (least significant first) of byte `pos / 8`.
    let len = (paths.len() as u64 * u64::from(BLOOM_BITS_PER_ENTRY)).div_ceil(8);
    let bits = len * 8;
    let mut filter = vec![0u8; len as usize];
    for p in &paths {
        let h0 = murmur3(0x293a_e76f, p.as_bytes());
        let h1 = murmur3(0x7e64_6e2c, p.as_bytes());
        for i in 0..BLOOM_HASHES {
            let pos = u64::from(h0.wrapping_add(i.wrapping_mul(h1))) % bits;
            filter[(pos / 8) as usize] |= 1 << (pos % 8);
        }
    }
    filter
}

fn encode_graph(
    repo: &Repository,
    oids: &[Oid],
    infos: &HashMap<Oid, Info>,
    base_oids: &[Oid],
    base_hashes: &[String],
    filters: FilterBudget,
) -> CliResult<Vec<u8>> {
    let format = repo.object_format();
    let hl = format.digest_len();
    let gens = generations(infos);
    let mut pos: HashMap<Oid, u32> = HashMap::new();
    for (i, o) in base_oids.iter().enumerate() {
        pos.insert(*o, i as u32);
    }
    for (i, o) in oids.iter().enumerate() {
        pos.insert(*o, (base_oids.len() + i) as u32);
    }
    // Chunk bodies.
    let mut fanout = vec![0u8; 256 * 4];
    let mut counts = [0u32; 256];
    for o in oids {
        counts[o.as_bytes()[0] as usize] += 1;
    }
    let mut acc = 0u32;
    for (i, c) in counts.iter().enumerate() {
        acc += c;
        fanout[i * 4..i * 4 + 4].copy_from_slice(&acc.to_be_bytes());
    }
    let mut oidl = Vec::with_capacity(oids.len() * hl);
    let mut cdat = Vec::with_capacity(oids.len() * (hl + 16));
    let mut edge: Vec<u8> = Vec::new();
    let mut gda2: Vec<u8> = Vec::new();
    let mut gdo2: Vec<u8> = Vec::new();
    let mut bidx: Vec<u8> = Vec::new();
    let mut bdat: Vec<u8> = Vec::new();
    let mut new_filters = 0i64;
    for o in oids {
        oidl.extend_from_slice(o.as_bytes());
        let info = &infos[o];
        cdat.extend_from_slice(info.tree.as_bytes());
        let pp = |p: &Oid| pos.get(p).copied().unwrap_or(NO_PARENT);
        let p1 = info.parents.first().map_or(NO_PARENT, pp);
        let p2 = match info.parents.len() {
            0 | 1 => NO_PARENT,
            2 => pp(&info.parents[1]),
            _ => {
                let at = (edge.len() / 4) as u32;
                let rest = &info.parents[1..];
                for (k, p) in rest.iter().enumerate() {
                    let mut v = pp(p);
                    if k + 1 == rest.len() {
                        v |= LAST_EDGE;
                    }
                    edge.extend_from_slice(&v.to_be_bytes());
                }
                EXTRA_EDGES | at
            }
        };
        cdat.extend_from_slice(&p1.to_be_bytes());
        cdat.extend_from_slice(&p2.to_be_bytes());
        let (level, corrected) = gens.get(o).copied().unwrap_or((1, info.time));
        let t = info.time & 0x3_ffff_ffff;
        cdat.extend_from_slice(&((level << 2) | (t >> 32) as u32).to_be_bytes());
        cdat.extend_from_slice(&(t as u32).to_be_bytes());
        let offset = corrected - info.time;
        if offset > 0x7fff_ffff {
            let idx = (gdo2.len() / 8) as u32;
            gdo2.extend_from_slice(&offset.to_be_bytes());
            gda2.extend_from_slice(&(0x8000_0000 | idx).to_be_bytes());
        } else {
            gda2.extend_from_slice(&(offset as u32).to_be_bytes());
        }
        if filters.enabled {
            let compute = filters.max_new.is_none_or(|m| m < 0 || new_filters < m);
            if compute {
                bdat.extend_from_slice(&bloom_filter(repo, info, infos));
                new_filters += 1;
            }
            bidx.extend_from_slice(&(bdat.len() as u32).to_be_bytes());
        }
    }
    let mut chunks: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"OIDF", fanout),
        (*b"OIDL", oidl),
        (*b"CDAT", cdat),
        (*b"GDA2", gda2),
    ];
    if !gdo2.is_empty() {
        chunks.push((*b"GDO2", gdo2));
    }
    if !edge.is_empty() {
        chunks.push((*b"EDGE", edge));
    }
    if filters.enabled {
        chunks.push((*b"BIDX", bidx));
        let mut d = Vec::new();
        d.extend_from_slice(&1u32.to_be_bytes());
        d.extend_from_slice(&BLOOM_HASHES.to_be_bytes());
        d.extend_from_slice(&BLOOM_BITS_PER_ENTRY.to_be_bytes());
        d.extend_from_slice(&bdat);
        chunks.push((*b"BDAT", d));
    }
    if !base_hashes.is_empty() {
        let mut b = Vec::new();
        for h in base_hashes {
            let oid = Oid::from_hex(h).map_err(|e| CliError::Msg(e.to_string()))?;
            b.extend_from_slice(oid.as_bytes());
        }
        chunks.push((*b"BASE", b));
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"CGPH");
    out.push(1);
    out.push(hash_version(format));
    out.push(chunks.len() as u8);
    out.push(base_hashes.len() as u8);
    let mut off = (8 + (chunks.len() + 1) * 12) as u64;
    for (id, body) in &chunks {
        out.extend_from_slice(id);
        out.extend_from_slice(&off.to_be_bytes());
        off += body.len() as u64;
    }
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&off.to_be_bytes());
    for (_, body) in &chunks {
        out.extend_from_slice(body);
    }
    let mut h = Hasher::new(format);
    h.update(&out);
    let d = h.finalize().map_err(|e| CliError::Msg(e.to_string()))?;
    out.extend_from_slice(d.as_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::murmur3;

    #[test]
    fn murmur3_reference_values() {
        // Published murmur3 x86_32 test vectors.
        assert_eq!(murmur3(0, b""), 0);
        assert_eq!(murmur3(1, b""), 0x514e_28b7);
        assert_eq!(murmur3(0, b"test"), 0xba6b_d213);
        assert_eq!(murmur3(0x9747_b28c, b"Hello, world!"), 0x2488_4cba);
    }
}
