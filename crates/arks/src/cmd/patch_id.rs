//! `git patch-id`: ids of patches read from stdin.
//!
//! A patch id is the SHA-1 of the patch text with `index` lines, hunk
//! headers (line numbers) and, unless `--verbatim`, `\ No newline` markers
//! and all whitespace removed. `--unstable` (the default) hashes the whole
//! patch; `--stable` hashes each file separately and adds the digests, so
//! file order does not matter. Each id is printed with the commit id taken
//! from the preceding `commit <id>` / `From <id>` line (zeros if none).
//!
//! Primary: <https://git-scm.com/docs/git-patch-id>.

use crate::error::{CliError, CliResult};
use arks_hash::{Hasher, ObjectFormat};
use std::io::{self, Read, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Unstable,
    Stable,
    Verbatim,
}

struct Acc {
    mode: Mode,
    whole: Hasher,
    file: Hasher,
    file_has: bool,
    total: [u8; 20],
    any: bool,
}

impl Acc {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            whole: Hasher::new(ObjectFormat::Sha1),
            file: Hasher::new(ObjectFormat::Sha1),
            file_has: false,
            total: [0; 20],
            any: false,
        }
    }

    fn flush_file(&mut self) {
        if !self.file_has {
            return;
        }
        let d = std::mem::replace(&mut self.file, Hasher::new(ObjectFormat::Sha1))
            .finalize()
            .map(|d| d.as_bytes().to_vec())
            .unwrap_or_default();
        let mut carry: u32 = 0;
        for (t, b) in self.total.iter_mut().zip(d.iter()) {
            carry += *t as u32 + *b as u32;
            *t = carry as u8;
            carry >>= 8;
        }
        self.file_has = false;
    }

    fn add(&mut self, line: &[u8]) {
        let data: Vec<u8> = if self.mode == Mode::Verbatim {
            line.to_vec()
        } else {
            line.iter()
                .copied()
                .filter(|c| !c.is_ascii_whitespace())
                .collect()
        };
        self.whole.update(&data);
        self.file.update(&data);
        self.file_has = true;
        self.any = true;
    }

    fn finish(mut self) -> Option<String> {
        if !self.any {
            return None;
        }
        self.flush_file();
        let bytes: Vec<u8> = match self.mode {
            Mode::Unstable => self
                .whole
                .finalize()
                .map(|d| d.as_bytes().to_vec())
                .unwrap_or_default(),
            _ => self.total.to_vec(),
        };
        Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
    }
}

/// Patch ids of every patch in `text` as `(patch id, commit id)`.
pub fn patch_ids(text: &[u8], mode: Mode) -> Vec<(String, String)> {
    let zero = "0".repeat(40);
    let mut out = Vec::new();
    let mut commit = zero.clone();
    let mut acc: Option<Acc> = None;
    let mut in_diff = false;
    let mut file_has_hunk = false;
    let mut pending_header = false;
    for line in text.split_inclusive(|c| *c == b'\n') {
        let s = String::from_utf8_lossy(line);
        let new_commit = s
            .strip_prefix("commit ")
            .or_else(|| s.strip_prefix("From "))
            .map(|r| r.split_whitespace().next().unwrap_or("").to_string())
            .filter(|h| h.len() >= 40 && h.bytes().all(|b| b.is_ascii_hexdigit()));
        if let Some(h) = new_commit {
            if let Some(a) = acc.take() {
                if let Some(id) = a.finish() {
                    out.push((id, commit.clone()));
                }
            }
            commit = h;
            in_diff = false;
            continue;
        }
        if s.starts_with("diff ") {
            let a = acc.get_or_insert_with(|| Acc::new(mode));
            // Header-only files (pure renames, mode changes) share the
            // hash of the next file that has content.
            if file_has_hunk || !pending_header {
                a.flush_file();
            }
            in_diff = true;
            file_has_hunk = false;
            pending_header = true;
        }
        if !in_diff {
            continue;
        }
        if s.starts_with("--- ") {
            file_has_hunk = true;
        }
        if s.starts_with("index ") || s.starts_with("@@ ") {
            continue;
        }
        if s.starts_with('\\') && mode != Mode::Verbatim {
            continue;
        }
        if let Some(a) = acc.as_mut() {
            a.add(line);
        }
    }
    if let Some(a) = acc.take() {
        if let Some(id) = a.finish() {
            out.push((id, commit));
        }
    }
    out
}

pub fn cmd_patch_id(args: &[String]) -> CliResult<()> {
    let repo = crate::util::open_repo().ok();
    let cfg = arks_config::ConfigSet::load(repo.as_ref().map(|r| r.common_dir()));
    let mut mode = if cfg.get_bool("patchid.verbatim") == Some(true) {
        Mode::Verbatim
    } else if cfg.get_bool("patchid.stable") == Some(true) {
        Mode::Stable
    } else {
        Mode::Unstable
    };
    for a in args {
        match a.as_str() {
            "--stable" => mode = Mode::Stable,
            "--unstable" => mode = Mode::Unstable,
            "--verbatim" => mode = Mode::Verbatim,
            "--no-verbatim" => {
                if mode == Mode::Verbatim {
                    mode = Mode::Stable;
                }
            }
            x => {
                return Err(CliError::NotImplemented(format!("unsupported option: {x}")));
            }
        }
    }
    let mut buf = Vec::new();
    io::stdin().read_to_end(&mut buf)?;
    let mut out = io::stdout().lock();
    for (id, commit) in patch_ids(&buf, mode) {
        writeln!(out, "{id} {commit}")?;
    }
    Ok(())
}
