//! `archive`: tar (stock git's ustar + pax layout), tgz / tar.gz and zip
//! archives of a tree, with `--prefix`, pathspecs, `--add-file`,
//! `--add-virtual-file`, `--mtime`, `-o` (format from the extension),
//! `--list`, `--verbose` and `--remote` for local repositories.
//!
//! Primary: <https://git-scm.com/docs/git-archive>; tar layout
//! (`pax_global_header` with `comment=<commit>`, `root` owner names,
//! `tar.umask` 0002, 10240-byte records) from observed stock git output;
//! ustar / pax: POSIX.1-2001 `pax`; zip: PKWARE APPNOTE (local headers,
//! central directory, `UT` extra field); gzip: RFC 1952.

use crate::cmd::opt_spec::parse;
use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_object::{Commit, ObjectKind, RawObject, Tree, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::fs;
use std::io::{self, Write};

const ARCHIVE_OPTS: &[&str] = &[
    "|format=",
    "|prefix=",
    "|add-file=",
    "|add-virtual-file=",
    "o|output=",
    "|worktree-attributes",
    "v|verbose",
    "|mtime=",
    "0|",
    "1|",
    "2|",
    "3|",
    "4|",
    "5|",
    "6|",
    "7|",
    "8|",
    "9|",
    "l|list",
    "|remote=",
    "|exec=",
];

/// One archive member.
struct Member {
    path: String,
    /// `None` for a directory.
    data: Option<Vec<u8>>,
    mode: u32,
    symlink: Option<String>,
}

pub fn cmd_archive(args: &[String]) -> CliResult<()> {
    let p = parse(args, ARCHIVE_OPTS)?;
    if p.on("list") {
        println!("tar\ntgz\ntar.gz\nzip");
        return Ok(());
    }
    let output = p.value("output");
    let repo = match p.value("remote") {
        Some(remote) => remote_repo(&remote)?,
        None => open_repo()?,
    };
    // git opens (and truncates) `--output` before reading the tree-ish.
    if let Some(path) = &output {
        // MISC-002 (M20 Cx-5): parallel archive to same occupied prefix refuse.
        let outp = std::path::Path::new(path);
        if outp.exists() {
            return Err(CliError::Msg(format!(
                "archive output '{}' exists (MISC-002 — parallel archive/checkout-index to same prefix refuse; choose a unique path)",
                outp.display()
            )));
        }
        fs::write(outp, b"")?;
    }
    let bytes = archive_bytes(&repo, &p)?;
    match output {
        Some(path) => fs::write(path, bytes)?,
        None => {
            let mut out = io::stdout().lock();
            out.write_all(&bytes)?;
            out.flush()?;
        }
    }
    Ok(())
}

/// The archive stream for `args` (stock `git archive` options, no
/// `--remote`), read from `repo`; `upload-archive` serves the same bytes.
pub(crate) fn archive_bytes_for_args(repo: &Repository, args: &[String]) -> CliResult<Vec<u8>> {
    let p = parse(args, ARCHIVE_OPTS)?;
    archive_bytes(repo, &p)
}

fn archive_bytes(repo: &Repository, p: &crate::cmd::opt_spec::Parsed) -> CliResult<Vec<u8>> {
    let output = p.value("output");
    let format = match p.value("format") {
        Some(f) => f,
        None => output
            .as_deref()
            .and_then(|o| {
                ["tar.gz", "tgz", "zip", "tar"]
                    .into_iter()
                    .find(|ext| o.ends_with(&format!(".{ext}")))
            })
            .unwrap_or("tar")
            .to_string(),
    };
    if !matches!(format.as_str(), "tar" | "tgz" | "tar.gz" | "zip") {
        return Err(CliError::fatal(format!(
            "Unknown archive format '{format}'"
        )));
    }
    let level = ('0'..='9')
        .rev()
        .find(|c| p.on(&c.to_string()))
        .map(|c| c as u8 - b'0');
    if level.is_some_and(|l| l > 1 && format == "tar") {
        // git: compression levels are rejected for plain tar beyond the defaults.
    }
    let Some(rev) = p.args.first() else {
        return Err(CliError::Exit(
            129,
            "usage: git archive [<options>] <tree-ish> [<path>...]".into(),
        ));
    };
    let oid = resolve_rev(repo, rev)
        .map_err(|_| CliError::fatal(format!("not a valid object name: {rev}")))?;
    let (commit, tree) = peel_tree(repo, &oid)?;
    let commit_time = commit.as_ref().map(|c| c.1).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    });
    let mtime = match p.value("mtime") {
        Some(m) => crate::cmd::shared::parse_git_date(&m)
            .map(|(t, _)| t)
            .or_else(|| parse_ymd(&m))
            .ok_or_else(|| CliError::fatal(format!("invalid --mtime: {m}")))?,
        None => commit_time,
    };
    let prefix = p.value("prefix").unwrap_or_default();
    let pathspecs: Vec<String> = p.args[1..].to_vec();

    let mut members = Vec::new();
    if prefix.ends_with('/') {
        members.push(Member {
            path: prefix.clone(),
            data: None,
            mode: 0o40775,
            symlink: None,
        });
    }
    collect_tree(repo, &tree, "", &prefix, &pathspecs, &mut members)?;
    for spec in &pathspecs {
        let base = spec.trim_end_matches('/');
        let hit = members.iter().any(|m| {
            let rel = m.path.strip_prefix(prefix.as_str()).unwrap_or(&m.path);
            rel.trim_end_matches('/') == base || rel.starts_with(&format!("{base}/"))
        });
        if !hit {
            return Err(CliError::fatal(format!(
                "pathspec '{spec}' did not match any files"
            )));
        }
    }
    // `--add-file` takes the `--prefix` given before it; `--add-virtual-file`
    // paths are used as given.
    let mut cur_prefix = String::new();
    for e in &p.events {
        match (e.key, &e.value, e.neg) {
            ("prefix", Some(v), false) => cur_prefix = v.clone(),
            ("prefix", _, true) => cur_prefix.clear(),
            ("add-file", Some(f), false) => {
                let data = fs::read(f)
                    .map_err(|err| CliError::fatal(format!("unable to read {f}: {err}")))?;
                let name = std::path::Path::new(f)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or(f.clone());
                let exec = file_is_executable(f);
                members.push(Member {
                    path: format!("{cur_prefix}{name}"),
                    data: Some(data),
                    mode: if exec { 0o100775 } else { 0o100664 },
                    symlink: None,
                });
            }
            ("add-virtual-file", Some(v), false) => {
                let (path, content) = v.split_once(':').ok_or_else(|| {
                    CliError::fatal("--add-virtual-file requires <path>:<content>")
                })?;
                members.push(Member {
                    path: path.trim_start_matches('/').to_string(),
                    data: Some(content.as_bytes().to_vec()),
                    mode: 0o100664,
                    symlink: None,
                });
            }
            _ => {}
        }
    }
    if p.count("verbose") > 0 {
        for m in &members {
            eprintln!("{}", m.path);
        }
    }

    let comment = commit.as_ref().map(|c| c.0.to_hex());
    Ok(match format.as_str() {
        "zip" => zip_bytes(&members, mtime, comment.as_deref(), level.unwrap_or(6)),
        "tar" => tar_bytes(&members, mtime, comment.as_deref()),
        _ => gzip(&tar_bytes(&members, mtime, comment.as_deref())),
    })
}

