//! Pack version 2 decode and undeltified encode.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack>

mod decode;
mod encode;
pub(crate) mod entry;
pub(crate) mod header;

pub use decode::{DecodedPack, ResolvedObject};
pub use encode::EncodedPack;
