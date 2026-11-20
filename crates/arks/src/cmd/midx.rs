//! `multi-pack-index` — write, verify, expire and repack with a MIDX.
//!
//! The file lists the packs (PNAM), every object once (OIDF, OIDL) with its
//! pack and offset (OOFF, LOFF) and, with `--bitmap`, the pseudo-pack order
//! (RIDX) plus `multi-pack-index-<checksum>.bitmap`.
//! Cite: <https://git-scm.com/docs/git-multi-pack-index>,
//! <https://git-scm.com/docs/gitformat-pack> (multi-pack-index files).

use crate::cmd::opt_parse::{Mode, OptDef, Takes::*, o, parse};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_hash::ObjectFormat;
use arks_oid::Oid;
use arks_pack::PackIndex;
use arks_repo::Repository;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

const USAGE: &str = "usage: git multi-pack-index [<options>] write [--preferred-pack=<pack>][--refs-snapshot=<path>]
   or: git multi-pack-index [<options>] verify
   or: git multi-pack-index [<options>] expire
   or: git multi-pack-index [<options>] repack [--batch-size=<size>]

    --object-dir <directory>
                          object directory containing set of packfile and pack-index pairs
";

const LARGE: u32 = 0x8000_0000;

const OBJDIR_HELP: &str = "    --object-dir <directory>
                          object directory containing set of packfile and pack-index pairs
";

/// The usage block of one subcommand (`-h` and argument errors).
fn sub_usage(sub: &str) -> String {
    let (line, extra) = match sub {
        "write" => (
            "write [--preferred-pack=<pack>][--refs-snapshot=<path>]",
            "    --preferred-pack <preferred-pack>
                          pack for reuse when computing a multi-pack bitmap
    --bitmap              write multi-pack bitmap
    --progress            force progress reporting
    --stdin-packs         write multi-pack index containing only given indexes
    --refs-snapshot <file>
                          refs snapshot for selecting bitmap commits
",
        ),
        "repack" => (
            "repack [--batch-size=<size>]",
            "    --batch-size <n>      during repack, collect pack-files of smaller size into a batch that is larger than this size
    --progress            force progress reporting
",
        ),
        other => (
            if other == "verify" { "verify" } else { "expire" },
            "    --progress            force progress reporting
",
        ),
    };
    format!("usage: git multi-pack-index [<options>] {line}\n\n{OBJDIR_HELP}{extra}")
}

/// `<n>[kmg]` byte counts.
fn parse_size(v: &str) -> Option<u64> {
    let (num, mul) = match v.chars().last()? {
        'k' | 'K' => (&v[..v.len() - 1], 1024),
        'm' | 'M' => (&v[..v.len() - 1], 1024 * 1024),
        'g' | 'G' => (&v[..v.len() - 1], 1024 * 1024 * 1024),
        _ => (v, 1),
    };
    num.parse::<u64>().ok().map(|n| n * mul)
}

fn usage_err(prefix: &str) -> CliError {
    CliError::Exit(129, format!("{prefix}{USAGE}"))
}

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
                    "could not find object directory matching {}",
                    full.display()
                )));
            }
            Ok(full)
        }
    }
}