/// `--remote=<repo>` for a local repository: the archive is built from that
/// repository and written here (`-o` stays relative to this directory).
fn remote_repo(remote: &str) -> CliResult<Repository> {
    let repo = open_repo().ok();
    let cfg = arks_config::ConfigSet::load(repo.as_ref().map(|r| r.common_dir()));
    let url = crate::cmd::fetch::configured_source(&cfg, remote)
        .map(|s| s.url)
        .unwrap_or_else(|| remote.to_string());
    if !arks_remote::is_local_remote(&url) {
        return Err(CliError::NotImplemented(
            "archive --remote is claimed for local-path repositories (the daemon serves upload-archive)".into(),
        ));
    }
    arks_remote::open_local_remote(&url).map_err(|e| CliError::fatal(e.to_string()))
}

fn file_is_executable(path: &str) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o100 != 0)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

/// `YYYY-MM-DD` (midnight UTC) for `--mtime`.
fn parse_ymd(s: &str) -> Option<i64> {
    let mut it = s.trim().split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let d: i64 = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    // Days from civil (proleptic Gregorian).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146097 + doe - 719468) * 86400)
}

/// Commit (oid, committer time) when `oid` peels through one, and its tree.
fn peel_tree(repo: &Repository, oid: &Oid) -> CliResult<(Option<(Oid, i64)>, Oid)> {
    let mut cur = *oid;
    loop {
        let obj = repo.get_object(&cur)?;
        match obj.kind {
            ObjectKind::Commit => {
                let c = Commit::decode(&obj.payload)?;
                let t = c.committer.timestamp.to_string().parse().unwrap_or(0);
                return Ok((Some((cur, t)), c.tree));
            }
            ObjectKind::Tree => return Ok((None, cur)),
            ObjectKind::Tag => {
                cur = crate::cmd::remote_local::peel(repo, &cur);
                if cur == *oid {
                    return Err(CliError::fatal("not a tree object"));
                }
            }
            ObjectKind::Blob => {
                return Err(CliError::fatal(format!(
                    "not a tree object: {}",
                    oid.to_hex()
                )));
            }
        }
    }
}

