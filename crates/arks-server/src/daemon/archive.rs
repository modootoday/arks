//! `git-upload-archive` over `git://` (ustar body, path filter, sideband).
//!
//! Primary: <https://git-scm.com/docs/git-upload-archive>,
//! <https://git-scm.com/docs/git-archive>.

use super::{parse_pkt_len, write_err_pkt};
use crate::error::ServerError;
use arks_object::{Commit, ObjectKind, Tree, TreeMode};
use arks_oid::Oid;
use arks_protocol::{Packet, encode_packet};
use arks_repo::Repository;
use std::io::{Read, Write};

/// Parsed upload-archive path-filter Partial (closer to git archive / daemon args).
#[derive(Debug, Default)]
struct ArchivePathFilter {
    prefix: String,
    includes: Vec<String>,
    excludes: Vec<String>,
}

fn parse_archive_path_filter(args: &[String]) -> Result<(String, ArchivePathFilter), String> {
    let mut filter = ArchivePathFilter::default();
    let mut treeish = None;
    let mut i = 0usize;
    while i < args.len() {
        let a = &args[i];
        if let Some(p) = a.strip_prefix("--prefix=") {
            filter.prefix = p.to_string();
        } else if a == "--prefix" {
            i += 1;
            filter.prefix = args
                .get(i)
                .cloned()
                .ok_or_else(|| "upload-archive --prefix needs value".to_string())?;
        } else if let Some(p) = a.strip_prefix("--exclude=") {
            filter.excludes.push(p.to_string());
        } else if a == "--exclude" {
            i += 1;
            filter.excludes.push(
                args.get(i)
                    .cloned()
                    .ok_or_else(|| "upload-archive --exclude needs value".to_string())?,
            );
        } else if a == "--format"
            || a.starts_with("--format=")
            || a == "--no-sideband"
            || a == "--raw-ustar"
        {
            // handled elsewhere
        } else if a.starts_with('-') {
            return Err(format!(
                "upload-archive Partial: unknown option `{a}` (not Claimed)"
            ));
        } else if treeish.is_none() {
            treeish = Some(a.clone());
        } else {
            // Additional non-option args = include pathspecs (git archive style Partial).
            filter.includes.push(a.clone());
        }
        i += 1;
    }
    Ok((treeish.unwrap_or_else(|| "HEAD".into()), filter))
}

fn path_allowed(path: &str, filter: &ArchivePathFilter) -> bool {
    if !filter.includes.is_empty() {
        let ok = filter.includes.iter().any(|inc| {
            path == inc.as_str()
                || path.starts_with(&format!("{inc}/"))
                || (inc.ends_with('/') && path.starts_with(inc.as_str()))
        });
        if !ok {
            return false;
        }
    }
    for ex in &filter.excludes {
        if path == ex.as_str()
            || path.starts_with(&format!("{ex}/"))
            || path.split('/').any(|seg| seg == ex)
        {
            return false;
        }
    }
    true
}

/// Resolve `--format=` honesty: only tar/ustar Partial; empty/others fail-closed.
fn archive_format_honesty(args: &[String]) -> Result<&'static str, String> {
    let fmt = args
        .iter()
        .find_map(|a| a.strip_prefix("--format=").map(str::to_string))
        .or_else(|| {
            args.windows(2).find_map(|w| {
                if w[0] == "--format" {
                    Some(w[1].clone())
                } else {
                    None
                }
            })
        });
    match fmt.as_deref() {
        None => Ok("tar"),
        Some("") => Err(
            "upload-archive Partial: --format= needs value (tar|ustar only; \
             zip/others Missing — design ≠ Claimed)"
                .into(),
        ),
        Some(f) => {
            let f = f.to_ascii_lowercase();
            if f == "tar" || f == "ustar" {
                Ok(if f == "ustar" { "ustar" } else { "tar" })
            } else {
                Err(format!(
                    "upload-archive Partial: --format={f} not wired (tar/ustar only; \
                     zip/others Missing — design ≠ Claimed)"
                ))
            }
        }
    }
}