pub fn cmd_multi_pack_index(args: &[String]) -> CliResult<()> {
    let mut obj_dir: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-h" {
            print!("{USAGE}");
            println!();
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
        } else if a == "--progress" || a == "--no-progress" {
        } else if a.starts_with('-') {
            return Err(usage_err(&format!(
                "error: unknown option `{}'\n",
                a.trim_start_matches('-')
            )));
        } else {
            break;
        }
        i += 1;
    }
    let Some(sub) = args.get(i) else {
        return Err(usage_err("error: need a subcommand\n"));
    };
    let rest = &args[i + 1..];
    if rest.len() == 1
        && rest[0] == "-h"
        && matches!(sub.as_str(), "write" | "verify" | "expire" | "repack")
    {
        println!("{}", sub_usage(sub));
        return Err(CliError::silent(129));
    }
    let repo = open_repo()?;
    let defs: &[OptDef] = match sub.as_str() {
        "write" => &[
            o("object-dir", ' ', Req, true),
            o("preferred-pack", ' ', Req, true),
            o("bitmap", ' ', No, true),
            o("progress", ' ', No, true),
            o("stdin-packs", ' ', No, true),
            o("refs-snapshot", ' ', Req, true),
        ],
        "verify" | "expire" => &[
            o("object-dir", ' ', Req, true),
            o("progress", ' ', No, true),
        ],
        "repack" => &[
            o("object-dir", ' ', Req, true),
            o("batch-size", ' ', Req, false),
            o("progress", ' ', No, true),
        ],
        other => {
            return Err(usage_err(&format!(
                "error: unknown subcommand: `{other}'\n"
            )));
        }
    };
    let p = parse("multi-pack-index", rest, defs, Mode::default())?;
    if !p.args().is_empty() {
        return Err(CliError::Exit(129, sub_usage(sub)));
    }
    let dir = object_dir(&repo, p.value("object-dir").or(obj_dir).as_deref())?;
    match sub.as_str() {
        "write" => {
            let only: Option<Vec<String>> = if p.on("stdin-packs") {
                let mut v = Vec::new();
                for line in io::stdin().lock().lines() {
                    let line = line?;
                    let l = line.trim();
                    if !l.is_empty() {
                        v.push(l.to_string());
                    }
                }
                Some(v)
            } else {
                None
            };
            write(
                &repo,
                &dir,
                only.as_deref(),
                p.value("preferred-pack").as_deref(),
                p.on("bitmap"),
            )
        }
        "verify" => verify(&repo, &dir),
        "expire" => expire(&repo, &dir),
        _ => {
            let batch = match p.value("batch-size") {
                None => 0,
                Some(v) => parse_size(&v).ok_or_else(|| {
                    CliError::Exit(129, "error: option `batch-size' expects a non-negative integer value with an optional k/m/g suffix".into())
                })?,
            };
            repack(&repo, &dir, batch)
        }
    }
}

struct Pack {
    /// `pack-<hash>.idx`
    name: String,
    index: PackIndex,
    mtime: u64,
    size: u64,
    keep: bool,
}

fn idx_name(n: &str) -> String {
    let base = n
        .strip_suffix(".idx")
        .or_else(|| n.strip_suffix(".pack"))
        .unwrap_or(n);
    format!("{base}.idx")
}

