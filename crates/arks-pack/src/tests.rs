//! Host-advisory pack/idx tests. Stock Git is not invoked from this crate.

use super::*;
use crate::checksum::append_checksum;
use crate::delta::write_size;
use crate::idx::IDX_MAGIC;
use crate::pack::entry::{
    OBJ_OFS_DELTA, OBJ_REF_DELTA, type_of_kind, write_ofs_offset, write_type_and_size,
    zlib_compress,
};
use crate::pack::header::{self, HEADER_LEN};
use arks_hash::{ObjectFormat, hash_bytes};
use arks_object::{ObjectKind, RawObject};
use arks_oid::{Oid, OidKind};

fn blob(payload: &[u8]) -> RawObject {
    RawObject::new(ObjectKind::Blob, payload.to_vec())
}

fn oid_of(obj: &RawObject) -> Oid {
    obj.oid(ObjectFormat::Sha1).unwrap()
}

fn delta_copy_insert(base: &[u8], copy_off: u32, copy_len: u32, insert: &[u8]) -> Vec<u8> {
    let mut delta = Vec::new();
    let result_len = u64::from(copy_len) + insert.len() as u64;
    write_size(&mut delta, base.len() as u64);
    write_size(&mut delta, result_len);
    if copy_len > 0 {
        let mut cmd = 0x80u8;
        let mut rest = Vec::new();
        for bit in 0..4 {
            let byte = ((copy_off >> (8 * bit)) & 0xff) as u8;
            if byte != 0 {
                cmd |= 1 << bit;
                rest.push(byte);
            }
        }
        if copy_len != 0x10000 {
            for bit in 0..3 {
                let byte = ((copy_len >> (8 * bit)) & 0xff) as u8;
                if byte != 0 {
                    cmd |= 1 << (4 + bit);
                    rest.push(byte);
                }
            }
        }
        delta.push(cmd);
        delta.extend_from_slice(&rest);
    }
    let mut rest = insert;
    while !rest.is_empty() {
        let n = rest.len().min(127);
        delta.push(n as u8);
        delta.extend_from_slice(&rest[..n]);
        rest = &rest[n..];
    }
    delta
}

struct PackBuf {
    buf: Vec<u8>,
}

impl PackBuf {
    fn new(count: u32) -> Self {
        let mut buf = Vec::new();
        header::write_header(&mut buf, count);
        Self { buf }
    }

    fn undeltified(&mut self, obj: &RawObject) -> u64 {
        let offset = self.buf.len() as u64;
        write_type_and_size(
            &mut self.buf,
            type_of_kind(obj.kind),
            obj.payload.len() as u64,
        );
        let compressed = zlib_compress(&obj.payload).unwrap();
        self.buf.extend_from_slice(&compressed);
        offset
    }

    fn ofs(&mut self, base_offset: u64, delta: &[u8]) -> u64 {
        let offset = self.buf.len() as u64;
        write_type_and_size(&mut self.buf, OBJ_OFS_DELTA, delta.len() as u64);
        let rel = offset - base_offset;
        write_ofs_offset(&mut self.buf, rel).unwrap();
        let compressed = zlib_compress(delta).unwrap();
        self.buf.extend_from_slice(&compressed);
        offset
    }

    fn reference(&mut self, base: &Oid, delta: &[u8]) {
        write_type_and_size(&mut self.buf, OBJ_REF_DELTA, delta.len() as u64);
        self.buf.extend_from_slice(base.as_bytes());
        let compressed = zlib_compress(delta).unwrap();
        self.buf.extend_from_slice(&compressed);
    }

    fn finish(mut self) -> Vec<u8> {
        append_checksum(&mut self.buf, ObjectFormat::Sha1);
        self.buf
    }
}

fn synthetic_oid(first: u8, last: u8) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[0] = first;
    bytes[19] = last;
    Oid::from_bytes(OidKind::Sha1, &bytes).unwrap()
}

