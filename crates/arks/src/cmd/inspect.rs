//! Ancillary interrogators: fsck / count-objects / pack-refs (blame lives in `blame`).
//!
//! Cite: <https://git-scm.com/docs/git-fsck>
//! Cite: <https://git-scm.com/docs/git-count-objects>
//! Cite: <https://git-scm.com/docs/git-pack-refs>
//! Output order and wording follow observed stock git (family-plumbing corpus).

use crate::cmd::opt_parse::{self, OptDef, Takes::*, o};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, Tree};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Every `.idx` in `objects/pack` (sorted by name) with its object names,
/// in name order and in pack offset order.
type PackNames = (PathBuf, Vec<Oid>, Vec<Oid>);

fn pack_indexes(repo: &Repository) -> Vec<PackNames> {
    let dir = repo.store().objects_dir().join("pack");
    let mut idx: Vec<PathBuf> = fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "idx"))
                .collect()
        })
        .unwrap_or_default();
    idx.sort();
    idx.into_iter()
        .filter_map(|p| {
            let bytes = fs::read(&p).ok()?;
            let index = arks_pack::PackIndex::decode(&bytes, repo.object_format()).ok()?;
            let by_name = index.entries.iter().map(|e| e.oid).collect();
            let mut by_offset: Vec<_> = index.entries.iter().map(|e| (e.offset, e.oid)).collect();
            by_offset.sort_by_key(|(off, _)| *off);
            Some((p, by_name, by_offset.into_iter().map(|(_, o)| o).collect()))
        })
        .collect()
}

/// A path as git prints it: relative to the current directory when below it.
fn shown(p: &Path) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    p.strip_prefix(&cwd)
        .map(|r| r.display().to_string())
        .unwrap_or_else(|_| p.display().to_string())
}

/// Loose objects in git's scan order: fan-out directories 00..ff, entries in
/// directory (readdir) order.
fn loose_scan_order(repo: &Repository) -> Vec<(Oid, PathBuf)> {
    let objects = repo.store().objects_dir();
    let mut out = Vec::new();
    for i in 0..=255u8 {
        let prefix = format!("{i:02x}");
        let Ok(rd) = fs::read_dir(objects.join(&prefix)) else {
            continue;
        };
        for e in rd.flatten() {
            let hex = format!("{prefix}{}", e.file_name().to_string_lossy());
            if let Ok(oid) = Oid::from_hex(&hex) {
                out.push((oid, e.path()));
            }
        }
    }
    out
}

/// git's in-memory object table, as its `fsck` report order shows it:
/// open addressing on the first four name bytes read little endian, a
/// table of 32 slots doubled (re-inserting in slot order) whenever an insert
/// would pass half load, and a lookup that finds an object past the slot
/// where its probe started swaps it into that slot. Every lookup git makes
/// therefore shapes the order, so `fsck` replays them in git's sequence.
#[derive(Default)]
struct ObjectTable {
    slots: Vec<Option<Oid>>,
    at: HashMap<Oid, usize>,
}

impl ObjectTable {
    fn home(&self, oid: &Oid) -> usize {
        let b = oid.as_bytes();
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize & (self.slots.len() - 1)
    }

    fn place(&mut self, oid: Oid) {
        let mask = self.slots.len() - 1;
        let mut i = self.home(&oid);
        while self.slots[i].is_some() {
            i = (i + 1) & mask;
        }
        self.slots[i] = Some(oid);
        self.at.insert(oid, i);
    }

    /// Look `oid` up, creating its entry when absent.
    fn look(&mut self, oid: Oid) {
        if let Some(&i) = self.at.get(&oid) {
            let first = self.home(&oid);
            if i != first {
                self.slots.swap(i, first);
                if let Some(moved) = self.slots[i] {
                    self.at.insert(moved, i);
                }
                self.at.insert(oid, first);
            }
            return;
        }
        if (self.at.len() + 1) * 2 > self.slots.len() {
            let size = (self.slots.len() * 2).max(32);
            let old = std::mem::replace(&mut self.slots, vec![None; size]);
            for o in old.into_iter().flatten() {
                self.place(o);
            }
        }
        self.place(oid);
    }