fn load_packs(repo: &Repository, dir: &Path) -> Vec<Pack> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir.join("pack")) else {
        return out;
    };
    for e in rd.flatten() {
        let path = e.path();
        if path.extension().is_none_or(|x| x != "idx") || !path.with_extension("pack").is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        let Ok(index) = PackIndex::decode(&bytes, repo.object_format()) else {
            continue;
        };
        let meta = fs::metadata(path.with_extension("pack")).ok();
        out.push(Pack {
            name: e.file_name().to_string_lossy().into_owned(),
            index,
            mtime: meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs()),
            size: meta.map_or(0, |m| m.len()),
            keep: path.with_extension("keep").is_file(),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn hash_id(f: ObjectFormat) -> u8 {
    match f {
        ObjectFormat::Sha1 => 1,
        ObjectFormat::Sha256 => 2,
    }
}

/// Objects of the MIDX: (oid, pack index, offset), sorted by oid.
fn select(packs: &[Pack], preferred: Option<usize>) -> Vec<(Oid, u32, u64)> {
    // Preferred pack first, then newer packs.
    let mut rank: Vec<usize> = (0..packs.len()).collect();
    rank.sort_by(|a, b| {
        (Some(*b) == preferred)
            .cmp(&(Some(*a) == preferred))
            .then(packs[*b].mtime.cmp(&packs[*a].mtime))
            .then(a.cmp(b))
    });
    let mut chosen: HashMap<Vec<u8>, (Oid, u32, u64)> = HashMap::new();
    for &pi in &rank {
        for e in &packs[pi].index.entries {
            chosen
                .entry(e.oid.as_bytes().to_vec())
                .or_insert((e.oid, pi as u32, e.offset));
        }
    }
    let mut v: Vec<(Oid, u32, u64)> = chosen.into_values().collect();
    v.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    v
}

fn write(
    repo: &Repository,
    dir: &Path,
    only: Option<&[String]>,
    preferred: Option<&str>,
    bitmap: bool,
) -> CliResult<()> {
    let mut packs = load_packs(repo, dir);
    if let Some(list) = only {
        let want: HashSet<String> = list.iter().map(|n| idx_name(n)).collect();
        packs.retain(|p| want.contains(&p.name));
    }
    if packs.is_empty() {
        return Err(CliError::Exit(1, "error: no pack files to index.".into()));
    }
    let pref = match preferred {
        None => None,
        Some(n) => {
            let i = packs.iter().position(|p| p.name == idx_name(n));
            if i.is_none() {
                eprintln!("warning: unknown preferred pack: '{n}'");
            }
            i
        }
    };
    let format = repo.object_format();
    let objects = select(&packs, pref);
    let mut chunks: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    let mut pnam = Vec::new();
    for p in &packs {
        pnam.extend_from_slice(p.name.as_bytes());
        pnam.push(0);
    }
    while pnam.len() % 4 != 0 {
        pnam.push(0);
    }
    chunks.push((*b"PNAM", pnam));
    let mut fan = vec![0u8; 1024];
    let mut counts = [0u32; 256];
    for (o, _, _) in &objects {
        counts[o.as_bytes()[0] as usize] += 1;
    }
    let mut acc = 0u32;
    for (i, c) in counts.iter().enumerate() {
        acc += c;
        fan[i * 4..i * 4 + 4].copy_from_slice(&acc.to_be_bytes());
    }
    chunks.push((*b"OIDF", fan));
    chunks.push((
        *b"OIDL",
        objects
            .iter()
            .flat_map(|(o, _, _)| o.as_bytes().to_vec())
            .collect(),
    ));
    let mut ooff = Vec::new();
    let mut loff = Vec::new();
    for (_, pi, off) in &objects {
        ooff.extend_from_slice(&pi.to_be_bytes());
        if *off >= u64::from(LARGE) {
            let i = (loff.len() / 8) as u32;
            loff.extend_from_slice(&off.to_be_bytes());
            ooff.extend_from_slice(&(LARGE | i).to_be_bytes());
        } else {
            ooff.extend_from_slice(&(*off as u32).to_be_bytes());
        }
    }
    chunks.push((*b"OOFF", ooff));
    if !loff.is_empty() {
        chunks.push((*b"LOFF", loff));
    }
    // Pseudo-pack order: preferred pack first, then pack order, then offset.
    let pref_id = pref.map(|p| p as u32);
    let mut pseudo: Vec<usize> = (0..objects.len()).collect();
    pseudo.sort_by_key(|i| {
        let (_, pi, off) = objects[*i];
        (Some(pi) != pref_id, pi, off)
    });
    if bitmap {
        let ridx: Vec<u8> = pseudo
            .iter()
            .flat_map(|i| (*i as u32).to_be_bytes())
            .collect();
        chunks.push((*b"RIDX", ridx));
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"MIDX");
    out.push(1);
    out.push(hash_id(format));
    out.push(chunks.len() as u8);
    out.push(0);
    out.extend_from_slice(&(packs.len() as u32).to_be_bytes());
    let mut off = (12 + (chunks.len() + 1) * 12) as u64;
    for (id, body) in &chunks {
        out.extend_from_slice(id);
        out.extend_from_slice(&off.to_be_bytes());
        off += body.len() as u64;
    }
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&off.to_be_bytes());
    for (_, body) in &chunks {
        out.extend_from_slice(body);
    }
    arks_pack::trailer(&mut out, format);
    let checksum = out[out.len() - format.digest_len()..].to_vec();
    let pack_dir = dir.join("pack");
    // Old bitmaps belong to the old MIDX.
    if let Ok(rd) = fs::read_dir(&pack_dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with("multi-pack-index-") {
                let _ = fs::remove_file(e.path());
            }
        }
    }
    fs::write(pack_dir.join("multi-pack-index"), &out)?;
    if bitmap {
        let mut order = Vec::with_capacity(pseudo.len());
        for &i in &pseudo {
            let oid = objects[i].0;
            let kind = repo.get_object(&oid).map(|o| o.kind)?;
            order.push((oid, kind, i));
        }
        let bytes = crate::cmd::pack_bitmap::encode_ordered(repo, &order, &checksum)?;
        let hex: String = checksum.iter().map(|b| format!("{b:02x}")).collect();
        fs::write(
            pack_dir.join(format!("multi-pack-index-{hex}.bitmap")),
            bytes,
        )?;
    }
    Ok(())
}