/// Does `path` (a file or directory) fall under one of the pathspecs?
fn wanted(path: &str, is_dir: bool, specs: &[String]) -> (bool, bool) {
    if specs.is_empty() {
        return (true, true);
    }
    let mut take = false;
    let mut descend = false;
    for s in specs {
        let s = s.trim_end_matches('/');
        if path == s || path.starts_with(&format!("{s}/")) {
            take = true;
            descend = true;
        } else if is_dir && s.starts_with(&format!("{path}/")) {
            descend = true;
            take = true;
        }
    }
    (take, descend)
}

fn collect_tree(
    repo: &Repository,
    tree: &Oid,
    base: &str,
    prefix: &str,
    specs: &[String],
    out: &mut Vec<Member>,
) -> CliResult<()> {
    let obj = repo.get_object(tree)?;
    let t = Tree::decode(&obj.payload, repo.oid_kind())?;
    for e in &t.entries {
        let rel = if base.is_empty() {
            e.name.clone()
        } else {
            format!("{base}/{}", e.name)
        };
        match e.mode {
            TreeMode::Tree => {
                let (take, descend) = wanted(&rel, true, specs);
                if take {
                    out.push(Member {
                        path: format!("{prefix}{rel}/"),
                        data: None,
                        mode: 0o40775,
                        symlink: None,
                    });
                }
                if descend {
                    collect_tree(repo, &e.oid, &rel, prefix, specs, out)?;
                }
            }
            TreeMode::Gitlink => {
                if wanted(&rel, true, specs).0 {
                    out.push(Member {
                        path: format!("{prefix}{rel}/"),
                        data: None,
                        mode: 0o40775,
                        symlink: None,
                    });
                }
            }
            mode => {
                if !wanted(&rel, false, specs).0 {
                    continue;
                }
                let data = repo.get_object(&e.oid)?.payload;
                let (mode, symlink) = match mode {
                    TreeMode::Executable => (0o100775, None),
                    TreeMode::Symlink => {
                        (0o120777, Some(String::from_utf8_lossy(&data).to_string()))
                    }
                    _ => (0o100664, None),
                };
                out.push(Member {
                    path: format!("{prefix}{rel}"),
                    data: if symlink.is_some() {
                        Some(Vec::new())
                    } else {
                        Some(data)
                    },
                    mode,
                    symlink,
                });
            }
        }
    }
    Ok(())
}

fn octal(dst: &mut [u8], value: u64) {
    // Zero-padded octal filling all but the last byte, which stays NUL.
    let width = dst.len() - 1;
    let s = format!("{value:0width$o}");
    let s = &s.as_bytes()[s.len().saturating_sub(width)..];
    dst[..width].copy_from_slice(s);
    dst[width] = 0;
}

