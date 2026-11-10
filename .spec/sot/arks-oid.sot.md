---
id: sot:root/arks-oid
title: arks-oid unit
status: active
created: 2026-11-08T09:18:17Z
updated: 2026-11-08T18:52:00Z
tags: [oid, identity, m1, ring-0]
verify: manual
rel:
  governs: [crates/arks-oid]
  depends_on: [sot:root/core-object-model, sot:root/crate-architecture, sot:root/git-compat-surface, sot:root/product-verification-clean-room, sot:root/program-roadmap, plan:root/20261108090646]
  evidence: [research:root/20261108071913, evidence:root/20261108185319]
---

# arks-oid unit

Version 0.1.1. Normative intent for **`crates/arks-oid`**: a Ring-0 newtype for Git **object IDs (OIDs)** — fixed-width digests for SHA-1 (20 bytes) and SHA-256 (32 bytes). This crate **does not** hash object bytes, read loose files, or parse object payloads; identity **computation** is [[sot:root/core-object-model]] / `arks-hash`. **Representation, validation, and hex interchange** are in scope. Under [[sot:root/program-roadmap]] **v0.3.1**, M1 is **restart / re-verify** ([[plan:root/20261108090646]]); oracle rows in `verify/fixtures/m001-clean-room-g0-g1/` support unit tests; product OID claims remain clean-room authoritative ([[sot:root/product-verification-clean-room]]).

Primary: [gitformat-loose](https://git-scm.com/docs/gitformat-loose) — object id is the hash of **uncompressed** bytes `type SP size NUL` + payload ([[research:root/20261108071913]]).

## Rule

### Scope (M1)

1. **Public surface** MUST remain: `OidKind`, `Oid`, `OidError`, and `Oid::{kind, as_bytes, from_bytes, from_hex, to_hex}` plus `Debug` / `Display`.
2. **No workspace dependencies** — especially MUST NOT depend on `arks-hash`, `gix`, or other arks engine crates (Ring 0 per [[sot:root/crate-architecture]]).
3. **`OidKind`** MUST distinguish SHA-1 (`digest_len` 20, hex 40) and SHA-256 (`digest_len` 32, hex 64) widths consistent with Git object format ([[sot:root/git-compat-surface]] hash-function transition surface).
4. **`from_bytes`** MUST reject slices whose length ≠ `kind.digest_len()` with `OidError::Length`.
5. **`from_hex`** MUST:
   - infer `OidKind` from hex **character** count: 40 → SHA-1, 64 → SHA-256 (`OidKind::digest_len` / `hex_len` expose widths);
   - reject other lengths with `OidError::HexLength`;
   - reject non-hexadecimal ASCII with `OidError::HexCharset`;
   - accept **case-insensitive** hex digits (`0-9`, `a-f`, `A-F`).
6. **`to_hex`** MUST emit **lowercase** a–f (stock Git default display style); round-trip with `from_hex` MUST preserve digest bytes for valid input.
7. **`Display` / `Debug`** MUST format via lowercase hex (`Debug` as `Oid(<hex>)`, `Display` as bare hex) — no path or refname semantics.
8. OID values are **digests**, not loose-object paths (`objects/aa/bb…`); path layout is out of scope (store crates).

### Known vectors (M1 advisory / unit tests)

These SHA-1 OIDs MUST match stock Git for the cited uncompressed headers + payloads (see `verify/fixtures/m001-clean-room-g0-g1/` intent docs):

| Case | Uncompressed header + payload | SHA-1 OID (hex) |
|------|------------------------------|-----------------|
| Empty blob | `blob 0\0` (empty payload) | `e69de29bb2d1d6434b8b29ae775ad8c2e48c5391` |
| Blob `hello` | `blob 5\0hello` | `b6fc4c620b67d95f953a5c1c1230aaab5db5a1b0` |

### Out of scope (this SoT / crate)

- Computing hashes from object content (`arks-hash`).
- Zlib, loose/pack I/O, ref names, or comparing OIDs to filesystem paths.
- SHA-256 fixture oracle rows until `arks-hash` + clean-room rows are claimed.
- Tree/commit/tag **payload** grammar — cite plumbing docs at `arks-object` time only.

## Why

Splitting **digest representation** from **hashing** keeps Ring 0 dependency-free so `arks-oid` can be used anywhere identifiers appear without pulling hash algorithms or object parsers. Strict hex/length validation catches bad input early and matches how Git names objects on the wire and in logs. Lowercase `to_hex` matches common Git output while permissive parse avoids brittle callers.

## Verification

1. Host (advisory): `cargo test -p arks-oid` and `cargo clippy -p arks-oid -- -D warnings`.
2. Unit tests MUST cover: known blob OIDs (empty, `hello`), hex length/charset refusal, byte-length refusal, case-insensitive hex parse, `Display`/`Debug` formatting.
3. `spec check` clean for this document and links to primary URLs.
4. Authoritative OID oracle vs stock Git: Docker clean-room per [[sot:root/product-verification-clean-room]] when harness exists; until then, fixture intent in `verify/fixtures/m001-clean-room-g0-g1/` cites primary docs only.