    fn contains(&self, oid: &Oid) -> bool {
        self.at.contains_key(oid)
    }

    /// Entries in slot order.
    fn order(&self) -> impl Iterator<Item = Oid> + '_ {
        self.slots.iter().flatten().copied()
    }
}

fn tag_header(payload: &[u8]) -> (Option<Oid>, String, String) {
    let text = String::from_utf8_lossy(payload);
    let (mut target, mut ty, mut name) = (None, String::new(), String::new());
    for line in text.lines() {
        if line.is_empty() {
            break;
        }
        if let Some(r) = line.strip_prefix("object ") {
            target = Oid::from_hex(r.trim()).ok();
        } else if let Some(r) = line.strip_prefix("type ") {
            ty = r.to_string();
        } else if let Some(r) = line.strip_prefix("tag ") {
            name = r.to_string();
        }
    }
    (target, ty, name)
}

/// `arks fsck [--tags] [--root] [--unreachable] [--cache] [--no-reflogs]
/// [--[no-]full] [--strict] [--verbose] [--lost-found] [--[no-]dangling]
/// [--[no-]progress] [--connectivity-only] [--[no-]name-objects] [<object>...]`
///
/// Exit status: bit 0 for corrupt objects or bad arguments, bit 1 for
/// missing objects. OBJ-004: content whose recomputed name differs from its
/// path is reported and treated as missing (sha1 collision pretend refuse).
pub fn cmd_fsck(args: &[String]) -> CliResult<()> {
    const SPEC: &[OptDef] = &[
        o("verbose", 'v', No, true),
        o("unreachable", ' ', No, true),
        o("dangling", ' ', No, true),
        o("tags", ' ', No, true),
        o("root", ' ', No, true),
        o("cache", ' ', No, true),
        o("reflogs", ' ', No, true),
        o("full", ' ', No, true),
        o("connectivity-only", ' ', No, true),
        o("strict", ' ', No, true),
        o("lost-found", ' ', No, true),
        o("progress", ' ', No, true),
        o("name-objects", ' ', No, true),
    ];
    let items = opt_parse::parse("fsck", args, SPEC, opt_parse::Mode::default())?;
    let verbose = items.on("verbose");
    let show_unreachable = items.on("unreachable");
    let show_dangling = items.flag("dangling").unwrap_or(true);
    let show_tags = items.on("tags");
    let show_root = items.on("root");
    let use_cache = items.on("cache");
    let lost_found = items.on("lost-found");
    // --lost-found implies --no-reflogs.
    let reflogs = items.flag("reflogs").unwrap_or(true) && !lost_found;
    let connectivity_only = items.on("connectivity-only");
    let heads = items.operands();

    let repo = open_repo()?;
    let format = repo.object_format();
    let mut errors = 0u8;
    let mut out = String::new();

    // Phase 1: check every stored copy (loose first, then each pack).
    let loose = loose_scan_order(&repo);
    let packs = pack_indexes(&repo);
    if verbose {
        eprintln!("Checking object directory");
    }
    // Objects referenced by each object, with their expected type.
    let children = |oid: &Oid| -> Vec<(Oid, &'static str)> {
        let Ok(obj) = repo.get_object(oid) else {
            return Vec::new();
        };
        match obj.kind {
            ObjectKind::Commit => Commit::decode(&obj.payload)
                .map(|c| {
                    let mut v = vec![(c.tree, "tree")];
                    v.extend(c.parents.iter().map(|p| (*p, "commit")));
                    v
                })
                .unwrap_or_default(),
            ObjectKind::Tree => Tree::decode(&obj.payload, repo.oid_kind())
                .map(|t| {
                    t.entries
                        .iter()
                        .filter(|e| e.mode.as_str() != "160000")
                        .map(|e| (e.oid, if e.mode.is_tree() { "tree" } else { "blob" }))
                        .collect()
                })
                .unwrap_or_default(),
            ObjectKind::Tag => {
                let (t, ty, _) = tag_header(&obj.payload);
                let ty: &'static str = match ty.as_str() {
                    "commit" => "commit",
                    "tree" => "tree",
                    "tag" => "tag",
                    _ => "blob",
                };
                t.map(|t| vec![(t, ty)]).unwrap_or_default()
            }
            ObjectKind::Blob => Vec::new(),
        }
    };
    let mut present: HashSet<Oid> = HashSet::new();
    let mut expected: HashMap<Oid, &'static str> = HashMap::new();
    let mut kinds: HashMap<Oid, ObjectKind> = HashMap::new();
    let mut corrupt: HashSet<Oid> = HashSet::new();
    // Observed lookup sequence of the scan: each loose object (scan order),
    // then each packed object (offset order), every one followed by the
    // objects it names.
    let mut table = ObjectTable::default();
    for o in loose
        .iter()
        .map(|(o, _)| o)
        .chain(packs.iter().flat_map(|(_, _, by_offset)| by_offset.iter()))
    {
        table.look(*o);
        for (c, _) in children(o) {
            table.look(c);
        }
    }
    let copies = loose.iter().map(|(o, p)| (*o, Some(p.clone()))).chain(
        packs
            .iter()
            .flat_map(|(_, oids, _)| oids.iter().map(|o| (*o, None))),
    );
    for (oid, path) in copies {
        for (c, ty) in children(&oid) {
            expected.entry(c).or_insert(ty);
        }
        let obj = match repo.get_object(&oid) {
            Ok(o) => o,
            Err(_) => {
                corrupt.insert(oid);
                continue;
            }
        };
        if !connectivity_only {
            match obj.oid(format) {
                Ok(got) if got == oid => {}
                Ok(got) => {
                    let at = path.as_deref().map(shown).unwrap_or_default();
                    eprintln!(
                        "error: {}: hash-path mismatch, found at: {at}",
                        got.to_hex()
                    );
                    errors |= 1;
                    corrupt.insert(oid);
                    continue;
                }
                Err(e) => {
                    eprintln!("error: {}: object corrupt or missing: {e}", oid.to_hex());
                    errors |= 1;
                    corrupt.insert(oid);
                    continue;
                }
            }
            if verbose {
                eprintln!("Checking {} {}", obj.kind.as_str(), oid.to_hex());
            }
            match obj.kind {
                ObjectKind::Commit if show_root => {
                    if Commit::decode(&obj.payload).is_ok_and(|c| c.parents.is_empty()) {
                        out.push_str(&format!("root {}\n", oid.to_hex()));
                    }
                }
                ObjectKind::Tag if show_tags => {
                    if let (Some(t), ty, name) = tag_header(&obj.payload) {
                        out.push_str(&format!(
                            "tagged {ty} {} ({name}) in {}\n",
                            t.to_hex(),
                            oid.to_hex()
                        ));
                    }
                }
                _ => {}
            }
        }
        present.insert(oid);
        kinds.insert(oid, obj.kind);
    }

    // Phase 2: connectivity from the head nodes.
    let mut roots: Vec<Oid> = Vec::new();
    if heads.is_empty() {
        // Refs in name order, HEAD after them.
        let mut head = None;
        for (name, oid) in repo.refs().list_refs("")? {
            if name == "HEAD" {
                head = Some(oid);
            } else {
                roots.push(oid);
            }
        }
        roots.extend(head);
        if reflogs {
            // git-fsck walks every reflog, including logs of deleted refs.
            for name in all_reflog_names(&repo) {
                for (old, new, _) in crate::reflog::read_entries(&repo, &name) {
                    roots.push(new);
                    roots.extend(old);
                }
            }
        }
    } else {
        for h in &heads {
            match crate::cmd::shared::resolve_rev(&repo, h) {
                Ok(o) => roots.push(o),
                Err(_) => {
                    eprintln!("error: invalid parameter: expected sha1, got '{h}'");
                    errors |= 1;
                }
            }
        }
    }
    if heads.is_empty() || use_cache {
        if let Ok(index) = crate::util::read_index(&repo) {
            for e in &index.entries {
                if e.mode != 0o160000 {
                    expected.entry(e.oid).or_insert("blob");
                    roots.push(e.oid);
                }
            }
        }
    }
    roots.retain(|o| o.as_bytes().iter().any(|b| *b != 0));
    for o in &roots {
        table.look(*o);
    }
    let mut reachable: HashSet<Oid> = HashSet::new();
    let mut missing: HashMap<Oid, &'static str> = HashMap::new();
    let mut stack: Vec<(Oid, Option<&'static str>)> = roots.iter().map(|o| (*o, None)).collect();
    while let Some((oid, want)) = stack.pop() {
        if !reachable.insert(oid) {
            continue;
        }
        if !present.contains(&oid) {
            let ty = want.or_else(|| expected.get(&oid).copied());
            missing.insert(oid, ty.unwrap_or("object"));
            continue;
        }
        // The walk looks up the entries of each tree it reaches.
        let is_tree = kinds.get(&oid) == Some(&ObjectKind::Tree);
        for (c, ty) in children(&oid) {
            if is_tree {
                table.look(c);
            }
            stack.push((c, Some(ty)));
        }
    }
    let unreachable: Vec<Oid> = present
        .iter()
        .filter(|o| !reachable.contains(*o))
        .copied()
        .collect();
    let unreachable_set: HashSet<Oid> = unreachable.iter().copied().collect();
    let mut referenced: HashSet<Oid> = HashSet::new();
    for o in &unreachable {
        for (c, _) in children(o) {
            if unreachable_set.contains(&c) {
                referenced.insert(c);
            }
        }
    }
    if !missing.is_empty() {
        errors |= 2;
    }

    // Phase 3: report in object-table order.
    for o in missing.keys() {
        if !table.contains(o) {
            table.look(*o);
        }
    }
    let mut dangling = Vec::new();
    for oid in table.order() {
        if let Some(ty) = missing.get(&oid) {
            out.push_str(&format!("missing {ty} {}\n", oid.to_hex()));
            continue;
        }
        if !unreachable_set.contains(&oid) {
            continue;
        }
        let kind = kinds.get(&oid).map(|k| k.as_str()).unwrap_or("object");
        if show_unreachable {
            out.push_str(&format!("unreachable {kind} {}\n", oid.to_hex()));
        } else if show_dangling && !referenced.contains(&oid) {
            out.push_str(&format!("dangling {kind} {}\n", oid.to_hex()));
        }
        if !referenced.contains(&oid) {
            dangling.push(oid);
        }
    }
    print!("{out}");
    if lost_found {
        let base = repo.git_dir().join("lost-found");
        for oid in dangling {
            let Ok(obj) = repo.get_object(&oid) else {
                continue;
            };
            let (sub, body) = match obj.kind {
                ObjectKind::Commit => ("commit", format!("{}\n", oid.to_hex()).into_bytes()),
                ObjectKind::Blob => ("other", obj.payload.to_vec()),
                _ => ("other", format!("{}\n", oid.to_hex()).into_bytes()),
            };
            let dir = base.join(sub);
            fs::create_dir_all(&dir)?;
            fs::write(dir.join(oid.to_hex()), body)?;
        }
    }
    if errors != 0 {
        return Err(CliError::silent(errors));
    }
    Ok(())
}

