//! Pack and idx decode/encode errors.

use arks_object::ObjectError;
use arks_oid::OidError;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PackError {
    #[error("pack or idx file truncated")]
    Truncated,
    #[error("trailing bytes after checksum")]
    Trailing,
    #[error("bad pack signature (expected PACK)")]
    BadPackMagic,
    #[error("bad idx signature")]
    BadIdxMagic,
    #[error("unsupported pack version: {0}")]
    UnsupportedPackVersion(u32),
    #[error("unsupported idx version: {0}")]
    UnsupportedIdxVersion(u32),
    #[error("invalid pack object type {0}")]
    InvalidType(u8),
    #[error("pack object type 5 is reserved")]
    ReservedType,
    #[error("size encoding overflow")]
    SizeOverflow,
    #[error("ofs-delta offset encoding overflow")]
    OffsetOverflow,
    #[error("ofs-delta base offset is invalid")]
    BadOfsOffset,
    #[error("inflated length {actual} != declared size {declared}")]
    SizeMismatch { declared: u64, actual: usize },
    #[error("zlib stream: {0}")]
    Zlib(String),
    #[error("bad delta: {0}")]
    BadDelta(&'static str),
    #[error("delta base is not in the pack")]
    DeltaBaseMissing,
    #[error("delta cycle")]
    DeltaCycle,
    #[error("pack checksum mismatch")]
    PackChecksum,
    #[error("idx checksum mismatch")]
    IdxChecksum,
    #[error("checksum length does not match object format")]
    BadChecksumLength,
    #[error("OID width does not match object format")]
    OidWidth,
    #[error("idx fanout table does not match object names")]
    Fanout,
    #[error("idx object names are not strictly sorted")]
    UnsortedNames,
    #[error("idx file is corrupt")]
    CorruptIdx,
    #[error("idx large-offset index out of range")]
    LargeOffset,
    #[error("duplicate object id in pack index")]
    DuplicateOid,
    #[error("object count does not fit the pack format")]
    CountOverflow,
    #[error(transparent)]
    Object(#[from] ObjectError),
    #[error(transparent)]
    Oid(#[from] OidError),
    #[error("bundle: {0}")]
    Bundle(String),
}
