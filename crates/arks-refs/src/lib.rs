//! Git refs: refnames, loose/packed-refs, HEAD/symref, L0 locks (M3 Phase R).
//!
//! Primary: <https://git-scm.com/docs/gitrepository-layout>
//! Cite: `.spec/plan/20261108090648-m3-refs-index-repo.plan.md`
//! No `gix`. No product lease API (L1+).

#![forbid(unsafe_code)]

mod error;
mod lock;
mod loose;
mod packed;
mod refname;
mod store;
mod target;

pub use error::RefsError;
pub use lock::{delete_ref_locked, ref_lock_path, write_ref_locked};
pub use refname::{is_valid_refname, is_valid_refname_allow_onelevel};
pub use store::FilesystemRefStore;
pub use target::RefTarget;