/// Upload-archive body: ACK section + ustar of treeish (default HEAD)
/// multiplexed as sideband-64k band-1.
/// Path-filter (`--prefix=` / includes / `--exclude=`) applied while packing the
/// stream body; B5 core stream parity is evidenced while extended filters stay Partial.
pub(super) fn serve_upload_archive_partial<W: Write>(
    repo: &Repository,
    args: &[String],
    out: &mut W,
    verbose: bool,
) -> Result<(), ServerError> {
    let format = match archive_format_honesty(args) {
        Ok(f) => f,
        Err(e) => return write_err_pkt(out, &e),
    };
    let want_raw = args
        .iter()
        .any(|a| a == "--no-sideband" || a == "--raw-ustar");
    let (treeish, filter) = match parse_archive_path_filter(args) {
        Ok(v) => v,
        Err(e) => return write_err_pkt(out, &e),
    };
    if verbose {
        eprintln!(
            "arks daemon: upload-archive Partial treeish={treeish} format={format} \
             prefix={:?} includes={} excludes={} (ACK+{}; path-filter in stream; \
             B5 core stream parity evidenced; extended filters remain Partial)",
            filter.prefix,
            filter.includes.len(),
            filter.excludes.len(),
            if want_raw {
                "raw ustar"
            } else {
                "sideband-64k band1+band2"
            }
        );
    }
    let meta = match resolve_archive_meta(repo, &treeish) {
        Ok(m) => m,
        Err(e) => return write_err_pkt(out, &e),
    };
    let ack = encode_packet(&Packet::Data(b"ACK\n".to_vec()))
        .map_err(|e| ServerError::Protocol(e.to_string()))?;
    out.write_all(&ack)?;
    // upload-archive protocol: ACK is a response section terminated by flush;
    // archive sideband/raw bytes follow only after this boundary.
    let ack_flush =
        encode_packet(&Packet::Flush).map_err(|e| ServerError::Protocol(e.to_string()))?;
    out.write_all(&ack_flush)?;
    // Force TCP delivery of ACK+flush before archive bytes (stock clients read
    // the flush as its own pkt-line before demuxing sideband).
    out.flush()?;
    if want_raw {
        match write_ustar_archive(repo, &meta, out, &filter) {
            Ok(_n) => {
                out.flush()?;
                Ok(())
            }
            Err(e) => {
                eprintln!("arks daemon: upload-archive Partial ustar err: {e}");
                Err(e)
            }
        }
    } else {
        // Closer to git: band-2 progress (format + filter honesty), then band-1 ustar.
        // Modes follow stock git-archive default tar.umask=002 (0664 files / 0775 dirs+exec).
        write_sideband_progress(
            out,
            &format!(
                "arks Partial upload-archive: packing {treeish} format={format} \
                 (path-filter in stream Partial)\n"
            ),
        )?;
        let mut archive = Vec::new();
        match write_ustar_archive(repo, &meta, &mut archive, &filter) {
            Ok(n) => {
                write_sideband_progress(
                    out,
                    &format!(
                        "arks upload-archive: {n} entries / {} bytes ustar \
                         (B5 core stream parity; extended path-filter Partial)\n",
                        archive.len()
                    ),
                )?;
                write_sideband_stream(out, &archive)
            }
            Err(e) => {
                eprintln!("arks daemon: upload-archive Partial ustar err: {e}");
                Err(e)
            }
        }
    }
}

/// Sideband band-2 progress (Partial; closer to git multiplex before data).
fn write_sideband_progress<W: Write>(out: &mut W, msg: &str) -> Result<(), ServerError> {
    let mut payload = Vec::with_capacity(1 + msg.len());
    payload.push(2); // band 2 = progress
    payload.extend_from_slice(msg.as_bytes());
    let pkt =
        encode_packet(&Packet::Data(payload)).map_err(|e| ServerError::Protocol(e.to_string()))?;
    out.write_all(&pkt)?;
    Ok(())
}