#[test]
fn roundtrip_undeltified_pack_and_idx() {
    let objects = [
        blob(b""),
        blob(b"hello"),
        blob(b"a\0b-and-a-longer-payload!!"),
        RawObject::new(ObjectKind::Tree, b"tree-bytes"),
        RawObject::new(ObjectKind::Commit, b"commit-bytes"),
        RawObject::new(ObjectKind::Tag, b"tag-bytes"),
    ];
    let encoded = EncodedPack::from_undeltified(ObjectFormat::Sha1, &objects).unwrap();
    let decoded = DecodedPack::decode(&encoded.pack, ObjectFormat::Sha1).unwrap();
    let index = PackIndex::decode(&encoded.idx, ObjectFormat::Sha1).unwrap();

    assert_eq!(decoded.objects.len(), objects.len());
    assert_eq!(index.entries.len(), objects.len());
    assert_eq!(index.fanout[255], objects.len() as u32);
    assert_eq!(index.pack_checksum, decoded.checksum);
    let hash_len = ObjectFormat::Sha1.digest_len();
    assert_eq!(
        hash_bytes(
            ObjectFormat::Sha1,
            &encoded.pack[..encoded.pack.len() - hash_len]
        )
        .as_bytes(),
        decoded.checksum
    );

    for obj in &objects {
        let oid = oid_of(obj);
        let got = decoded.find(&oid).unwrap();
        assert_eq!(got.object, *obj);
        assert_eq!(got.oid, oid);
        let idx = index.lookup(&oid).unwrap();
        assert_eq!(idx.offset, got.offset);
        assert_eq!(idx.crc32, got.crc32);
        let next = decoded
            .objects
            .iter()
            .map(|o| o.offset)
            .filter(|off| *off > got.offset)
            .min()
            .unwrap_or((encoded.pack.len() - hash_len) as u64);
        let packed = &encoded.pack[got.offset as usize..next as usize];
        assert_eq!(crc32fast::hash(packed), got.crc32);
    }

    assert_eq!(
        oid_of(&objects[0]).to_hex(),
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
    );
    assert_eq!(
        oid_of(&objects[1]).to_hex(),
        "b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0"
    );

    let mut bad = encoded.pack.clone();
    let last = bad.len() - 1;
    bad[last] ^= 0xff;
    assert_eq!(
        DecodedPack::decode(&bad, ObjectFormat::Sha1).unwrap_err(),
        PackError::PackChecksum
    );
}

#[test]
fn resolve_ofs_and_ref_deltas() {
    let base = blob(b"hello world");
    let base_oid = oid_of(&base);
    let there = b"hello there";
    let there_bang = b"hello there!";
    let yellow = b"yellow world";

    let there_delta = delta_copy_insert(b"hello world", 0, 6, b"there");
    let bang_delta = delta_copy_insert(there, 0, there.len() as u32, b"!");
    // Insert "yellow" then copy " world" from the base (offset 5, length 6).
    let mut yellow_delta = Vec::new();
    write_size(&mut yellow_delta, b"hello world".len() as u64);
    write_size(&mut yellow_delta, yellow.len() as u64);
    yellow_delta.push(6);
    yellow_delta.extend_from_slice(b"yellow");
    yellow_delta.extend_from_slice(&[0x91, 5, 6]);

    let mut pack = PackBuf::new(4);
    // REF appears before its base. OFS chains through an intermediate delta.
    pack.reference(&base_oid, &yellow_delta);
    let base_off = pack.undeltified(&base);
    let there_off = pack.ofs(base_off, &there_delta);
    pack.ofs(there_off, &bang_delta);
    let bytes = pack.finish();

    let decoded = DecodedPack::decode(&bytes, ObjectFormat::Sha1).unwrap();
    assert_eq!(decoded.objects.len(), 4);

    let yellow_obj = blob(yellow);
    let there_obj = blob(there);
    let bang_obj = blob(there_bang);
    assert_eq!(decoded.find(&base_oid).unwrap().object, base);
    assert_eq!(
        decoded.find(&oid_of(&yellow_obj)).unwrap().object,
        yellow_obj
    );
    assert_eq!(decoded.find(&oid_of(&there_obj)).unwrap().object, there_obj);
    assert_eq!(decoded.find(&oid_of(&bang_obj)).unwrap().object, bang_obj);
    assert_eq!(
        decoded.find(&oid_of(&there_obj)).unwrap().object.kind,
        ObjectKind::Blob
    );

    let missing = blob(b"not-in-pack");
    let mut thin = PackBuf::new(1);
    thin.reference(&oid_of(&missing), &yellow_delta);
    let thin_bytes = thin.finish();
    assert_eq!(
        DecodedPack::decode(&thin_bytes, ObjectFormat::Sha1).unwrap_err(),
        PackError::DeltaBaseMissing
    );
}