/// A parsed MIDX: pack names and (oid, pack, offset) records.
struct Midx {
    packs: Vec<String>,
    objects: Vec<(Oid, u32, u64)>,
}

fn read_midx(repo: &Repository, dir: &Path) -> Result<Option<Midx>, String> {
    let path = dir.join("pack/multi-pack-index");
    let Ok(d) = fs::read(&path) else {
        return Ok(None);
    };
    let format = repo.object_format();
    let hl = format.digest_len();
    if d.len() < 12 + hl || &d[..4] != b"MIDX" {
        return Err("multi-pack-index signature does not match".into());
    }
    let body = &d[..d.len() - hl];
    if arks_hash::hash_bytes(format, body)
        .map(|x| x.as_bytes() != &d[d.len() - hl..])
        .unwrap_or(true)
    {
        return Err("incorrect checksum".into());
    }
    let n_chunks = d[6] as usize;
    let mut table = Vec::new();
    for i in 0..=n_chunks {
        let at = 12 + i * 12;
        let id: [u8; 4] = d[at..at + 4].try_into().map_err(|_| "truncated")?;
        let off =
            u64::from_be_bytes(d[at + 4..at + 12].try_into().map_err(|_| "truncated")?) as usize;
        table.push((id, off));
    }
    let chunk = |id: &[u8; 4]| -> Option<&[u8]> {
        let i = table.iter().position(|(c, _)| c == id)?;
        d.get(table[i].1..table.get(i + 1)?.1)
    };
    let pnam =
        chunk(b"PNAM").ok_or("multi-pack-index required pack-name chunk missing or corrupted")?;
    let packs: Vec<String> = pnam
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    let oidl =
        chunk(b"OIDL").ok_or("multi-pack-index required OID lookup chunk missing or corrupted")?;
    let ooff = chunk(b"OOFF")
        .ok_or("multi-pack-index required object offsets chunk missing or corrupted")?;
    let loff = chunk(b"LOFF").unwrap_or(&[]);
    let kind = match format {
        ObjectFormat::Sha1 => arks_oid::OidKind::Sha1,
        ObjectFormat::Sha256 => arks_oid::OidKind::Sha256,
    };
    let mut objects = Vec::new();
    for (i, raw) in oidl.chunks(hl).enumerate() {
        let oid = Oid::from_bytes(kind, raw).map_err(|e| e.to_string())?;
        let rec = ooff
            .get(i * 8..i * 8 + 8)
            .ok_or("bad object offsets chunk")?;
        let pi = u32::from_be_bytes(rec[..4].try_into().map_err(|_| "bad")?);
        let mut off = u64::from(u32::from_be_bytes(rec[4..].try_into().map_err(|_| "bad")?));
        if off & u64::from(LARGE) != 0 {
            let li = (off & 0x7fff_ffff) as usize;
            off = u64::from_be_bytes(
                loff.get(li * 8..li * 8 + 8)
                    .and_then(|s| s.try_into().ok())
                    .ok_or("bad large offset")?,
            );
        }
        objects.push((oid, pi, off));
    }
    Ok(Some(Midx { packs, objects }))
}