/// `1.60 KiB` / `12 bytes`, as git's `-H` prints sizes.
fn human(bytes: u64) -> String {
    const K: f64 = 1024.0;
    let b = bytes as f64;
    if bytes >= 1 << 30 {
        format!("{:.2} GiB", b / (K * K * K))
    } else if bytes >= 1 << 20 {
        format!("{:.2} MiB", b / (K * K))
    } else if bytes >= 1 << 10 {
        format!("{:.2} KiB", b / K)
    } else if bytes == 1 {
        "1 byte".into()
    } else {
        format!("{bytes} bytes")
    }
}

/// Disk space a file occupies (git `on_disk_bytes`: allocated blocks).
fn on_disk_bytes(meta: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.blocks() * 512
    }
    #[cfg(not(unix))]
    {
        meta.len()
    }
}

/// `arks count-objects [-v | --verbose] [-H | --human-readable]`.
pub fn cmd_count_objects(args: &[String]) -> CliResult<()> {
    const SPEC: &[OptDef] = &[
        o("verbose", 'v', No, true),
        o("human-readable", 'H', No, true),
    ];
    let items = opt_parse::parse("count-objects", args, SPEC, opt_parse::Mode::default())?;
    let verbose = items.on("verbose");
    let hr = items.on("human-readable");
    if !items.operands().is_empty() {
        return Err(CliError::Exit(
            129,
            "usage: git count-objects [-v] [-H | --human-readable]".into(),
        ));
    }
    let repo = open_repo()?;
    let objects = repo.store().objects_dir().to_path_buf();
    let (mut count, mut size) = (0u64, 0u64);
    let mut garbage: Vec<(PathBuf, u64)> = Vec::new();
    let loose = crate::cmd::plumbing_ext::loose_objects(&repo);
    let loose_paths: HashSet<PathBuf> = loose.iter().map(|(_, p)| p.clone()).collect();
    if let Ok(rd) = fs::read_dir(&objects) {
        let mut dirs: Vec<PathBuf> = rd
            .flatten()
            .filter(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.len() == 2 && n.bytes().all(|b| b.is_ascii_hexdigit()) && e.path().is_dir()
            })
            .map(|e| e.path())
            .collect();
        dirs.sort();
        for d in dirs {
            let Ok(inner) = fs::read_dir(&d) else {
                continue;
            };
            let mut files: Vec<_> = inner.flatten().collect();
            files.sort_by_key(|e| e.file_name());
            for f in files {
                let Ok(meta) = f.metadata() else { continue };
                if !meta.is_file() {
                    continue;
                }
                if loose_paths.contains(&f.path()) {
                    count += 1;
                    size += on_disk_bytes(&meta);
                } else {
                    garbage.push((f.path(), meta.len()));
                }
            }
        }
    }
    let packs = pack_indexes(&repo);
    let mut in_pack = 0u64;
    let mut size_pack = 0u64;
    let mut packed: HashSet<Oid> = HashSet::new();
    for (idx, oids, _) in &packs {
        in_pack += oids.len() as u64;
        packed.extend(oids.iter().copied());
        size_pack += fs::metadata(idx).map(|m| m.len()).unwrap_or(0);
        size_pack += fs::metadata(idx.with_extension("pack"))
            .map(|m| m.len())
            .unwrap_or(0);
    }
    if let Ok(rd) = fs::read_dir(objects.join("pack")) {
        let mut files: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        files.sort();
        for p in files {
            let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("");
            let known = ["pack", "idx", "rev", "keep", "bitmap", "promisor", "mtimes"];
            if p.is_file() && !known.contains(&ext) {
                garbage.push((p.clone(), fs::metadata(&p).map(|m| m.len()).unwrap_or(0)));
            }
        }
    }
    // git reports pack-directory garbage before loose-directory garbage.
    let pack_dir = objects.join("pack");
    garbage.sort_by_key(|(p, _)| !p.starts_with(&pack_dir));
    let prune_packable = loose.iter().filter(|(o, _)| packed.contains(o)).count();
    for (p, _) in &garbage {
        eprintln!("warning: garbage found: {}", shown(p));
    }
    let size_garbage: u64 = garbage.iter().map(|(_, n)| n).sum();
    let fmt = |bytes: u64| {
        if hr {
            human(bytes)
        } else {
            (bytes / 1024).to_string()
        }
    };
    if verbose {
        println!("count: {count}");
        println!("size: {}", fmt(size));
        println!("in-pack: {in_pack}");
        println!("packs: {}", packs.len());
        println!("size-pack: {}", fmt(size_pack));
        println!("prune-packable: {prune_packable}");
        println!("garbage: {}", garbage.len());
        println!("size-garbage: {}", fmt(size_garbage));
    } else if hr {
        println!("{count} objects, {}", human(size));
    } else {
        println!("{count} objects, {} kilobytes", size / 1024);
    }
    Ok(())
}