/// Sideband-64k data stream: pkt-lines with leading band byte `1`, then flush.
/// Cite gitprotocol-common / upload-pack sideband shape (archive Partial reuse).
fn write_sideband_stream<W: Write>(out: &mut W, data: &[u8]) -> Result<(), ServerError> {
    // gitprotocol-common max data 65516; one byte is the sideband designator.
    const CHUNK: usize = 65_515;
    for chunk in data.chunks(CHUNK) {
        let mut payload = Vec::with_capacity(1 + chunk.len());
        payload.push(1); // band 1 = data
        payload.extend_from_slice(chunk);
        let pkt = encode_packet(&Packet::Data(payload))
            .map_err(|e| ServerError::Protocol(e.to_string()))?;
        out.write_all(&pkt)?;
    }
    let flush = encode_packet(&Packet::Flush).map_err(|e| ServerError::Protocol(e.to_string()))?;
    out.write_all(&flush)?;
    out.flush()?;
    Ok(())
}

fn tree_for_oid(repo: &Repository, oid: &Oid) -> Result<Oid, String> {
    let obj = repo.get_object(oid).map_err(|e| e.to_string())?;
    match obj.kind {
        ObjectKind::Commit => {
            let c = Commit::decode(&obj.payload).map_err(|e| e.to_string())?;
            Ok(c.tree)
        }
        ObjectKind::Tree => Ok(*oid),
        ObjectKind::Tag => {
            let text = std::str::from_utf8(&obj.payload).map_err(|_| "tag not utf-8")?;
            let mut target = None;
            let mut typ = None;
            for line in text.lines() {
                if line.is_empty() {
                    break;
                }
                if let Some(rest) = line.strip_prefix("object ") {
                    target = Some(Oid::from_hex(rest.trim()).map_err(|e| e.to_string())?);
                } else if let Some(rest) = line.strip_prefix("type ") {
                    typ = Some(rest.trim().to_owned());
                }
            }
            let target = target.ok_or_else(|| "tag missing object".to_string())?;
            if typ.as_deref() == Some("tree") {
                Ok(target)
            } else {
                tree_for_oid(repo, &target)
            }
        }
        ObjectKind::Blob => Err("cannot archive a blob".into()),
    }
}

struct ArchiveMeta {
    tree_oid: Oid,
    /// Commit OID used for stock `pax_global_header` `comment=` (40-hex).
    commit_oid: Oid,
    /// Committer unix time — stock `git archive` mtime for every header.
    mtime: u64,
}

/// Resolve treeish to tree + commit identity for stock-shaped ustar (pax comment/mtime).
fn resolve_archive_meta(repo: &Repository, treeish: &str) -> Result<ArchiveMeta, String> {
    let oid = if treeish == "HEAD" {
        repo.head_oid().map_err(|e| e.to_string())?
    } else if let Ok(o) = Oid::from_hex(treeish) {
        o
    } else {
        repo.refs()
            .resolve(treeish)
            .or_else(|_| repo.refs().resolve(&format!("refs/heads/{treeish}")))
            .or_else(|_| repo.refs().resolve(&format!("refs/tags/{treeish}")))
            .map_err(|e| e.to_string())?
    };
    let obj = repo.get_object(&oid).map_err(|e| e.to_string())?;
    match obj.kind {
        ObjectKind::Commit => {
            let c = Commit::decode(&obj.payload).map_err(|e| e.to_string())?;
            let mtime = if c.committer.timestamp < 0 {
                0
            } else {
                c.committer.timestamp as u64
            };
            Ok(ArchiveMeta {
                tree_oid: c.tree,
                commit_oid: oid,
                mtime,
            })
        }
        ObjectKind::Tag => {
            let tree = tree_for_oid(repo, &oid)?;
            Ok(ArchiveMeta {
                tree_oid: tree,
                commit_oid: oid,
                mtime: 0,
            })
        }
        ObjectKind::Tree => Ok(ArchiveMeta {
            tree_oid: oid,
            commit_oid: oid,
            mtime: 0,
        }),
        ObjectKind::Blob => Err("cannot archive a blob".into()),
    }
}

