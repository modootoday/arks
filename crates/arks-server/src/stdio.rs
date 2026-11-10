//! Duplex pack-protocol sessions (SSH command-mode / local / git://).
//!
//! Write ref advertisement, negotiate want/have/done, then write pack.
//! Intermediate `NAK` on flush when peer is waiting; skip a second NAK before
//! the side-band pack (avoids `bad band #78`). Cite: gitprotocol-pack.

use crate::error::ServerError;
use crate::receive::{receive_pack, receive_pack_advertisement};
use crate::upload::{upload_pack_advertisement, upload_pack_result_with_nak};
use arks_protocol::{Packet, decode_packet, decode_text_line, encode_text_line};
use arks_repo::Repository;
use std::io::{Read, Write};

/// Separate read/write halves (CLI stdio).
pub fn upload_pack_session<R: Read, W: Write>(
    repo: &Repository,
    mut stdin: R,
    mut stdout: W,
) -> Result<(), ServerError> {
    let ad = upload_pack_advertisement(repo)?;
    stdout.write_all(&ad)?;
    stdout.flush()?;
    let (body, sent_nak) = read_upload_split(&mut stdin, &mut stdout)?;
    let out = upload_pack_result_with_nak(repo, &body, !sent_nak)?;
    stdout.write_all(&out)?;
    stdout.flush()?;
    Ok(())
}

pub fn receive_pack_session<R: Read, W: Write>(
    repo: &mut Repository,
    mut stdin: R,
    mut stdout: W,
) -> Result<(), ServerError> {
    let ad = receive_pack_advertisement(repo)?;
    stdout.write_all(&ad)?;
    stdout.flush()?;
    let mut body = Vec::new();
    stdin.read_to_end(&mut body)?;
    let out = receive_pack(repo, &body)?;
    stdout.write_all(&out)?;
    stdout.flush()?;
    Ok(())
}

/// Single duplex stream (git:// TcpStream).
pub fn upload_pack_stream<S: Read + Write>(
    repo: &Repository,
    stream: &mut S,
) -> Result<(), ServerError> {
    let ad = upload_pack_advertisement(repo)?;
    stream.write_all(&ad)?;
    stream.flush()?;
    let (body, sent_nak) = read_upload_one(stream)?;
    let out = upload_pack_result_with_nak(repo, &body, !sent_nak)?;
    stream.write_all(&out)?;
    stream.flush()?;
    Ok(())
}

pub fn receive_pack_stream<S: Read + Write>(
    repo: &mut Repository,
    stream: &mut S,
) -> Result<(), ServerError> {
    let ad = receive_pack_advertisement(repo)?;
    stream.write_all(&ad)?;
    stream.flush()?;
    let mut body = Vec::new();
    stream.read_to_end(&mut body)?;
    let out = receive_pack(repo, &body)?;
    stream.write_all(&out)?;
    stream.flush()?;
    Ok(())
}

fn read_upload_split<R: Read, W: Write>(
    stdin: &mut R,
    stdout: &mut W,
) -> Result<(Vec<u8>, bool), ServerError> {
    let mut body = Vec::new();
    let mut buf = [0u8; 8192];
    let mut sent_nak = false;
    let mut nak_for_flush_at = None::<usize>;
    loop {
        while body.len() < 4 || incomplete_last_packet(&body) {
            let n = stdin.read(&mut buf)?;
            if n == 0 {
                return Ok((body, sent_nak));
            }
            body.extend_from_slice(&buf[..n]);
        }
        if request_has_done(&body) {
            return Ok((body, sent_nak));
        }
        if let Some(flush_end) = trailing_flush_end(&body) {
            if nak_for_flush_at != Some(flush_end) {
                let nak = encode_text_line("NAK")
                    .map_err(|e| ServerError::Protocol(e.to_string()))?;
                stdout.write_all(&nak)?;
                stdout.flush()?;
                sent_nak = true;
                nak_for_flush_at = Some(flush_end);
            }
        }
        let n = stdin.read(&mut buf)?;
        if n == 0 {
            return Ok((body, sent_nak));
        }
        body.extend_from_slice(&buf[..n]);
    }
}

fn read_upload_one<S: Read + Write>(stream: &mut S) -> Result<(Vec<u8>, bool), ServerError> {
    let mut body = Vec::new();
    let mut buf = [0u8; 8192];
    let mut sent_nak = false;
    let mut nak_for_flush_at = None::<usize>;
    loop {
        while body.len() < 4 || incomplete_last_packet(&body) {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                return Ok((body, sent_nak));
            }
            body.extend_from_slice(&buf[..n]);
        }
        if request_has_done(&body) {
            return Ok((body, sent_nak));
        }
        if let Some(flush_end) = trailing_flush_end(&body) {
            if nak_for_flush_at != Some(flush_end) {
                let nak = encode_text_line("NAK")
                    .map_err(|e| ServerError::Protocol(e.to_string()))?;
                stream.write_all(&nak)?;
                stream.flush()?;
                sent_nak = true;
                nak_for_flush_at = Some(flush_end);
            }
        }
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Ok((body, sent_nak));
        }
        body.extend_from_slice(&buf[..n]);
    }
}

fn request_has_done(body: &[u8]) -> bool {
    let mut offset = 0;
    while offset + 4 <= body.len() {
        let Ok((packet, n)) = decode_packet(&body[offset..]) else {
            return false;
        };
        offset += n;
        if let Packet::Data(payload) = packet {
            if let Ok(text) = decode_text_line(&payload) {
                if text == "done" || text.starts_with("done ") {
                    return true;
                }
            }
        }
    }
    false
}

fn trailing_flush_end(body: &[u8]) -> Option<usize> {
    let mut offset = 0;
    let mut last_flush_end = None;
    while offset + 4 <= body.len() {
        match decode_packet(&body[offset..]) {
            Ok((packet, n)) => {
                offset += n;
                last_flush_end = match packet {
                    Packet::Flush => Some(offset),
                    _ => None,
                };
            }
            Err(_) => return None,
        }
    }
    if offset != body.len() {
        return None;
    }
    last_flush_end
}

fn incomplete_last_packet(body: &[u8]) -> bool {
    let mut offset = 0;
    while offset + 4 <= body.len() {
        match decode_packet(&body[offset..]) {
            Ok((_, n)) => offset += n,
            Err(_) => return true,
        }
    }
    offset != body.len()
}