fn verify(repo: &Repository, dir: &Path) -> CliResult<()> {
    let midx = match read_midx(repo, dir) {
        Ok(Some(m)) => m,
        Ok(None) => return Ok(()),
        Err(e) => return Err(CliError::Exit(1, e)),
    };
    let packs = load_packs(repo, dir);
    let mut errs = Vec::new();
    for w in midx.objects.windows(2) {
        if w[0].0.as_bytes() >= w[1].0.as_bytes() {
            errs.push(format!(
                "oid lookup out of order: oid[{}] = {} >= {} = oid[{}]",
                0,
                w[0].0.to_hex(),
                w[1].0.to_hex(),
                1
            ));
        }
    }
    for (oid, pi, off) in &midx.objects {
        let Some(name) = midx.packs.get(*pi as usize) else {
            errs.push(format!("failed to load pack in position {pi}"));
            continue;
        };
        let Some(pk) = packs.iter().find(|p| &p.name == name) else {
            errs.push(format!("failed to load pack-index for packfile {name}"));
            continue;
        };
        match pk.index.lookup(oid) {
            Some(e) if e.offset == *off => {}
            _ => errs.push(format!(
                "incorrect object offset for oid[{}] = {}: {off:x} != {:x}",
                0,
                oid.to_hex(),
                pk.index.lookup(oid).map_or(0, |e| e.offset)
            )),
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(CliError::Exit(
            1,
            errs.iter()
                .map(|e| format!("error: {e}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ))
    }
}

fn expire(repo: &Repository, dir: &Path) -> CliResult<()> {
    let midx = match read_midx(repo, dir) {
        Ok(Some(m)) => m,
        Ok(None) => return Ok(()),
        Err(e) => return Err(CliError::Exit(1, format!("error: {e}"))),
    };
    let used: HashSet<u32> = midx.objects.iter().map(|(_, p, _)| *p).collect();
    let packs = load_packs(repo, dir);
    let mut removed = false;
    for (i, name) in midx.packs.iter().enumerate() {
        if used.contains(&(i as u32)) {
            continue;
        }
        let Some(pk) = packs.iter().find(|p| &p.name == name) else {
            continue;
        };
        if pk.keep {
            continue;
        }
        let base = dir.join("pack").join(name.trim_end_matches(".idx"));
        for ext in ["pack", "idx", "rev", "bitmap", "mtimes", "promisor"] {
            let _ = fs::remove_file(base.with_extension(ext));
        }
        removed = true;
    }
    if removed {
        write(repo, dir, None, None, false)?;
    }
    Ok(())
}

fn repack(repo: &Repository, dir: &Path, batch: u64) -> CliResult<()> {
    let midx = match read_midx(repo, dir) {
        Ok(Some(m)) => m,
        Ok(None) => return Ok(()),
        Err(e) => return Err(CliError::Exit(1, format!("error: {e}"))),
    };
    let packs = load_packs(repo, dir);
    let in_midx: Vec<&Pack> = packs
        .iter()
        .filter(|p| midx.packs.contains(&p.name) && !p.keep)
        .collect();
    let mut chosen: Vec<&Pack> = Vec::new();
    if batch == 0 {
        if in_midx.len() < 2 {
            return Ok(());
        }
        chosen = in_midx;
    } else {
        let mut by_size = in_midx;
        by_size.sort_by_key(|p| p.size);
        let mut total = 0;
        for p in by_size {
            if total + p.size > batch && chosen.len() >= 2 {
                break;
            }
            total += p.size;
            chosen.push(p);
        }
        if chosen.len() < 2 || total < batch {
            return Ok(());
        }
    }
    let names: HashSet<&str> = chosen.iter().map(|p| p.name.as_str()).collect();
    let mut list = String::new();
    for (oid, pi, _) in &midx.objects {
        if names.contains(midx.packs[*pi as usize].as_str()) {
            list.push_str(&oid.to_hex());
            list.push('\n');
        }
    }
    // A new pack of the chosen objects, written in-process.
    let mut wants = Vec::new();
    for line in list.lines() {
        if let Ok(oid) = Oid::from_hex(line) {
            if let Ok(obj) = repo.get_object(&oid) {
                wants.push((oid, obj));
            }
        }
    }
    let format = repo.object_format();
    let mut writer = arks_pack::PackWriter::new(format, 0, None);
    for (oid, obj) in &wants {
        writer.add_full(*oid, obj.kind, &obj.payload)?;
    }
    let pk = writer.finish();
    let hex: String = pk.checksum.iter().map(|b| format!("{b:02x}")).collect();
    let index = PackIndex::from_entries(format, pk.entries.clone(), pk.checksum.clone())?;
    let stem = dir.join("pack").join(format!("pack-{hex}"));
    fs::write(stem.with_extension("pack"), &pk.pack)?;
    fs::write(stem.with_extension("idx"), index.encode()?)?;
    write(repo, dir, None, None, false)
}
