//! Ancillary interrogators: fsck / count-objects / pack-refs (blame lives in `blame`).
//!
//! Cite: <https://git-scm.com/docs/git-fsck>
//! Cite: <https://git-scm.com/docs/git-count-objects>
//! Cite: <https://git-scm.com/docs/git-pack-refs>
//! Output order and wording follow observed stock git (family-plumbing corpus).

use crate::cmd::optparse::{self, Opt};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, Tree};
use arks_oid::Oid;
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Every `.idx` in `objects/pack` (sorted by name) with its object names.
fn pack_indexes(repo: &Repository) -> Vec<(PathBuf, Vec<Oid>)> {
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
            Some((p, index.entries.iter().map(|e| e.oid).collect()))
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

/// Order in which git's in-memory object hash table yields `inserted` (in
/// insertion order): open addressing on the first four name bytes (little
/// endian), table size the smallest power of two (at least 32) holding the
/// objects at half load.
fn object_table_order(inserted: &[Oid]) -> Vec<Oid> {
    let mut size = 32usize;
    while inserted.len() > size / 2 {
        size *= 2;
    }
    let mut slots: Vec<Option<Oid>> = vec![None; size];
    for &o in inserted {
        let b = o.as_bytes();
        let mut i = u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize & (size - 1);
        while slots[i].is_some() {
            i = (i + 1) & (size - 1);
        }
        slots[i] = Some(o);
    }
    slots.into_iter().flatten().collect()
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
    const SPEC: &[Opt] = &[
        Opt::flag("verbose").with_short('v'),
        Opt::flag("unreachable"),
        Opt::flag("dangling"),
        Opt::flag("tags"),
        Opt::flag("root"),
        Opt::flag("cache"),
        Opt::flag("reflogs"),
        Opt::flag("full"),
        Opt::flag("connectivity-only"),
        Opt::flag("strict"),
        Opt::flag("lost-found"),
        Opt::flag("progress"),
        Opt::flag("name-objects"),
    ];
    let items = optparse::parse("fsck", args, SPEC, optparse::Mode::default())?;
    let verbose = optparse::on(&items, "verbose");
    let show_unreachable = optparse::on(&items, "unreachable");
    let show_dangling = optparse::flag(&items, "dangling").unwrap_or(true);
    let show_tags = optparse::on(&items, "tags");
    let show_root = optparse::on(&items, "root");
    let use_cache = optparse::on(&items, "cache");
    let lost_found = optparse::on(&items, "lost-found");
    // --lost-found implies --no-reflogs.
    let reflogs = optparse::flag(&items, "reflogs").unwrap_or(true) && !lost_found;
    let connectivity_only = optparse::on(&items, "connectivity-only");
    let heads = optparse::positionals(&items);

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
    let mut inserted: Vec<Oid> = Vec::new();
    let mut in_table: HashSet<Oid> = HashSet::new();
    let mut expected: HashMap<Oid, &'static str> = HashMap::new();
    let mut kinds: HashMap<Oid, ObjectKind> = HashMap::new();
    let mut corrupt: HashSet<Oid> = HashSet::new();
    // Observed table insertion order: packed objects, then loose objects
    // (exact placement of hash collisions is approximated).
    for o in packs
        .iter()
        .flat_map(|(_, oids)| oids.iter())
        .chain(loose.iter().map(|(o, _)| o))
    {
        if in_table.insert(*o) {
            inserted.push(*o);
        }
    }
    let copies = loose.iter().map(|(o, p)| (*o, Some(p.clone()))).chain(
        packs
            .iter()
            .flat_map(|(_, oids)| oids.iter().map(|o| (*o, None))),
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
        for (_name, oid) in repo.refs().list_refs("")? {
            roots.push(oid);
        }
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
        for (c, ty) in children(&oid) {
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
    for o in roots.iter().chain(missing.keys()) {
        if in_table.insert(*o) {
            inserted.push(*o);
        }
    }
    let mut dangling = Vec::new();
    for oid in object_table_order(&inserted) {
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
    const SPEC: &[Opt] = &[
        Opt::flag("verbose").with_short('v'),
        Opt::flag("human-readable").with_short('H'),
    ];
    let items = optparse::parse("count-objects", args, SPEC, optparse::Mode::default())?;
    let verbose = optparse::on(&items, "verbose");
    let hr = optparse::on(&items, "human-readable");
    if !optparse::positionals(&items).is_empty() {
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
    for (idx, oids) in &packs {
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
    const SPEC: &[Opt] = &[
        Opt::flag("all"),
        Opt::flag("prune"),
        Opt::value("include"),
        Opt::value("exclude"),
    ];
    let items = optparse::parse("pack-refs", args, SPEC, optparse::Mode::default())?;
    if !optparse::positionals(&items).is_empty() {
        return Err(CliError::Exit(
            129,
            "usage: git pack-refs [--all] [--no-prune] [--include <pattern>] [--exclude <pattern>]"
                .into(),
        ));
    }
    let all = optparse::on(&items, "all");
    let prune = optparse::flag(&items, "prune").unwrap_or(true);
    let (mut includes, mut excludes): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for it in &items {
        if let optparse::Item::Opt { name, value, neg } = it {
            let list = match *name {
                "include" => &mut includes,
                "exclude" => &mut excludes,
                _ => continue,
            };
            if *neg {
                list.clear();
            } else if let Some(v) = value {
                list.push(v.clone());
            }
        }
    }
    if includes.is_empty() {
        includes.push("refs/tags/*".into());
    }
    let repo = open_repo()?;
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