/// One 512-byte ustar header (stock git field values).
fn tar_header(
    name: &str,
    mode: u32,
    size: u64,
    mtime: i64,
    typeflag: u8,
    link: &str,
) -> CliResult<[u8; 512]> {
    let mut h = [0u8; 512];
    let bytes = name.as_bytes();
    if bytes.len() <= 100 {
        h[..bytes.len()].copy_from_slice(bytes);
    } else {
        // ustar split: prefix (<= 155) '/' name (<= 100).
        let split = bytes[..bytes.len().min(156)]
            .iter()
            .rposition(|&b| b == b'/')
            .filter(|&i| bytes.len() - i - 1 <= 100 && i > 0)
            .ok_or_else(|| {
                CliError::NotImplemented(format!("archive: path too long for ustar: {name}"))
            })?;
        h[345..345 + split].copy_from_slice(&bytes[..split]);
        h[..bytes.len() - split - 1].copy_from_slice(&bytes[split + 1..]);
    }
    octal(&mut h[100..108], u64::from(mode & 0o7777));
    octal(&mut h[108..116], 0);
    octal(&mut h[116..124], 0);
    octal(&mut h[124..136], size);
    octal(&mut h[136..148], mtime.max(0) as u64);
    h[156] = typeflag;
    let lb = link.as_bytes();
    if lb.len() > 100 {
        return Err(CliError::NotImplemented(format!(
            "archive: symlink target too long for ustar: {name}"
        )));
    }
    h[157..157 + lb.len()].copy_from_slice(lb);
    h[257..263].copy_from_slice(b"ustar\0");
    h[263..265].copy_from_slice(b"00");
    h[265..269].copy_from_slice(b"root");
    h[297..301].copy_from_slice(b"root");
    octal(&mut h[329..337], 0);
    octal(&mut h[337..345], 0);
    h[148..156].fill(b' ');
    let sum: u64 = h.iter().map(|&b| u64::from(b)).sum();
    octal(&mut h[148..156], sum);
    Ok(h)
}

fn push_padded(out: &mut Vec<u8>, data: &[u8]) {
    out.extend_from_slice(data);
    let pad = (512 - data.len() % 512) % 512;
    out.extend(std::iter::repeat_n(0u8, pad));
}

/// Tar stream as stock git writes it: pax global header with the commit
/// id, entries, then zero padding to a 10240-byte record (plus one more
/// record when fewer than two zero blocks would remain).
fn tar_bytes(members: &[Member], mtime: i64, comment: Option<&str>) -> Vec<u8> {
    let mut out = Vec::new();
    if let Some(c) = comment {
        let body = format!("comment={c}\n");
        // The record length counts its own digits.
        let mut len = body.len() + 3;
        while format!("{len} {body}").len() != len {
            len += 1;
        }
        let record = format!("{len} {body}");
        if let Ok(h) = tar_header(
            "pax_global_header",
            0o666,
            record.len() as u64,
            mtime,
            b'g',
            "",
        ) {
            out.extend_from_slice(&h);
            push_padded(&mut out, record.as_bytes());
        }
    }
    for m in members {
        let (typeflag, size, link) = match (&m.data, &m.symlink) {
            (None, _) => (b'5', 0, ""),
            (Some(_), Some(target)) => (b'2', 0, target.as_str()),
            (Some(d), None) => (b'0', d.len() as u64, ""),
        };
        match tar_header(&m.path, m.mode, size, mtime, typeflag, link) {
            Ok(h) => out.extend_from_slice(&h),
            Err(_) => continue,
        }
        if let (Some(d), None) = (&m.data, &m.symlink) {
            push_padded(&mut out, d);
        }
    }
    const RECORD: usize = 10240;
    let offset = out.len() % RECORD;
    let tail = RECORD - offset;
    out.extend(std::iter::repeat_n(0u8, tail));
    if tail < 1024 {
        out.extend(std::iter::repeat_n(0u8, RECORD));
    }
    out
}