/// `arks pack-refs [--all] [--no-prune] [--include <pattern>] [--exclude <pattern>]`.
/// Without `--all` (or `--include`) only tags are packed; refs already packed
/// stay packed. Loose copies are removed unless `--no-prune`.
pub fn cmd_pack_refs(args: &[String]) -> CliResult<()> {
    const SPEC: &[OptDef] = &[
        o("all", ' ', No, true),
        o("prune", ' ', No, true),
        o("include", ' ', Req, true),
        o("exclude", ' ', Req, true),
    ];
    let items = opt_parse::parse("pack-refs", args, SPEC, opt_parse::Mode::default())?;
    if !items.operands().is_empty() {
        return Err(CliError::Exit(
            129,
            "usage: git pack-refs [--all] [--no-prune] [--include <pattern>] [--exclude <pattern>]"
                .into(),
        ));
    }
    let all = items.on("all");
    let prune = items.flag("prune").unwrap_or(true);
    let (mut includes, excludes) = (items.values("include"), items.values("exclude"));
    if includes.is_empty() {
        includes.push("refs/tags/*".into());
    }
    let mut repo = open_repo()?;
    if repo.refs().is_reftable() {
        let _ = repo.refs_mut().pack_refs_all()?;
        return Ok(());
    }
    // REF-012 (M20 Cx-3): pack-refs rewrites packed-refs under one exclusive lock.
    let packed_path = repo.common_dir().join("packed-refs");
    let lock = repo.common_dir().join("packed-refs.lock");
    let mut lock_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|_| {
            CliError::fatal(format!(
                "Unable to create '{}': File exists. (REF-012 — packed-refs.lock held)",
                lock.display()
            ))
        })?;
    let result = (|| -> CliResult<()> {
        let mut packed: std::collections::BTreeMap<String, Oid> = Default::default();
        if let Ok(text) = fs::read_to_string(&packed_path) {
            for line in text.lines() {
                if line.starts_with('#') || line.starts_with('^') {
                    continue;
                }
                if let Some((hex, name)) = line.split_once(' ') {
                    if let Ok(o) = Oid::from_hex(hex) {
                        packed.insert(name.to_string(), o);
                    }
                }
            }
        }
        // Loose refs under the shared refs/ tree (symbolic refs stay loose).
        let mut loose: Vec<(String, Oid, PathBuf)> = Vec::new();
        let root = repo.common_dir().to_path_buf();
        let mut stack = vec![root.join("refs")];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let Ok(rel) = p.strip_prefix(&root) else {
                    continue;
                };
                let name = rel.to_string_lossy().replace('\\', "/");
                if name.ends_with(".lock") {
                    continue;
                }
                let Ok(text) = fs::read_to_string(&p) else {
                    continue;
                };
                if let Ok(o) = Oid::from_hex(text.trim()) {
                    loose.push((name, o, p));
                }
            }
        }
        let matches = |pats: &[String], name: &str| {
            pats.iter()
                .any(|pat| crate::cmd::ignore::wildmatch(pat.as_bytes(), name.as_bytes()))
        };
        let mut pruned = Vec::new();
        for (name, oid, path) in loose {
            let per_worktree = ["refs/bisect/", "refs/worktree/", "refs/rewritten/"]
                .iter()
                .any(|p| name.starts_with(p));
            if per_worktree || matches(&excludes, &name) || !(all || matches(&includes, &name)) {
                continue;
            }
            packed.insert(name, oid);
            pruned.push(path);
        }
        let mut body = String::from("# pack-refs with: peeled fully-peeled sorted \n");
        for (name, oid) in &packed {
            body.push_str(&format!("{} {name}\n", oid.to_hex()));
            let mut cur = *oid;
            let mut peeled = None;
            while let Ok(obj) = repo.get_object(&cur) {
                if obj.kind != ObjectKind::Tag {
                    break;
                }
                match tag_header(&obj.payload).0 {
                    Some(t) => {
                        cur = t;
                        peeled = Some(t);
                    }
                    None => break,
                }
            }
            if let Some(p) = peeled {
                body.push_str(&format!("^{}\n", p.to_hex()));
            }
        }
        use std::io::Write;
        lock_file.write_all(body.as_bytes())?;
        lock_file.sync_all()?;
        fs::rename(&lock, &packed_path)?;
        if prune {
            for p in pruned {
                let _ = fs::remove_file(p);
            }
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&lock);
    }
    result
}