/// Pack stock-shaped ustar: pax_global_header + dirs + blobs (git archive 2.39.5).
/// Returns count of file (blob) entries included after path-filter.
fn write_ustar_archive<W: Write>(
    repo: &Repository,
    meta: &ArchiveMeta,
    out: &mut W,
    filter: &ArchivePathFilter,
) -> Result<usize, ServerError> {
    let mut archive = Vec::new();
    write_pax_global_header(&mut archive, &meta.commit_oid, meta.mtime)?;
    let included =
        write_ustar_tree_recursive(repo, &meta.tree_oid, "", &mut archive, filter, meta.mtime)?;
    archive.extend_from_slice(&[0u8; 1024]);
    // git archive's tar writer flushes complete 20-record blocks.
    const TAR_BLOCK: usize = 20 * 512;
    let pad = (TAR_BLOCK - (archive.len() % TAR_BLOCK)) % TAR_BLOCK;
    archive.resize(archive.len() + pad, 0);
    out.write_all(&archive)?;
    Ok(included)
}

fn write_pax_global_header<W: Write>(
    out: &mut W,
    commit_oid: &Oid,
    mtime: u64,
) -> Result<(), ServerError> {
    // Stock git archive emits a global pax header: `NN comment=<oid>\n`.
    let oid_hex = commit_oid.to_hex();
    let record_body = format!(" comment={oid_hex}\n");
    // Length field is decimal size of the whole record including the digits.
    let mut len_digits = 1usize;
    loop {
        let total = len_digits + record_body.len();
        let d = total.to_string().len();
        if d == len_digits {
            break;
        }
        len_digits = d;
    }
    let payload = format!("{}{record_body}", len_digits + record_body.len());
    write_ustar_entry(
        out,
        "pax_global_header",
        payload.as_bytes(),
        0o666,
        b'g',
        mtime,
    )
}

fn write_ustar_tree_recursive<W: Write>(
    repo: &Repository,
    tree_oid: &Oid,
    parent: &str,
    out: &mut W,
    filter: &ArchivePathFilter,
    mtime: u64,
) -> Result<usize, ServerError> {
    let obj = repo
        .get_object(tree_oid)
        .map_err(|e| ServerError::Msg(e.to_string()))?;
    if obj.kind != ObjectKind::Tree {
        return Err(ServerError::Msg("archive tree object expected".into()));
    }
    let tree =
        Tree::decode(&obj.payload, tree_oid.kind()).map_err(|e| ServerError::Msg(e.to_string()))?;
    let mut included = 0usize;
    for entry in tree.entries {
        let path = if parent.is_empty() {
            entry.name.clone()
        } else {
            format!("{parent}/{}", entry.name)
        };
        match entry.mode {
            TreeMode::Tree => {
                // Recurse even when the directory itself is not an include match:
                // a pathspec may name one of its children.
                let mut nested = Vec::new();
                let child_count = write_ustar_tree_recursive(
                    repo,
                    &entry.oid,
                    &path,
                    &mut nested,
                    filter,
                    mtime,
                )?;
                if child_count > 0 || path_allowed(&path, filter) {
                    let archived = format!("{}{path}/", filter.prefix);
                    if archived.len() <= 100 {
                        // Stock git archive: directory before its children.
                        write_ustar_entry(out, &archived, &[], 0o775, b'5', mtime)?;
                        out.write_all(&nested)?;
                        included += child_count;
                    }
                }
            }
            TreeMode::File | TreeMode::Executable => {
                if !path_allowed(&path, filter) {
                    continue;
                }
                let archived = format!("{}{path}", filter.prefix);
                if archived.len() > 100 {
                    continue;
                }
                let blob = repo
                    .get_object(&entry.oid)
                    .map_err(|e| ServerError::Msg(e.to_string()))?;
                if blob.kind != ObjectKind::Blob {
                    continue;
                }
                // Match bookworm git 2.39.5 archive mode bits observed in clean-room.
                let mode = if entry.mode == TreeMode::Executable {
                    0o775
                } else {
                    0o664
                };
                write_ustar_entry(out, &archived, &blob.payload, mode, b'0', mtime)?;
                included += 1;
            }
            TreeMode::Symlink | TreeMode::Gitlink => {}
        }
    }
    Ok(included)
}