#[test]
fn idx_fanout_and_lookup() {
    let entries = vec![
        PackIndexEntry {
            oid: synthetic_oid(0x00, 1),
            crc32: 0x11,
            offset: 12,
        },
        PackIndexEntry {
            oid: synthetic_oid(0x00, 2),
            crc32: 0x22,
            offset: 100,
        },
        PackIndexEntry {
            oid: synthetic_oid(0x10, 3),
            crc32: 0x33,
            offset: 0x7fff_ffff,
        },
        PackIndexEntry {
            oid: synthetic_oid(0x10, 4),
            crc32: 0x44,
            offset: 0x8000_0000,
        },
        PackIndexEntry {
            oid: synthetic_oid(0xff, 5),
            crc32: 0x55,
            offset: 0x1_0000_0005,
        },
    ];
    // Insert out of order; from_entries sorts by object name.
    let mut shuffled = entries.clone();
    shuffled.swap(0, 4);
    let checksum = vec![0xab; 20];
    let index = PackIndex::from_entries(ObjectFormat::Sha1, shuffled, checksum.clone()).unwrap();
    assert_eq!(index.fanout[0x00], 2);
    assert_eq!(index.fanout[0x0f], 2);
    assert_eq!(index.fanout[0x10], 4);
    assert_eq!(index.fanout[0xfe], 4);
    assert_eq!(index.fanout[0xff], 5);

    let bytes = index.encode().unwrap();
    let decoded = PackIndex::decode(&bytes, ObjectFormat::Sha1).unwrap();
    assert_eq!(decoded.fanout, index.fanout);
    assert_eq!(decoded.pack_checksum, checksum);
    for entry in &entries {
        let got = decoded.lookup(&entry.oid).unwrap();
        assert_eq!(got, entry);
    }
    assert!(decoded.lookup(&synthetic_oid(0x00, 9)).is_none());
    assert!(decoded.lookup(&synthetic_oid(0x20, 1)).is_none());
    let sha256 = Oid::from_bytes(OidKind::Sha256, &[0u8; 32]).unwrap();
    assert!(decoded.lookup(&sha256).is_none());

    let mut bad = bytes.clone();
    let last = bad.len() - 1;
    bad[last] ^= 0xff;
    assert_eq!(
        PackIndex::decode(&bad, ObjectFormat::Sha1).unwrap_err(),
        PackError::IdxChecksum
    );
}

#[test]
fn refuse_invalid_magic_version_and_type() {
    let mut pack = Vec::new();
    pack.extend_from_slice(b"XXXX");
    pack.extend_from_slice(&2u32.to_be_bytes());
    pack.extend_from_slice(&0u32.to_be_bytes());
    assert_eq!(
        DecodedPack::decode(&pack, ObjectFormat::Sha1).unwrap_err(),
        PackError::BadPackMagic
    );

    for version in [0u32, 1, 3, 99] {
        let mut pack = Vec::new();
        pack.extend_from_slice(b"PACK");
        pack.extend_from_slice(&version.to_be_bytes());
        pack.extend_from_slice(&0u32.to_be_bytes());
        append_checksum(&mut pack, ObjectFormat::Sha1);
        assert_eq!(
            DecodedPack::decode(&pack, ObjectFormat::Sha1).unwrap_err(),
            PackError::UnsupportedPackVersion(version)
        );
    }

    let mut type0 = PackBuf::new(1);
    type0.buf.push(0x00);
    assert_eq!(
        DecodedPack::decode(&type0.finish(), ObjectFormat::Sha1).unwrap_err(),
        PackError::InvalidType(0)
    );
    let mut type5 = PackBuf::new(1);
    type5.buf.push(0x50);
    assert_eq!(
        DecodedPack::decode(&type5.finish(), ObjectFormat::Sha1).unwrap_err(),
        PackError::ReservedType
    );

    assert_eq!(
        PackIndex::decode(b"PACK....", ObjectFormat::Sha1).unwrap_err(),
        PackError::BadIdxMagic
    );
    let mut idx = Vec::new();
    idx.extend_from_slice(&IDX_MAGIC);
    idx.extend_from_slice(&3u32.to_be_bytes());
    assert_eq!(
        PackIndex::decode(&idx, ObjectFormat::Sha1).unwrap_err(),
        PackError::UnsupportedIdxVersion(3)
    );

    let empty = EncodedPack::from_undeltified(ObjectFormat::Sha1, &[]).unwrap();
    let decoded = DecodedPack::decode(&empty.pack, ObjectFormat::Sha1).unwrap();
    assert!(decoded.objects.is_empty());
    assert_eq!(empty.pack.len(), HEADER_LEN + 20);
    let index = PackIndex::decode(&empty.idx, ObjectFormat::Sha1).unwrap();
    assert_eq!(index.fanout, [0u32; 256]);
    assert!(index.entries.is_empty());
}

