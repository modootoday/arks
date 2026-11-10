//! Git index file read/write (M3 Phase R).
//!
//! Primary: <https://git-scm.com/docs/gitformat-index>
//! Cite: `.spec/plan/20261108090648-m3-refs-index-repo.plan.md`
//! No `gix`. No product lease API (L1+).

#![forbid(unsafe_code)]

mod checksum;
mod entry;
mod error;
mod extension;
mod header;
mod index;
mod lock;

pub use entry::{CE_EXTENDED, CE_STAGEMASK, CE_VALID, IndexEntry, cmp_entries, entries_sorted};
pub use error::IndexError;
pub use extension::IndexExtension;
pub use index::{Index, sort_entries};
pub use lock::{index_lock_path, write_index_locked};

#[cfg(test)]
mod tests;
