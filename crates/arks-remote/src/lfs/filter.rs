//! In-process Git LFS `clean`, `smudge`, and long-running `filter-process` (pkt-line v2).
//!
//! Primary references:
//! - Git `Documentation/gitattributes.txt` (§Long Running Filter Process)
//! - <https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md>
//!
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.7 (E6).

use crate::lfs::LfsError;
use crate::lfs::pointer::{LfsPointer, MAX_POINTER_BYTES};
use crate::lfs::store::LfsStore;
use std::io::{Cursor, Read, Write};

const MAX_PKT_DATA: usize = 65516;

/// `git-lfs clean`: store content into `store` and write a canonical LFS v1 pointer.
/// If input is already a valid LFS v1 pointer, passes it through unchanged (idempotent).
pub fn clean<R: Read, W: Write>(
    store: &LfsStore,
    mut reader: R,
    mut writer: W,
) -> Result<LfsPointer, LfsError> {
    // Peek up to MAX_POINTER_BYTES + 1 to check if input is already a canonical pointer.
    let mut head = Vec::with_capacity(MAX_POINTER_BYTES + 1);
    let mut chunk = [0u8; MAX_POINTER_BYTES + 1];
    let mut filled = 0usize;
    while filled < chunk.len() {
        let n = reader.read(&mut chunk[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    head.extend_from_slice(&chunk[..filled]);

    if filled <= MAX_POINTER_BYTES {
        if let Some(ptr) = LfsPointer::parse(&head) {
            writer.write_all(&head)?;
            return Ok(ptr);
        }
    }

    let chained = Cursor::new(head).chain(reader);
    let ptr = store.store_reader(chained)?;
    writer.write_all(ptr.encode().as_bytes())?;
    Ok(ptr)
}

/// `git-lfs smudge`: if input is a canonical LFS v1 pointer, stream verified object from `store`.
/// If `skip_smudge` is true or input is not a pointer, passes input through unchanged.
/// Returns `Ok(true)` if smudged from store, `Ok(false)` if passed through unchanged.
pub fn smudge<R: Read, W: Write>(
    store: &LfsStore,
    mut reader: R,
    mut writer: W,
    skip_smudge: bool,
) -> Result<bool, LfsError> {
    let mut head = vec![0u8; MAX_POINTER_BYTES + 1];
    let mut filled = 0usize;
    while filled < head.len() {
        let n = reader.read(&mut head[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    head.truncate(filled);

    if filled <= MAX_POINTER_BYTES {
        if let Some(ptr) = LfsPointer::parse(&head) {
            if skip_smudge {
                writer.write_all(&head)?;
                return Ok(false);
            }
            store.copy_verified(&ptr.oid, Some(ptr.size), &mut writer)?;
            return Ok(true);
        }
    }

    writer.write_all(&head)?;
    std::io::copy(&mut reader, &mut writer)?;
    Ok(false)
}

/// Run the Git long-running `filter-process` protocol v2 over `reader` / `writer` using pkt-lines.
pub fn filter_process<R: Read, W: Write>(
    store: &LfsStore,
    mut reader: R,
    mut writer: W,
    skip_smudge: bool,
) -> Result<(), LfsError> {
    // 1. Handshake: expect `git-filter-client\n` and `version=2\n` followed by flush.
    let hello = read_pkt_lines_until_flush(&mut reader)?;
    if !hello.iter().any(|l| l == b"git-filter-client\n")
        || !hello.iter().any(|l| l == b"version=2\n")
    {
        return Err(LfsError::Protocol(
            "filter-process: expected git-filter-client version=2 handshake".into(),
        ));
    }
    write_pkt_line(&mut writer, b"git-filter-server\n")?;
    write_pkt_line(&mut writer, b"version=2\n")?;
    write_flush(&mut writer)?;
    writer.flush()?;

    // 2. Capability negotiation: read client capabilities until flush, advertise supported subset.
    let client_caps = read_pkt_lines_until_flush(&mut reader)?;
    for cap in &client_caps {
        if cap == b"capability=clean\n" || cap == b"capability=smudge\n" {
            write_pkt_line(&mut writer, cap)?;
        }
    }
    write_flush(&mut writer)?;
    writer.flush()?;

    // 3. Request loop until EOF.
    while let Some(headers) = read_pkt_lines_until_flush_or_eof(&mut reader)? {
        let mut command = String::new();
        for h in headers {
            if let Ok(s) = std::str::from_utf8(&h) {
                let line = s.trim_end_matches('\n');
                if let Some(cmd) = line.strip_prefix("command=") {
                    command = cmd.to_string();
                }
            }
        }
        // Read content payload until flush.
        let mut input = Vec::new();
        loop {
            match read_pkt_line(&mut reader)? {
                Pkt::Flush | Pkt::Eof => break,
                Pkt::Data(d) => input.extend_from_slice(&d),
            }
        }

        let mut output = Vec::new();
        let res = match command.as_str() {
            "clean" => clean(store, Cursor::new(&input), &mut output).map(|_| ()),
            "smudge" => smudge(store, Cursor::new(&input), &mut output, skip_smudge).map(|_| ()),
            other => Err(LfsError::Protocol(format!(
                "filter-process: unsupported command {other:?}"
            ))),
        };

        match res {
            Ok(()) => {
                write_pkt_line(&mut writer, b"status=success\n")?;
                write_flush(&mut writer)?;
                for chunk in output.chunks(MAX_PKT_DATA) {
                    write_pkt_line(&mut writer, chunk)?;
                }
                write_flush(&mut writer)?;
                write_flush(&mut writer)?;
                writer.flush()?;
            }
            Err(_) => {
                write_pkt_line(&mut writer, b"status=error\n")?;
                write_flush(&mut writer)?;
                writer.flush()?;
            }
        }
    }
    Ok(())
}

enum Pkt {
    Eof,
    Flush,
    Data(Vec<u8>),
}

fn read_pkt_line<R: Read>(reader: &mut R) -> Result<Pkt, LfsError> {
    let mut len_hex = [0u8; 4];
    let mut read_so_far = 0;
    while read_so_far < 4 {
        let n = reader.read(&mut len_hex[read_so_far..])?;
        if n == 0 {
            if read_so_far == 0 {
                return Ok(Pkt::Eof);
            }
            return Err(LfsError::Protocol(
                "filter-process: truncated pkt-line".into(),
            ));
        }
        read_so_far += n;
    }
    let hex_str = std::str::from_utf8(&len_hex)
        .map_err(|_| LfsError::Protocol("filter-process: non-ascii pkt-line length".into()))?;
    let total = usize::from_str_radix(hex_str, 16).map_err(|_| {
        LfsError::Protocol(format!("filter-process: bad pkt-line length {hex_str:?}"))
    })?;
    if total == 0 {
        return Ok(Pkt::Flush);
    }
    if total < 4 {
        return Err(LfsError::Protocol(format!(
            "filter-process: invalid pkt-line length {total}"
        )));
    }
    let mut data = vec![0u8; total - 4];
    reader.read_exact(&mut data)?;
    Ok(Pkt::Data(data))
}

fn read_pkt_lines_until_flush<R: Read>(reader: &mut R) -> Result<Vec<Vec<u8>>, LfsError> {
    read_pkt_lines_until_flush_or_eof(reader)?
        .ok_or_else(|| LfsError::Protocol("filter-process: unexpected EOF".into()))
}

fn read_pkt_lines_until_flush_or_eof<R: Read>(
    reader: &mut R,
) -> Result<Option<Vec<Vec<u8>>>, LfsError> {
    let mut lines = Vec::new();
    loop {
        match read_pkt_line(reader)? {
            Pkt::Eof => {
                if lines.is_empty() {
                    return Ok(None);
                }
                return Err(LfsError::Protocol(
                    "filter-process: unexpected EOF before flush".into(),
                ));
            }
            Pkt::Flush => return Ok(Some(lines)),
            Pkt::Data(d) => lines.push(d),
        }
    }
}

pub fn write_pkt_line<W: Write>(writer: &mut W, data: &[u8]) -> Result<(), LfsError> {
    let total = data.len() + 4;
    write!(writer, "{total:04x}")?;
    writer.write_all(data)?;
    Ok(())
}

pub fn write_flush<W: Write>(writer: &mut W) -> Result<(), LfsError> {
    writer.write_all(b"0000")?;
    Ok(())
}
