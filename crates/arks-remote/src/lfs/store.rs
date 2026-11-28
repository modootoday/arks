//! Content-addressed local Git LFS object store (`.git/lfs/objects/{aa}/{bb}/{oid}`).
//!
//! Primary reference: <https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md>
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.7 (E6).

use crate::lfs::LfsError;
use crate::lfs::pointer::LfsPointer;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Local Git LFS object storage rooted at `<common_dir>/lfs`.
#[derive(Debug, Clone)]
pub struct LfsStore {
    lfs_dir: PathBuf,
}

impl LfsStore {
    pub fn open(common_dir: impl AsRef<Path>) -> Self {
        Self {
            lfs_dir: common_dir.as_ref().join("lfs"),
        }
    }

    pub fn lfs_dir(&self) -> &Path {
        &self.lfs_dir
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.lfs_dir.join("objects")
    }

    pub fn tmp_dir(&self) -> PathBuf {
        self.lfs_dir.join("tmp")
    }

    /// Canonical sharded object path: `<lfs_dir>/objects/<aa>/<bb>/<oid>`.
    pub fn object_path(&self, oid: &str) -> Result<PathBuf, LfsError> {
        if !LfsPointer::is_valid_oid(oid) {
            return Err(LfsError::InvalidOid(oid.to_string()));
        }
        Ok(self
            .objects_dir()
            .join(&oid[0..2])
            .join(&oid[2..4])
            .join(oid))
    }

    pub fn exists(&self, oid: &str) -> bool {
        self.object_path(oid).is_ok_and(|p| p.is_file())
    }

    /// Stream `reader` to a temp file while computing SHA-256, then atomically commit to store.
    pub fn store_reader<R: Read>(&self, reader: R) -> Result<LfsPointer, LfsError> {
        self.store_inner(reader, None, None)
    }

    /// Stream `reader` while verifying exact SHA-256 (and optional size) before committing.
    /// Fails closed and deletes temp file on any mismatch.
    pub fn store_verified_reader<R: Read>(
        &self,
        expected_oid: &str,
        expected_size: Option<u64>,
        reader: R,
    ) -> Result<LfsPointer, LfsError> {
        if !LfsPointer::is_valid_oid(expected_oid) {
            return Err(LfsError::InvalidOid(expected_oid.to_string()));
        }
        self.store_inner(reader, Some(expected_oid), expected_size)
    }

    fn store_inner<R: Read>(
        &self,
        mut reader: R,
        expected_oid: Option<&str>,
        expected_size: Option<u64>,
    ) -> Result<LfsPointer, LfsError> {
        let tmp_dir = self.tmp_dir();
        fs::create_dir_all(&tmp_dir)?;
        let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let tmp_path = tmp_dir.join(format!("tmp-{}-{seq}", std::process::id()));

        let res = (|| -> Result<LfsPointer, LfsError> {
            let mut out = File::create(&tmp_path)?;
            let mut hasher = Sha256::new();
            let mut buf = [0u8; 64 * 1024];
            let mut total: u64 = 0;
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                out.write_all(&buf[..n])?;
                total += n as u64;
            }
            out.sync_all()?;
            drop(out);

            let digest = hasher.finalize();
            let oid = hex_lower(&digest);

            if let Some(exp) = expected_oid {
                if oid != exp {
                    return Err(LfsError::HashMismatch {
                        expected: exp.to_string(),
                        actual: oid,
                    });
                }
            }
            if let Some(exp_sz) = expected_size {
                if total != exp_sz {
                    return Err(LfsError::SizeMismatch {
                        expected: exp_sz,
                        actual: total,
                    });
                }
            }

            let dst = self.object_path(&oid)?;
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            if dst.is_file() {
                let _ = fs::remove_file(&tmp_path);
            } else {
                fs::rename(&tmp_path, &dst)?;
            }
            Ok(LfsPointer { oid, size: total })
        })();

        if res.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        res
    }

    /// Stream object `oid` to `writer` while verifying SHA-256 integrity on read.
    pub fn copy_verified<W: Write>(
        &self,
        oid: &str,
        expected_size: Option<u64>,
        mut writer: W,
    ) -> Result<u64, LfsError> {
        let path = self.object_path(oid)?;
        if !path.is_file() {
            return Err(LfsError::ObjectNotFound {
                oid: oid.to_string(),
                size: expected_size.unwrap_or(0),
            });
        }
        let mut f = File::open(&path)?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 64 * 1024];
        let mut total: u64 = 0;
        let mut chunks: Vec<Vec<u8>> = Vec::new();
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            chunks.push(buf[..n].to_vec());
            total += n as u64;
        }
        let actual = hex_lower(&hasher.finalize());
        if actual != oid {
            return Err(LfsError::HashMismatch {
                expected: oid.to_string(),
                actual,
            });
        }
        if let Some(exp_sz) = expected_size {
            if total != exp_sz {
                return Err(LfsError::SizeMismatch {
                    expected: exp_sz,
                    actual: total,
                });
            }
        }
        for c in chunks {
            writer.write_all(&c)?;
        }
        Ok(total)
    }

    /// Read full verified bytes of `oid`.
    pub fn read_verified(
        &self,
        oid: &str,
        expected_size: Option<u64>,
    ) -> Result<Vec<u8>, LfsError> {
        let mut out = Vec::new();
        self.copy_verified(oid, expected_size, &mut out)?;
        Ok(out)
    }

    /// List all stored object OIDs sorted lexicographically.
    pub fn list_objects(&self) -> Result<Vec<String>, LfsError> {
        let mut out = Vec::new();
        let root = self.objects_dir();
        if !root.is_dir() {
            return Ok(out);
        }
        for l1 in fs::read_dir(&root)?.flatten() {
            if !l1.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            for l2 in fs::read_dir(l1.path())?.flatten() {
                if !l2.file_type().is_ok_and(|t| t.is_dir()) {
                    continue;
                }
                for obj in fs::read_dir(l2.path())?.flatten() {
                    if obj.file_type().is_ok_and(|t| t.is_file()) {
                        let name = obj.file_name().to_string_lossy().to_string();
                        if LfsPointer::is_valid_oid(&name) {
                            out.push(name);
                        }
                    }
                }
            }
        }
        out.sort();
        Ok(out)
    }
}

pub fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}
