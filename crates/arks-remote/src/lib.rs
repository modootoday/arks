//! Generic Git remote **client** (M6 Wave H + M16 Sy-4 SSH + `git://` + R2).
//!
//! Claimed subset: `http://` and `https://` smart HTTP protocol v1
//! (`info/refs` + `git-upload-pack` / `git-receive-pack`), additive dumb
//! HTTP fetch/clone fallback and WebDAV dumb push ([gitprotocol-http] /
//! [git-http-push] / RFC 4918), OpenSSH client transport for `ssh://` and
//! SCP-like `user@host:path`, `git://` daemon pack-protocol client
//! (clone/fetch/push), fetch, clone, push with `report-status` /
//! `delete-refs` when advertised, remote config, additive local path /
//! same-host `file://` clone via object-directory copy, G9/G35
//! `--depth N` shallow on pack transports, and R3/S1
//! `--filter=blob:none|tree:0` partial-clone (promisor + extensions.partialClone).
//! GitHub-class hosts are a verify-peer label, not a special code path.
//!
//! Primary:
//! - <https://git-scm.com/docs/gitprotocol-http>
//! - <https://git-scm.com/docs/gitprotocol-pack>
//! - <https://git-scm.com/docs/gitprotocol-capabilities>
//! - <https://git-scm.com/docs/partial-clone>
//! - <https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---depthltdepthgt>
//!
//! Cite: `.spec/sot/arks-remote.sot.md`,
//! `.spec/sot/remote-host-compatibility-surface.sot.md`,
//! `.spec/sot/cli-flag-parity-complete.sot.md` (Sy-4)
//!
//! No `gix`. No CELIR. No lease runtime. No product server. No forge APIs.
//! No product-path `git` subprocess (OpenSSH `ssh` helper is claimed).

#![deny(unsafe_code)]

mod advertise;
mod checkout;
mod clone;
mod config;
mod credential;
mod dumb;
mod error;
mod fetch;
mod git;
mod http;
mod lfs;
mod push;
mod shallow;
mod ssh;
mod transport;
mod url;
mod walk;
#[allow(unsafe_code)] // Windows Cred* FFI for first-party wincred helper only.
mod wallet;

pub use checkout::{
    checkout_commit, checkout_tree, missing_blob_oids, missing_checkout_oids, smudge_worktree_blob,
};
pub use clone::{
    clone_git, clone_git_with, clone_http, clone_http_with, clone_local, clone_local_with,
    clone_ssh, clone_ssh_with,
};
pub use config::{RemoteConfig, add_remote, list_remotes, remove_remote};
pub use credential::{
    Credential, CredentialSource, approve as credential_approve, fill as credential_fill,
    reject as credential_reject,
};
pub use error::RemoteError;
pub use fetch::{
    FetchOptions, FetchOutcome, fetch_git, fetch_git_with, fetch_http, fetch_http_with,
    fetch_remote, fetch_remote_with, fetch_ssh, fetch_ssh_with, validate_claimed_filter,
};
pub use http::http_get;
pub use lfs::{
    LfsLock, LfsPointer, discover_lfs_endpoint, lfs_batch_download, lfs_batch_upload,
    lfs_lock_create, lfs_lock_unlock, lfs_locks_list, lfs_locks_verify, lfs_object_path,
    parse_lfs_pointer, read_local_lfs_object, resolve_lfs_pointer, write_local_lfs_object,
};
pub use push::{
    PushOptions, PushUpdate, push_git, push_git_with, push_http, push_http_with, push_remote,
    push_remote_with, push_ssh, push_ssh_with,
};
pub use transport::{HttpRequest, HttpResponse, RecordedExchange, RecordedTransport, Transport};
pub use url::{GitDaemonUrl, HttpRemoteUrl, LocalRemoteUrl, SshRemoteUrl};
pub use wallet::{WalletKind, cmd_wallet_helper, is_os_wallet_helper, wallet_helper_kind};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
