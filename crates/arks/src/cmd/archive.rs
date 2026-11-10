//! Tar archive of a tree.

use crate::cmd::shared::{commit_signature, resolve_rev};
use crate::error::{CliError, CliResult};
use crate::util::{FORMAT, open_repo, refuse_unknown_flags};
use arks_object::{Commit, ObjectKind, RawObject};
use arks_oid::Oid;
use arks_repo::{Repository, flatten_tree_entries};
use arks_store::ObjectStore;
use std::fs;
use std::io::{self, BufWriter, Write};
use std::thread;
pub fn cmd_archive(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["--format=tar", "--format", "-o"])?;
    let mut format = "tar".to_owned();
    let mut output: Option<String> = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--format" {
            i += 1;
            format = args
                .get(i)
                .ok_or_else(|| CliError::Usage("archive: --format requires a value".into()))?
                .clone();
        } else if let Some(rest) = a.strip_prefix("--format=") {
            format = rest.to_owned();
        } else if a == "-o" {
            i += 1;
            output = Some(
                args.get(i)
                    .ok_or_else(|| CliError::Usage("archive: -o requires a path".into()))?
                    .clone(),
            );
        } else if a.starts_with('-') {
            return Err(CliError::NotImplemented(format!(
                "unsupported archive option: {a}"
            )));
        } else {
            positional.push(a.clone());
        }
        i += 1;
    }
    if format != "tar" {
        return Err(CliError::NotImplemented(format!(
            "archive format '{format}' not claimed (tar only)"
        )));
    }
    let rev = positional.first().map(String::as_str).unwrap_or("HEAD");
    let repo = open_repo()?;
    let oid = resolve_rev(&repo, rev)?;
    let tree_oid = tree_for_rev(&repo, &oid)?;
    // Pf4-2: flatten straight into Vec, sort once; prefetch blobs in parallel, then stream.
    let mut entries: Vec<(String, Oid)> = Vec::new();
    flatten_tree_entries(&repo, &tree_oid, "", &mut entries)
        .map_err(|e| CliError::Msg(e.to_string()))?;
    entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let payloads = prefetch_blobs(&repo, &entries)?;
    // Stream ustar (M17 Pf3-2): avoid buffering the whole archive in RAM.
    let write_archive = |w: &mut dyn Write| -> CliResult<()> {
        for ((path, _), payload) in entries.iter().zip(payloads.iter()) {
            if let Some(bytes) = payload {
                write_ustar(w, path, bytes, 0o644)?;
            }
        }
        w.write_all(&[0u8; 1024])?;
        Ok(())
    };
    // Pf4-1: BufWriter for ustar emit (ARCHIVE G19 edge ratio).
    if let Some(path) = output {
        // MISC-002 (M20 Cx-5): parallel archive to same occupied prefix refuse.
        let outp = std::path::Path::new(&path);
        if outp.exists() {
            return Err(CliError::Msg(format!(
                "archive output '{}' exists (MISC-002 — parallel archive/checkout-index to same prefix refuse; choose a unique path)",
                outp.display()
            )));
        }
        let f = fs::File::create(path)?;
        let mut buf = BufWriter::with_capacity(256 * 1024, f);
        write_archive(&mut buf)?;
        buf.flush()?;
    } else {
        let mut out = BufWriter::with_capacity(256 * 1024, io::stdout().lock());
        write_archive(&mut out)?;
        out.flush()?;
    }
    Ok(())
}

fn prefetch_blobs(
    repo: &Repository,
    entries: &[(String, Oid)],
) -> CliResult<Vec<Option<Vec<u8>>>> {
    let n = entries.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    let workers = if n < 64 {
        1
    } else {
        thread::available_parallelism()
            .map(|p| p.get().clamp(2, 8))
            .unwrap_or(4)
            .min(n)
    };
    if workers <= 1 {
        let mut out = Vec::with_capacity(n);
        for (_, oid) in entries {
            let obj = repo.get_object(oid)?;
            out.push(if obj.kind == ObjectKind::Blob {
                Some(obj.payload)
            } else {
                None
            });
        }
        return Ok(out);
    }
    let chunk = n.div_ceil(workers);
    let mut out: Vec<Option<Vec<u8>>> = vec![None; n];
    let mut worker_err: Option<CliError> = None;
    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for (batch_idx, piece) in entries.chunks(chunk).enumerate() {
            let base = batch_idx * chunk;
            handles.push(scope.spawn(move || -> CliResult<Vec<(usize, Option<Vec<u8>>)>> {
                let mut local = Vec::with_capacity(piece.len());
                for (i, (_, oid)) in piece.iter().enumerate() {
                    let obj = repo.get_object(oid)?;
                    let payload = if obj.kind == ObjectKind::Blob {
                        Some(obj.payload)
                    } else {
                        None
                    };
                    local.push((base + i, payload));
                }
                Ok(local)
            }));
        }
        for h in handles {
            match h.join() {
                Ok(Ok(local)) => {
                    for (i, payload) in local {
                        out[i] = payload;
                    }
                }
                Ok(Err(e)) => {
                    worker_err = Some(e);
                    break;
                }
                Err(_) => {
                    worker_err = Some(CliError::Msg("archive: worker panicked".into()));
                    break;
                }
            }
        }
    });
    if let Some(e) = worker_err {
        return Err(e);
    }
    Ok(out)
}