#[test]
fn undeltified_sha256_width_roundtrip() {
    // API width only. M4 fixtures claim SHA-1.
    let objects = [blob(b"sha256-width")];
    let encoded = EncodedPack::from_undeltified(ObjectFormat::Sha256, &objects).unwrap();
    let decoded = DecodedPack::decode(&encoded.pack, ObjectFormat::Sha256).unwrap();
    let index = PackIndex::decode(&encoded.idx, ObjectFormat::Sha256).unwrap();
    let oid = objects[0].oid(ObjectFormat::Sha256).unwrap();
    assert_eq!(oid.kind(), OidKind::Sha256);
    assert_eq!(decoded.find(&oid).unwrap().object, objects[0]);
    assert_eq!(
        index.lookup(&oid).unwrap().offset,
        decoded.objects[0].offset
    );
    assert_eq!(index.pack_checksum.len(), 32);
}

#[test]
fn crc32_matches_zlib_check_value() {
    // ISO-HDLC / zlib CRC-32 of "123456789".
    assert_eq!(crc32fast::hash(b"123456789"), 0xcbf4_3926);
}

#[test]
fn duplicate_oid_is_refused_on_encode() {
    let objects = [blob(b"same"), blob(b"same")];
    assert_eq!(
        EncodedPack::from_undeltified(ObjectFormat::Sha1, &objects).unwrap_err(),
        PackError::DuplicateOid
    );
}

#[test]
fn idx_fanout_mismatch_and_unsorted_names() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&IDX_MAGIC);
    bytes.extend_from_slice(&2u32.to_be_bytes());
    let fanout = [1u32; 256];
    for count in fanout {
        bytes.extend_from_slice(&count.to_be_bytes());
    }
    // Name starts with 0x10, but fanout[0] already counts it.
    bytes.extend_from_slice(synthetic_oid(0x10, 1).as_bytes());
    bytes.extend_from_slice(&1u32.to_be_bytes());
    bytes.extend_from_slice(&12u32.to_be_bytes());
    bytes.extend_from_slice(&[0xcd; 20]);
    append_checksum(&mut bytes, ObjectFormat::Sha1);
    assert_eq!(
        PackIndex::decode(&bytes, ObjectFormat::Sha1).unwrap_err(),
        PackError::Fanout
    );

    let mut unsorted = Vec::new();
    unsorted.extend_from_slice(&IDX_MAGIC);
    unsorted.extend_from_slice(&2u32.to_be_bytes());
    let mut fanout = [0u32; 256];
    for slot in fanout.iter_mut().skip(0x11) {
        *slot = 2;
    }
    for count in fanout {
        unsorted.extend_from_slice(&count.to_be_bytes());
    }
    unsorted.extend_from_slice(synthetic_oid(0x11, 2).as_bytes());
    unsorted.extend_from_slice(synthetic_oid(0x11, 1).as_bytes());
    unsorted.extend_from_slice(&1u32.to_be_bytes());
    unsorted.extend_from_slice(&2u32.to_be_bytes());
    unsorted.extend_from_slice(&12u32.to_be_bytes());
    unsorted.extend_from_slice(&24u32.to_be_bytes());
    unsorted.extend_from_slice(&[0xcd; 20]);
    append_checksum(&mut unsorted, ObjectFormat::Sha1);
    assert_eq!(
        PackIndex::decode(&unsorted, ObjectFormat::Sha1).unwrap_err(),
        PackError::UnsortedNames
    );
}
