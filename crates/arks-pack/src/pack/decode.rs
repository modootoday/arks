//! Pack version 2 decoder.
//!
//! Primary: <https://git-scm.com/docs/gitformat-pack>

use super::entry::{
    OBJ_OFS_DELTA, OBJ_REF_DELTA, inflate_with, kind_of_type, read_ofs_offset, read_type_and_size,
    validate_type,
};
use super::header::{self, HEADER_LEN};
use crate::checksum::{self, checksum_matches};
use crate::delta::apply_delta;
use crate::error::PackError;
use arks_hash::ObjectFormat;
use arks_object::RawObject;
use arks_oid::Oid;
use flate2::Decompress;
use std::collections::HashMap;

/// One pack entry resolved to a canonical object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedObject {
    pub oid: Oid,
    pub object: RawObject,
    /// Offset of this entry's type byte from the start of the pack.
    pub offset: u64,
    /// zlib CRC32 of the packed entry (header, base name or offset, compressed bytes).
    pub crc32: u32,
}

/// Pack version 2 with every entry resolved to a canonical object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPack {
    pub format: ObjectFormat,
    pub objects: Vec<ResolvedObject>,
    /// Trailer digest (hash of all preceding pack bytes).
    pub checksum: Vec<u8>,
}

impl DecodedPack {
    pub fn decode(bytes: &[u8], format: ObjectFormat) -> Result<Self, PackError> {
        let (_version, count) = header::parse_header(bytes)?;
        let hash_len = format.digest_len();
        if bytes.len() < HEADER_LEN + hash_len {
            return Err(PackError::Truncated);
        }
        let body_limit = bytes.len() - hash_len;
        if usize::try_from(count).unwrap_or(usize::MAX) > body_limit.saturating_sub(HEADER_LEN) {
            return Err(PackError::Truncated);
        }

        let mut pos = HEADER_LEN;
        let mut slots = Vec::with_capacity(count as usize);
        let mut meta = Vec::with_capacity(count as usize);
        let mut offset_index: HashMap<u64, usize> = HashMap::with_capacity(count as usize);
        // One zlib state for the whole pack — J-CLONE-LOCAL inflate wall.
        let mut inflator = Decompress::new(true);

        for _ in 0..count {
            if pos >= body_limit {
                return Err(PackError::Truncated);
            }
            let entry_offset = pos as u64;
            let (type_id, declared, hdr_len) = read_type_and_size(&bytes[pos..body_limit])?;
            validate_type(type_id)?;
            pos += hdr_len;
            let slot = read_body(
                bytes,
                &mut pos,
                body_limit,
                format,
                type_id,
                declared,
                entry_offset,
                &offset_index,
                &mut inflator,
            )?;
            let crc32 = crc32fast::hash(&bytes[entry_offset as usize..pos]);
            offset_index.insert(entry_offset, slots.len());
            meta.push(EntryMeta {
                offset: entry_offset,
                crc32,
            });
            slots.push(slot);
        }

        if pos != body_limit {
            return Err(PackError::Trailing);
        }
        if !checksum_matches(&bytes[..pos], &bytes[pos..], format) {
            return Err(PackError::PackChecksum);
        }

        let objects = resolve_slots(slots, &meta, format)?;
        Ok(Self {
            format,
            objects,
            checksum: bytes[pos..].to_vec(),
        })
    }

    pub fn find(&self, oid: &Oid) -> Option<&ResolvedObject> {
        self.objects.iter().find(|obj| obj.oid == *oid)
    }
}

struct EntryMeta {
    offset: u64,
    crc32: u32,
}

enum Slot {
    Ready(RawObject),
    /// `base_idx` is the earlier pack slot (ofs-delta bases always precede).
    Ofs { base_idx: usize, delta: Vec<u8> },
    Ref { base: Oid, delta: Vec<u8> },
}

#[allow(clippy::too_many_arguments)]
fn read_body(
    bytes: &[u8],
    pos: &mut usize,
    body_limit: usize,
    format: ObjectFormat,
    type_id: u8,
    declared: u64,
    entry_offset: u64,
    offset_index: &HashMap<u64, usize>,
    inflator: &mut Decompress,
) -> Result<Slot, PackError> {
    if type_id == OBJ_OFS_DELTA {
        if *pos >= body_limit {
            return Err(PackError::Truncated);
        }
        let (rel, n) = read_ofs_offset(&bytes[*pos..body_limit])?;
        *pos += n;
        if rel == 0 || rel > entry_offset {
            return Err(PackError::BadOfsOffset);
        }
        let base_offset = entry_offset - rel;
        let base_idx = offset_index
            .get(&base_offset)
            .copied()
            .ok_or(PackError::BadOfsOffset)?;
        let delta = inflate_declared(bytes, pos, body_limit, declared, inflator)?;
        return Ok(Slot::Ofs { base_idx, delta });
    }
    if type_id == OBJ_REF_DELTA {
        let hash_len = format.digest_len();
        let end = pos.checked_add(hash_len).ok_or(PackError::Truncated)?;
        if end > body_limit {
            return Err(PackError::Truncated);
        }
        let base = Oid::from_bytes(checksum::oid_kind(format), &bytes[*pos..end])?;
        *pos = end;
        let delta = inflate_declared(bytes, pos, body_limit, declared, inflator)?;
        return Ok(Slot::Ref { base, delta });
    }
    let kind = kind_of_type(type_id)?;
    let payload = inflate_declared(bytes, pos, body_limit, declared, inflator)?;
    Ok(Slot::Ready(RawObject::new(kind, payload)))
}