fn tree_for_rev(repo: &Repository, oid: &Oid) -> CliResult<Oid> {
    let obj = repo.get_object(oid)?;
    match obj.kind {
        ObjectKind::Commit => {
            let c = Commit::decode(&obj.payload)?;
            Ok(c.tree)
        }
        ObjectKind::Tree => Ok(*oid),
        ObjectKind::Tag => {
            let text = std::str::from_utf8(&obj.payload)
                .map_err(|_| CliError::Msg("tag payload not utf-8".into()))?;
            let mut target = None;
            let mut typ = None;
            for line in text.lines() {
                if line.is_empty() {
                    break;
                }
                if let Some(rest) = line.strip_prefix("object ") {
                    target = Some(Oid::from_hex(rest.trim())?);
                } else if let Some(rest) = line.strip_prefix("type ") {
                    typ = Some(rest.trim().to_owned());
                }
            }
            let target = target.ok_or_else(|| CliError::Msg("tag missing object".into()))?;
            if typ.as_deref() == Some("commit") || typ.is_none() {
                tree_for_rev(repo, &target)
            } else if typ.as_deref() == Some("tree") {
                Ok(target)
            } else {
                Err(CliError::Msg("tag does not peel to a tree".into()))
            }
        }
        ObjectKind::Blob => Err(CliError::Msg("cannot archive a blob".into())),
    }
}

fn write_ustar(out: &mut dyn Write, path: &str, data: &[u8], mode: u32) -> CliResult<()> {
    if path.len() > 100 {
        return Err(CliError::NotImplemented(
            "archive: paths longer than 100 bytes not claimed".into(),
        ));
    }
    let mut header = [0u8; 512];
    header[..path.len()].copy_from_slice(path.as_bytes());
    // mode (octal)
    write_octal(&mut header[100..108], mode as u64);
    write_octal(&mut header[108..116], 0); // uid
    write_octal(&mut header[116..124], 0); // gid
    write_octal(&mut header[124..136], data.len() as u64);
    write_octal(&mut header[136..148], 0); // mtime
    // checksum field filled with spaces while computing
    header[148..156].fill(b' ');
    header[156] = b'0'; // regular file
    header[257..262].copy_from_slice(b"ustar");
    header[262] = 0;
    header[263..265].copy_from_slice(b"00");
    let sum: u32 = header.iter().map(|b| *b as u32).sum();
    write_octal(&mut header[148..156], sum as u64);
    out.write_all(&header)?;
    out.write_all(data)?;
    let pad = (512 - (data.len() % 512)) % 512;
    if pad > 0 {
        out.write_all(&[0u8; 512][..pad])?;
    }
    Ok(())
}

fn write_octal(dst: &mut [u8], value: u64) {
    // NUL-terminated octal; fill with '0' then write digits (Pf4-2: no format! alloc).
    dst.fill(b'0');
    let end = dst.len().saturating_sub(1);
    if end == 0 {
        return;
    }
    let mut v = value;
    let mut i = end;
    if v == 0 {
        dst[end - 1] = b'0';
        dst[end] = 0;
        return;
    }
    while v > 0 && i > 0 {
        i -= 1;
        dst[i] = b'0' + (v % 8) as u8;
        v /= 8;
    }
    dst[end] = 0;
}

/// Build annotated tag object payload bytes.
/// Build annotated tag object payload bytes.
pub fn encode_tag_payload(
    object: &Oid,
    object_type: &str,
    tag: &str,
    tagger_line: &str,
    message: &str,
) -> Vec<u8> {
    let mut out = String::new();
    out.push_str("object ");
    out.push_str(&object.to_hex());
    out.push('\n');
    out.push_str("type ");
    out.push_str(object_type);
    out.push('\n');
    out.push_str("tag ");
    out.push_str(tag);
    out.push('\n');
    out.push_str("tagger ");
    out.push_str(tagger_line);
    out.push('\n');
    out.push('\n');
    out.push_str(message);
    if !message.ends_with('\n') {
        out.push('\n');
    }
    out.into_bytes()
}

pub fn write_annotated_tag(
    repo: &mut Repository,
    name: &str,
    target: &Oid,
    message: &str,
) -> CliResult<Oid> {
    let sig = commit_signature()?;
    let payload = encode_tag_payload(target, "commit", name, &sig.format_line(), message);
    let obj = RawObject::new(ObjectKind::Tag, payload);
    let tag_oid = repo.store_mut().put(FORMAT, &obj)?;
    repo.refs_mut()
        .update(&format!("refs/tags/{name}"), &tag_oid, None)?;
    Ok(tag_oid)
}
