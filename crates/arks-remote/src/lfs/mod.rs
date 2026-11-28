//! Native Git LFS v1 client library (`arks-remote`, E6).
//!
//! Primary references:
//! - <https://github.com/git-lfs/git-lfs/blob/main/docs/spec.md>
//! - <https://github.com/git-lfs/git-lfs/blob/main/docs/api/batch.md>
//! - <https://github.com/git-lfs/git-lfs/blob/main/docs/api/basic-transfers.md>
//!
//! Spec: `sot:root/v0-2-0-scope-expansion` §2.7 (E6).

pub mod batch;
pub mod filter;
pub mod pointer;
pub mod store;

pub use batch::{
    BatchObjectResponse, BatchOperation, BatchResponse, LFS_MEDIA_TYPE, TransferAction,
    batch_request, download_objects, endpoint_from_remote_url, upload_objects,
};
pub use filter::{clean, filter_process, smudge};
pub use pointer::{LFS_POINTER_VERSION, LfsPointer};
pub use store::LfsStore;

#[derive(Debug, thiserror::Error)]
pub enum LfsError {
    #[error("invalid LFS sha256 oid: {0}")]
    InvalidOid(String),
    #[error("LFS object not found in local store: {oid} (size {size})")]
    ObjectNotFound { oid: String, size: u64 },
    #[error("LFS SHA-256 mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("LFS size mismatch: expected {expected} bytes, got {actual} bytes")]
    SizeMismatch { expected: u64, actual: u64 },
    #[error("unsupported Git LFS transfer adapter: {0}")]
    UnsupportedTransfer(String),
    #[error("LFS protocol error: {0}")]
    Protocol(String),
    #[error("LFS HTTP error: {0}")]
    Http(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lfs::filter::{write_flush, write_pkt_line};
    use std::io::Cursor;
    use tempfile::TempDir;

    #[test]
    fn clean_smudge_and_filter_process_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = LfsStore::open(tmp.path());
        let payload = b"binary-like-large-asset-content-0123456789\n";

        // clean -> pointer
        let mut ptr_buf = Vec::new();
        let ptr = clean(&store, Cursor::new(payload), &mut ptr_buf).unwrap();
        assert_eq!(ptr.size, payload.len() as u64);
        assert!(store.exists(&ptr.oid));

        // clean is idempotent on an already-cleaned pointer
        let mut ptr_buf2 = Vec::new();
        let ptr2 = clean(&store, Cursor::new(&ptr_buf), &mut ptr_buf2).unwrap();
        assert_eq!(ptr2, ptr);
        assert_eq!(ptr_buf2, ptr_buf);

        // smudge -> original bytes
        let mut out = Vec::new();
        let smudged = smudge(&store, Cursor::new(&ptr_buf), &mut out, false).unwrap();
        assert!(smudged);
        assert_eq!(out, payload);

        // filter-process pkt-line v2 handshake + smudge request
        let mut req = Vec::new();
        write_pkt_line(&mut req, b"git-filter-client\n").unwrap();
        write_pkt_line(&mut req, b"version=2\n").unwrap();
        write_flush(&mut req).unwrap();
        write_pkt_line(&mut req, b"capability=clean\n").unwrap();
        write_pkt_line(&mut req, b"capability=smudge\n").unwrap();
        write_flush(&mut req).unwrap();
        write_pkt_line(&mut req, b"command=smudge\n").unwrap();
        write_pkt_line(&mut req, b"pathname=assets/model.bin\n").unwrap();
        write_flush(&mut req).unwrap();
        write_pkt_line(&mut req, &ptr_buf).unwrap();
        write_flush(&mut req).unwrap();

        let mut resp = Vec::new();
        filter_process(&store, Cursor::new(req), &mut resp, false).unwrap();
        let resp_str = String::from_utf8_lossy(&resp);
        assert!(resp_str.contains("git-filter-server\n"));
        assert!(resp_str.contains("status=success\n"));
        assert!(resp_str.contains("binary-like-large-asset-content-0123456789\n"));
    }

    #[test]
    fn store_verified_rejects_tampered_payload() {
        let tmp = TempDir::new().unwrap();
        let store = LfsStore::open(tmp.path());
        let ptr = store.store_reader(Cursor::new(b"good payload")).unwrap();
        let err = store
            .store_verified_reader(&ptr.oid, Some(ptr.size), Cursor::new(b"evil payload"))
            .unwrap_err();
        assert!(matches!(err, LfsError::HashMismatch { .. }));
    }
}