fn write_ustar_entry<W: Write>(
    out: &mut W,
    path: &str,
    data: &[u8],
    mode: u32,
    entry_type: u8,
    mtime: u64,
) -> Result<(), ServerError> {
    let mut header = [0u8; 512];
    header[..path.len()].copy_from_slice(path.as_bytes());
    write_octal(&mut header[100..108], u64::from(mode));
    write_octal(&mut header[108..116], 0); // uid
    write_octal(&mut header[116..124], 0); // gid
    write_octal(&mut header[124..136], data.len() as u64);
    write_octal(&mut header[136..148], mtime);
    header[148..156].fill(b' ');
    header[156] = entry_type;
    header[257..262].copy_from_slice(b"ustar");
    header[262] = 0;
    header[263..265].copy_from_slice(b"00");
    // Stock git archive fills uname/gname with "root".
    header[265..269].copy_from_slice(b"root");
    header[297..301].copy_from_slice(b"root");
    write_octal(&mut header[329..337], 0); // devmajor
    write_octal(&mut header[337..345], 0); // devminor
    let sum: u32 = header.iter().map(|&b| u32::from(b)).sum();
    write_octal(&mut header[148..156], u64::from(sum));
    out.write_all(&header)?;
    out.write_all(data)?;
    let pad = (512 - (data.len() % 512)) % 512;
    if pad > 0 {
        out.write_all(&vec![0u8; pad])?;
    }
    Ok(())
}

