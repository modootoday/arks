//! Packfile and pack index codec (claimed M4 subset).
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack>
//! Cite: `.spec/sot/arks-pack.sot.md`,
//! `.spec/research/20261108071913-git-objects-pack.research.md`
//!
//! Decode pack version 2 (undeltified, ofs-delta, ref-delta) and idx version 2.
//! Encode writes undeltified pack version 2 plus a matching idx version 2.
//! Object ids are reconstructed as `type SP size NUL` + payload via `arks-object`.
//! SHA-256 widths are accepted by the API; M4 fixtures claim SHA-1.
//!
//! Out of scope here: MIDX, `.rev`, pack version 3, thin-pack product claims.
//! No `gix`. No product-path `git`.

#![forbid(unsafe_code)]

mod bundle;
mod checksum;
mod delta;
mod error;
mod idx;
mod pack;

pub use bundle::{BundleFile, encode_bundle, parse_bundle};
pub use error::PackError;
pub use idx::{PackIndex, PackIndexEntry};
pub use pack::{DecodedPack, EncodedPack, ResolvedObject};

#[cfg(test)]
mod tests;