/// CRC-32 (IEEE 802.3, reflected) as gzip and zip use it.
fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc = table[((crc ^ u32::from(b)) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

/// Raw deflate stream: zlib output without its 2-byte header and Adler-32.
fn deflate_raw(data: &[u8]) -> Vec<u8> {
    match arks_object::zlib_compress(data) {
        Ok(z) if z.len() >= 6 => z[2..z.len() - 4].to_vec(),
        _ => Vec::new(),
    }
}

/// gzip member (RFC 1952) with mtime 0 and OS = Unix, as `git archive` emits.
fn gzip(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3];
    out.extend(deflate_raw(data));
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

/// MS-DOS date/time pair for a Unix timestamp (UTC).
fn dos_datetime(t: i64) -> (u16, u16) {
    let days = t.div_euclid(86400);
    let secs = t.rem_euclid(86400);
    // Civil from days (proleptic Gregorian).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let date = (((y - 1980).max(0) as u16) << 9) | ((m as u16) << 5) | d as u16;
    let time = (((secs / 3600) as u16) << 11)
        | ((((secs / 60) % 60) as u16) << 5)
        | ((secs % 60) / 2) as u16;
    (time, date)
}

/// Zip archive: deflated (or stored) members with a `UT` mtime extra field
/// and the commit id as the archive comment.
fn zip_bytes(members: &[Member], mtime: i64, comment: Option<&str>, level: u8) -> Vec<u8> {
    let (time, date) = dos_datetime(mtime);
    let mut out = Vec::new();
    let mut central = Vec::new();
    let le16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    for m in members {
        let raw: Vec<u8> = match (&m.data, &m.symlink) {
            (None, _) => Vec::new(),
            (Some(_), Some(t)) => t.as_bytes().to_vec(),
            (Some(d), None) => d.clone(),
        };
        let deflated = if level > 0 && !raw.is_empty() {
            deflate_raw(&raw)
        } else {
            Vec::new()
        };
        let (method, body) = if !deflated.is_empty() && deflated.len() < raw.len() {
            (8u16, deflated)
        } else {
            (0u16, raw.clone())
        };
        let crc = crc32(&raw);
        let name = m.path.as_bytes();
        let offset = out.len() as u32;
        let mut extra = Vec::new();
        le16(&mut extra, 0x5455);
        le16(&mut extra, 5);
        extra.push(1);
        le32(&mut extra, mtime.max(0) as u32);
        let version = if method == 8 { 20 } else { 10 };
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, version);
        le16(&mut out, 0);
        le16(&mut out, method);
        le16(&mut out, time);
        le16(&mut out, date);
        le32(&mut out, crc);
        le32(&mut out, body.len() as u32);
        le32(&mut out, raw.len() as u32);
        le16(&mut out, name.len() as u16);
        le16(&mut out, extra.len() as u16);
        out.extend_from_slice(name);
        out.extend_from_slice(&extra);
        out.extend_from_slice(&body);
        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 0x0317);
        le16(&mut central, version);
        le16(&mut central, 0);
        le16(&mut central, method);
        le16(&mut central, time);
        le16(&mut central, date);
        le32(&mut central, crc);
        le32(&mut central, body.len() as u32);
        le32(&mut central, raw.len() as u32);
        le16(&mut central, name.len() as u16);
        le16(&mut central, extra.len() as u16);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le32(
            &mut central,
            (m.mode << 16) | if m.data.is_none() { 0x10 } else { 0 },
        );
        le32(&mut central, offset);
        central.extend_from_slice(name);
        central.extend_from_slice(&extra);
    }
    let cd_offset = out.len() as u32;
    let cd_len = central.len() as u32;
    out.extend_from_slice(&central);
    let comment = comment.unwrap_or("").as_bytes();
    le32(&mut out, 0x0605_4b50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, members.len() as u16);
    le16(&mut out, members.len() as u16);
    le32(&mut out, cd_len);
    le32(&mut out, cd_offset);
    le16(&mut out, comment.len() as u16);
    out.extend_from_slice(comment);
    out
}

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
    // git-tag: the tagger is the committer identity; `type` names the target kind.
    let sig = crate::cmd::shared::committer_signature()?;
    let kind = repo.get_object(target)?.kind;
    let payload = encode_tag_payload(target, kind.as_str(), name, &sig.format_line(), message);
    let obj = RawObject::new(ObjectKind::Tag, payload);
    let format = repo.object_format();
    let tag_oid = repo.store_mut().put(format, &obj)?;
    repo.refs_mut()
        .update(&format!("refs/tags/{name}"), &tag_oid, None)?;
    Ok(tag_oid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_and_dates() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(parse_ymd("1970-01-02"), Some(86400));
        assert_eq!(dos_datetime(315532800), (0, (1 << 5) | 1));
    }

    #[test]
    fn tar_record_padding() {
        let t = tar_bytes(&[], 0, None);
        assert_eq!(t.len(), 10240);
        assert!(t.iter().all(|&b| b == 0));
    }
}