fn write_octal(dst: &mut [u8], value: u64) {
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

pub(super) fn drain_upload_archive_args<R: Read>(
    input: &mut R,
) -> Result<Vec<String>, ServerError> {
    let mut args = Vec::new();
    // Cap reads so a stuck peer cannot hang forever (Partial).
    for _ in 0..64 {
        let mut lenbuf = [0u8; 4];
        match input.read_exact(&mut lenbuf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
        let pkt_len = parse_pkt_len(&lenbuf)?;
        // 0000 flush / 0001 delim / 0002 response-end — end of argument list.
        if pkt_len < 4 {
            break;
        }
        let mut payload = vec![0u8; pkt_len - 4];
        if !payload.is_empty() {
            input.read_exact(&mut payload)?;
        }
        let line = String::from_utf8_lossy(&payload);
        let line = line.trim_end_matches(['\n', '\r']);
        if let Some(arg) = line.strip_prefix("argument ") {
            args.push(arg.to_string());
        } else if !line.is_empty() {
            args.push(line.to_string());
        }
    }
    Ok(args)
}

#[cfg(test)]
mod archive_path_filter_tests {
    use super::*;

    #[test]
    fn parse_prefix_includes_excludes() {
        let args = vec![
            "HEAD".into(),
            "--prefix=pkg/".into(),
            "src".into(),
            "--exclude=src/secret".into(),
        ];
        let (treeish, filter) = parse_archive_path_filter(&args).unwrap();
        assert_eq!(treeish, "HEAD");
        assert_eq!(filter.prefix, "pkg/");
        assert_eq!(filter.includes, vec!["src".to_string()]);
        assert_eq!(filter.excludes, vec!["src/secret".to_string()]);
    }

    #[test]
    fn path_filter_include_exclude_parity_partial() {
        let filter = ArchivePathFilter {
            prefix: String::new(),
            includes: vec!["src".into()],
            excludes: vec!["src/secret".into()],
        };
        assert!(path_allowed("src/lib.rs", &filter));
        assert!(!path_allowed("src/secret/key", &filter));
        assert!(!path_allowed("docs/readme", &filter));
        let empty = ArchivePathFilter::default();
        assert!(path_allowed("anything", &empty));
    }

    #[test]
    fn path_filter_exclude_segment_name() {
        let filter = ArchivePathFilter {
            prefix: "out/".into(),
            includes: vec![],
            excludes: vec![".git".into()],
        };
        assert!(!path_allowed("foo/.git/config", &filter));
        assert!(path_allowed("foo/bar", &filter));
    }

    #[test]
    fn archive_format_honesty_tar_only_partial() {
        assert_eq!(archive_format_honesty(&[]).unwrap(), "tar");
        assert_eq!(
            archive_format_honesty(&["--format=ustar".into()]).unwrap(),
            "ustar"
        );
        assert!(archive_format_honesty(&["--format=zip".into()]).is_err());
        assert!(archive_format_honesty(&["--format=".into()]).is_err());
    }

    #[test]
    fn upload_archive_protocol_body_applies_path_filter() {
        use arks_object::{Commit, ObjectKind, RawObject, Signature, Tree, TreeEntry, TreeMode};
        use arks_store::{ObjectFormat, ObjectStore};
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let git = dir.path().to_path_buf();
        std::fs::create_dir_all(git.join("objects")).unwrap();
        std::fs::create_dir_all(git.join("refs/heads")).unwrap();
        std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        let mut repo = Repository::open(&git).unwrap();
        let keep = RawObject::new(ObjectKind::Blob, b"keep\n".to_vec());
        let keep_oid = repo.store_mut().put(ObjectFormat::Sha1, &keep).unwrap();
        let secret = RawObject::new(ObjectKind::Blob, b"secret\n".to_vec());
        let secret_oid = repo.store_mut().put(ObjectFormat::Sha1, &secret).unwrap();
        let tree = Tree::new(vec![
            TreeEntry {
                mode: TreeMode::File,
                name: "keep.txt".into(),
                oid: keep_oid,
            },
            TreeEntry {
                mode: TreeMode::File,
                name: "secret.txt".into(),
                oid: secret_oid,
            },
        ])
        .unwrap();
        let tree_obj = RawObject::new(ObjectKind::Tree, tree.encode().unwrap());
        let tree_oid = repo.store_mut().put(ObjectFormat::Sha1, &tree_obj).unwrap();
        let sig = Signature {
            name: "arks".into(),
            email: "arks@example.com".into(),
            timestamp: 1_700_000_000,
            timezone: "+0000".into(),
        };
        let commit = Commit {
            tree: tree_oid,
            parents: Vec::new(),
            author: sig.clone(),
            committer: sig,
            gpgsig: None,
            message: "archive filter\n".into(),
        };
        let commit_obj = RawObject::new(ObjectKind::Commit, commit.encode().unwrap());
        let commit_oid = repo
            .store_mut()
            .put(ObjectFormat::Sha1, &commit_obj)
            .unwrap();
        repo.refs_mut()
            .update("refs/heads/main", &commit_oid, None)
            .unwrap();

        let args = vec![
            "HEAD".into(),
            "--prefix=pkg/".into(),
            "--exclude=secret.txt".into(),
            "--format=tar".into(),
        ];
        let mut body = Vec::new();
        serve_upload_archive_partial(&repo, &args, &mut body, false).unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("ACK"), "protocol body needs ACK");
        assert!(
            body.windows(b"pkg/keep.txt".len())
                .any(|w| w == b"pkg/keep.txt"),
            "filtered stream must include prefixed keep.txt"
        );
        assert!(
            !body
                .windows(b"secret.txt".len())
                .any(|w| w == b"secret.txt"),
            "path-filter must drop secret.txt from stream body"
        );
        assert!(
            text.contains("path-filter"),
            "format/filter honesty in sideband progress Partial"
        );
    }

    #[test]
    fn path_filter_nested_include_exclude_partial() {
        let filter = ArchivePathFilter {
            prefix: "rel/".into(),
            includes: vec!["src".into(), "docs/readme".into()],
            excludes: vec!["src/vendor".into(), ".git".into()],
        };
        assert!(path_allowed("src/lib.rs", &filter));
        assert!(path_allowed("docs/readme", &filter));
        assert!(!path_allowed("src/vendor/dep.rs", &filter));
        assert!(!path_allowed("other/file", &filter));
        assert!(!path_allowed("docs/.git/config", &filter));
    }

    #[test]
    fn parse_archive_unknown_option_refuse_partial() {
        let err = parse_archive_path_filter(&["HEAD".into(), "--remote=origin".into()])
            .expect_err("unknown option must refuse");
        assert!(
            err.contains("unknown option") || err.contains("not Claimed"),
            "honest refuse: {err}"
        );
    }

    #[test]
    fn parse_archive_exclude_only_keeps_default_treeish() {
        let (treeish, filter) =
            parse_archive_path_filter(&["--exclude=tmp".into(), "--prefix=out/".into()]).unwrap();
        assert_eq!(treeish, "HEAD");
        assert_eq!(filter.prefix, "out/");
        assert!(filter.includes.is_empty());
        assert_eq!(filter.excludes, vec!["tmp".to_string()]);
    }

    #[test]
    fn upload_archive_zip_format_refuse_partial() {
        use arks_object::{Commit, ObjectKind, RawObject, Signature, Tree};
        use arks_store::{ObjectFormat, ObjectStore};
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let git = dir.path().to_path_buf();
        std::fs::create_dir_all(git.join("objects")).unwrap();
        std::fs::create_dir_all(git.join("refs/heads")).unwrap();
        std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        let mut repo = Repository::open(&git).unwrap();
        let tree = Tree::new(vec![]).unwrap();
        let tree_obj = RawObject::new(ObjectKind::Tree, tree.encode().unwrap());
        let tree_oid = repo.store_mut().put(ObjectFormat::Sha1, &tree_obj).unwrap();
        let sig = Signature {
            name: "arks".into(),
            email: "arks@example.com".into(),
            timestamp: 1_700_000_000,
            timezone: "+0000".into(),
        };
        let commit = Commit {
            tree: tree_oid,
            parents: Vec::new(),
            author: sig.clone(),
            committer: sig,
            gpgsig: None,
            message: "empty tree\n".into(),
        };
        let commit_obj = RawObject::new(ObjectKind::Commit, commit.encode().unwrap());
        let commit_oid = repo
            .store_mut()
            .put(ObjectFormat::Sha1, &commit_obj)
            .unwrap();
        repo.refs_mut()
            .update("refs/heads/main", &commit_oid, None)
            .unwrap();

        let mut body = Vec::new();
        serve_upload_archive_partial(
            &repo,
            &["HEAD".into(), "--format=zip".into()],
            &mut body,
            false,
        )
        .unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("ERR")
                && (text.contains("zip") || text.contains("Missing") || text.contains("not wired")),
            "zip must refuse via ERR pkt-line Partial: {text}"
        );
        assert!(
            !text.contains("ACK"),
            "zip refuse must not ACK a Claimed archive body"
        );
    }

    #[test]
    fn upload_archive_empty_tree_ustar_partial() {
        use arks_object::{Commit, ObjectKind, RawObject, Signature, Tree};
        use arks_store::{ObjectFormat, ObjectStore};
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let git = dir.path().to_path_buf();
        std::fs::create_dir_all(git.join("objects")).unwrap();
        std::fs::create_dir_all(git.join("refs/heads")).unwrap();
        std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        let mut repo = Repository::open(&git).unwrap();
        let tree = Tree::new(vec![]).unwrap();
        let tree_obj = RawObject::new(ObjectKind::Tree, tree.encode().unwrap());
        let tree_oid = repo.store_mut().put(ObjectFormat::Sha1, &tree_obj).unwrap();
        let sig = Signature {
            name: "arks".into(),
            email: "arks@example.com".into(),
            timestamp: 1_700_000_000,
            timezone: "+0000".into(),
        };
        let commit = Commit {
            tree: tree_oid,
            parents: Vec::new(),
            author: sig.clone(),
            committer: sig,
            gpgsig: None,
            message: "empty\n".into(),
        };
        let commit_obj = RawObject::new(ObjectKind::Commit, commit.encode().unwrap());
        let commit_oid = repo
            .store_mut()
            .put(ObjectFormat::Sha1, &commit_obj)
            .unwrap();
        repo.refs_mut()
            .update("refs/heads/main", &commit_oid, None)
            .unwrap();

        let mut body = Vec::new();
        serve_upload_archive_partial(
            &repo,
            &["HEAD".into(), "--format=ustar".into()],
            &mut body,
            false,
        )
        .unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("ACK"),
            "empty-tree archive still ACKs Partial"
        );
        // ustar ends with two 512-byte zero blocks; Claimed stream parity is Docker cmp -s.
        assert!(
            body.len() >= 512,
            "ustar body should carry trailer/padding bytes"
        );
    }

    /// Git v2.39.5 `prepare_header` numeric field shapes (`"%07o"` / `"%011o"`).
    /// Cite: research:root/20261111084400, research:root/20261111090827.
    #[test]
    fn prepare_header_octal_widths_match_git_v2_39_5() {
        let mut mode = [0u8; 8];
        write_octal(&mut mode, 0o664);
        assert_eq!(&mode, b"0000664\0");

        let mut size = [0u8; 12];
        write_octal(&mut size, 15);
        assert_eq!(&size, b"00000000017\0");

        let mut zero8 = [0u8; 8];
        write_octal(&mut zero8, 0);
        assert_eq!(&zero8, b"0000000\0");

        let mut zero12 = [0u8; 12];
        write_octal(&mut zero12, 0);
        assert_eq!(&zero12, b"00000000000\0");
    }

    #[test]
    fn prepare_header_writes_device_fields_and_space_chksum() {
        let mut out = Vec::new();
        write_ustar_entry(
            &mut out,
            "pax_global_header",
            b"52 comment=deadbeef\n",
            0o666,
            b'g',
            1_577_934_245,
        )
        .unwrap();
        assert!(out.len() >= 512);
        let header = &out[..512];
        // Device major/minor always written as "%07o" of 0 (never left all-NUL).
        assert_eq!(&header[329..337], b"0000000\0");
        assert_eq!(&header[337..345], b"0000000\0");
        assert_eq!(&header[257..263], b"ustar\0");
        assert_eq!(&header[263..265], b"00");
        assert_eq!(&header[265..269], b"root");
        assert_eq!(&header[297..301], b"root");
        // Chksum is "%07o" of sum-with-spaces (7 digits + NUL), not GNU 6+NUL+SP.
        assert_eq!(header[155], 0);
        assert!(header[148..155].iter().all(|&b| (b'0'..=b'7').contains(&b)));
        let mut spaces = header.to_vec();
        spaces[148..156].fill(b' ');
        let sum: u32 = spaces.iter().map(|&b| u32::from(b)).sum();
        let mut expect = [0u8; 8];
        write_octal(&mut expect, u64::from(sum));
        assert_eq!(&header[148..156], &expect);
    }
}