/// Every reflog under `logs/` (common dir) plus the per-worktree `HEAD` log,
/// as ref names (`HEAD`, `refs/...`), whether or not the ref still exists.
pub(crate) fn all_reflog_names(repo: &Repository) -> Vec<String> {
    fn walk(dir: &std::path::Path, prefix: &str, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().to_string();
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let path = ent.path();
            if path.is_dir() {
                walk(&path, &rel, out);
            } else {
                out.push(rel);
            }
        }
    }
    let mut names = Vec::new();
    walk(
        &repo.common_dir().join("logs").join("refs"),
        "refs",
        &mut names,
    );
    names.sort();
    names.insert(0, "HEAD".to_string());
    names
}

#[cfg(test)]
mod tests {
    use super::ObjectTable;
    use arks_oid::Oid;

    fn oid(prefix: &str) -> Oid {
        Oid::from_hex(&format!("{prefix:0<40}")).unwrap()
    }

    fn prefixes(t: &ObjectTable) -> Vec<String> {
        t.order().map(|o| o.to_hex()[..4].to_string()).collect()
    }

    #[test]
    fn table_probes_from_the_little_endian_prefix() {
        // 0x0201.. and 0x0101.. differ only in byte 1, so both start at
        // slot 1 of a 32-slot table: insertion order decides.
        let mut t = ObjectTable::default();
        for p in ["0102", "0101", "05"] {
            t.look(oid(p));
        }
        assert_eq!(prefixes(&t), ["0102", "0101", "0500"]);
    }

    #[test]
    fn lookup_swaps_a_displaced_entry_home() {
        let mut t = ObjectTable::default();
        t.look(oid("0102"));
        t.look(oid("0101"));
        t.look(oid("0101"));
        assert_eq!(prefixes(&t), ["0101", "0102"]);
    }

    #[test]
    fn growth_doubles_past_half_load() {
        let mut t = ObjectTable::default();
        // 16 entries fill a 32-slot table to half load; the 17th doubles it.
        for i in 0..17u8 {
            t.look(oid(&format!("{i:02x}")));
        }
        assert_eq!(t.slots.len(), 64);
        // 0x21 shares slot 1 with 0x01 in 32 slots, not in 64.
        t.look(oid("21"));
        assert_eq!(t.at[&oid("21")], 33);
    }
}