fn inflate_declared(
    bytes: &[u8],
    pos: &mut usize,
    body_limit: usize,
    declared: u64,
    inflator: &mut Decompress,
) -> Result<Vec<u8>, PackError> {
    if *pos > body_limit {
        return Err(PackError::Truncated);
    }
    let expected = usize::try_from(declared).map_err(|_| PackError::SizeOverflow)?;
    let (data, consumed) = inflate_with(inflator, &bytes[*pos..body_limit], expected)?;
    *pos += consumed;
    Ok(data)
}

fn resolve_slots(
    mut slots: Vec<Slot>,
    meta: &[EntryMeta],
    format: ObjectFormat,
) -> Result<Vec<ResolvedObject>, PackError> {
    let n = slots.len();
    let mut oids: Vec<Option<Oid>> = vec![None; n];
    let mut by_oid: HashMap<Oid, usize> = HashMap::with_capacity(n);

    for (i, slot) in slots.iter().enumerate() {
        if let Slot::Ready(obj) = slot {
            let oid = obj.oid(format)?;
            record_oid(&mut oids, &mut by_oid, i, oid);
        }
    }

    // Ofs-delta bases always precede in pack order → one forward pass.
    for i in 0..n {
        if oids[i].is_some() {
            continue;
        }
        let Slot::Ofs { base_idx, .. } = &slots[i] else {
            continue;
        };
        let base_idx = *base_idx;
        if oids[base_idx].is_none() {
            return Err(PackError::DeltaBaseMissing);
        }
        let delta = take_delta(&mut slots[i])?;
        let (kind, base_payload) = match &slots[base_idx] {
            Slot::Ready(obj) => (obj.kind, obj.payload.as_slice()),
            _ => {
                restore_delta(&mut slots[i], delta);
                return Err(PackError::DeltaBaseMissing);
            }
        };
        let payload = apply_delta(base_payload, &delta)?;
        let obj = RawObject::new(kind, payload);
        let oid = obj.oid(format)?;
        slots[i] = Slot::Ready(obj);
        record_oid(&mut oids, &mut by_oid, i, oid);
    }

    // Ref-delta is uncommon on non-thin packs; keep a short multi-pass.
    for _ in 0..n {
        let mut progress = false;
        for i in 0..n {
            if oids[i].is_some() {
                continue;
            }
            let Slot::Ref { base, .. } = &slots[i] else {
                continue;
            };
            let Some(&base_idx) = by_oid.get(base) else {
                continue;
            };
            if oids[base_idx].is_none() {
                continue;
            }
            let delta = take_delta(&mut slots[i])?;
            let (kind, base_payload) = match &slots[base_idx] {
                Slot::Ready(obj) => (obj.kind, obj.payload.as_slice()),
                _ => {
                    restore_delta(&mut slots[i], delta);
                    continue;
                }
            };
            let payload = apply_delta(base_payload, &delta)?;
            let obj = RawObject::new(kind, payload);
            let oid = obj.oid(format)?;
            slots[i] = Slot::Ready(obj);
            record_oid(&mut oids, &mut by_oid, i, oid);
            progress = true;
        }
        if !progress {
            break;
        }
    }

    if oids.iter().any(Option::is_none) {
        return Err(classify_unresolved(&slots, &oids, &by_oid));
    }

    let mut objects = Vec::with_capacity(n);
    for (i, slot) in slots.into_iter().enumerate() {
        let Slot::Ready(object) = slot else {
            return Err(PackError::DeltaBaseMissing);
        };
        let oid = oids[i].ok_or(PackError::DeltaBaseMissing)?;
        objects.push(ResolvedObject {
            oid,
            object,
            offset: meta[i].offset,
            crc32: meta[i].crc32,
        });
    }
    Ok(objects)
}

fn record_oid(oids: &mut [Option<Oid>], by_oid: &mut HashMap<Oid, usize>, index: usize, oid: Oid) {
    oids[index] = Some(oid);
    by_oid.entry(oid).or_insert(index);
}

fn take_delta(slot: &mut Slot) -> Result<Vec<u8>, PackError> {
    match slot {
        Slot::Ofs { delta, .. } | Slot::Ref { delta, .. } => Ok(std::mem::take(delta)),
        Slot::Ready(_) => Err(PackError::BadDelta("resolver stalled")),
    }
}

fn restore_delta(slot: &mut Slot, bytes: Vec<u8>) {
    match slot {
        Slot::Ofs { delta, .. } | Slot::Ref { delta, .. } => *delta = bytes,
        Slot::Ready(_) => {}
    }
}

fn classify_unresolved(
    slots: &[Slot],
    oids: &[Option<Oid>],
    by_oid: &HashMap<Oid, usize>,
) -> PackError {
    let mut internal = false;
    let mut external = false;
    for (i, slot) in slots.iter().enumerate() {
        if oids[i].is_some() {
            continue;
        }
        match slot {
            Slot::Ofs { .. } => internal = true,
            Slot::Ref { base, .. } => {
                if by_oid.contains_key(base) {
                    internal = true;
                } else {
                    external = true;
                }
            }
            Slot::Ready(_) => external = true,
        }
    }
    if internal && !external {
        PackError::DeltaCycle
    } else {
        PackError::DeltaBaseMissing
    }
}
