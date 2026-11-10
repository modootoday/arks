---
id: sot:root/arks-pack
title: arks-pack unit
status: active
created: 2026-11-08T10:12:00Z
updated: 2026-11-08T10:12:00Z
tags: [m4, unit, pack, idx, ring-2, phase-p]
verify: manual
rel:
  governs: [crates/arks-pack]
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090649
    - sot:root/git-compat-surface
    - sot:root/core-object-model
    - sot:root/crate-architecture
    - sot:root/arks-hash
    - sot:root/arks-oid
    - sot:root/arks-object
    - research:root/20261108071913
    - adr:root/20261108083507
    - evidence:root/20261108191033
  evidence: [evidence:root/20261108103610]
---

# arks-pack unit

Version 0.1.0. Unit SoT for Ring 2 pack + idx codec ([[plan:root/20261108090649]], M4 Phase P under [[sot:root/program-roadmap]] **v0.3.1**). Formats from [gitformat-pack](https://git-scm.com/docs/gitformat-pack) via [[research:root/20261108071913]]. Paths: `$GIT_DIR/objects/pack/pack-*.{pack,idx}` per [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout).

**Entry:** M3 exit proven — [[evidence:root/20261108191033]].

## Rule

1. **Public surface (M4 claimed):** pack decode + idx decode for the subset below; phased undeltified pack/idx **encode** sufficient for clean-room **G4** interchange with stock Git. Prefer modular `pack` / `idx` / `delta` / `error` layout ([[sot:root/structural-conventions]]).
2. **Claimed decode subset (MUST):**
   - Pack **version 2** (`PACK` + version + object count + entries + trailer checksum).
   - Object types: `OBJ_COMMIT` (1), `OBJ_TREE` (2), `OBJ_BLOB` (3), `OBJ_TAG` (4), `OBJ_OFS_DELTA` (6), `OBJ_REF_DELTA` (7). Refuse type 0 and reserved type 5.
   - zlib-compressed undeltified payloads; OFS/REF delta apply per gitformat-pack (base + size encodings; copy/insert instructions).
   - Idx **version 2** (`\377tOc` magic, fanout, names, CRCs, 4-byte offsets, optional 8-byte large-offset table, trailers).
   - Object format **SHA-1** for M4 G4 fixtures (OID width via [[sot:root/arks-oid]] / [[sot:root/arks-hash]]). SHA-256 pack/idx API MAY exist but MUST NOT be claimed verified in M4.
   - Reconstructed object OID MUST match `type SP size NUL` + payload hashing ([[sot:root/arks-object]] / gitformat-loose).
3. **Claimed encode subset (phased, for G4):**
   - Write **pack version 2** containing **undeltified** objects only (no OFS/REF delta emission required in M4).
   - Write matching **idx version 2** (CRC32 of each packed object entry; offsets; trailers).
   - Pack trailer and idx trailers MUST use the repository object-format hash (SHA-1 for claimed fixtures).
4. **MUST NOT invent** pack/idx bytes, type codes, delta instructions, or trailer rules. Primary only: gitformat-pack (+ chunk docs only if a later claim adds MIDX — not M4).
5. **No `gix`** and no product-path `git` subprocess ([[adr:root/20261108083507]]). Stock Git is clean-room oracle only.
6. **Out of scope (M4):** MIDX write/read product claims; `.rev` / `.mtimes` product claims; pack version 3; thin-pack as standalone product claim; pack-objects heuristics / delta window tuning; CLI porcelain; remotes; CELIR; leases.

## Why

Pack/idx interchange is the bridge from loose-object M1–M3 stores to protocol transfer and later remotes. Isolating a standards-cited decode (+ minimal undeltified encode) in `arks-pack` unlocks **G4** without pulling Wave L porcelain or M6 remotes into Phase P.

## Verification

1. Host (advisory): `CARGO_BUILD_JOBS=2 cargo test -p arks-pack`.
2. Clean-room **G4** `pack-local-roundtrip` (`verify/matrix-m4.md`, `verify/g4`) vs in-image stock Git — authoritative; carrying receipt: [[evidence:root/20261108103610]]. Host green alone is not product-verified ([[sot:root/product-verification-clean-room]], [[sot:root/interop-test-matrix]]).
3. `spec check` clean for this SoT.
4. Workspace `cargo metadata` shows no `gix` / gitoxide edge on `arks-pack`.
