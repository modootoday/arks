---
id: sot:root/arks-hash
title: arks-hash unit
status: active
created: 2026-11-08T09:18:01Z
updated: 2026-11-08T18:52:00Z
tags: [engine, hash, m1, ring-0]
verify: manual
rel:
  governs: [crates/arks-hash]
  depends_on: [sot:root/core-object-model, sot:root/crate-architecture, sot:root/git-compat-surface, sot:root/product-verification-clean-room, sot:root/program-roadmap, plan:root/20261108090646]
  evidence: [research:root/20261108071913, evidence:root/20261108185319]
---

# arks-hash unit

Version 0.1.1. Normative scope for the `arks-hash` crate (Ring 0 identity hashing). Object identity semantics are owned by [[sot:root/core-object-model]]; crate placement by [[sot:root/crate-architecture]]. Primary format cite: [gitformat-loose](https://git-scm.com/docs/gitformat-loose) (summarized in [[research:root/20261108071913]]). Under [[sot:root/program-roadmap]] **v0.3.1**, M1 is **restart / re-verify** ([[plan:root/20261108090646]]); host greens and prior receipts are not product exit without a new clean-room run ([[sot:root/product-verification-clean-room]]).

## Rule

1. **In-engine only.** The crate computes digests of uncompressed Git object bytes (`type SP size NUL` + payload) and arbitrary byte buffers. It MUST NOT perform filesystem or network I/O and MUST NOT depend on `gix`/gitoxide or spawn `git`.
2. **Algorithms.** `ObjectFormat::Sha1` (20-byte digest) and `ObjectFormat::Sha256` (32-byte digest) MUST be selectable via the public API (`Hasher`, `hash_bytes`, `hash_object`, `Digest`).
3. **M1 claimed interop subset.** SHA-1 `hash_object` results for the fixture types in unit tests (at minimum `blob` with known vectors) MUST match stock Git’s uncompressed-object hash for the same bytes. This is the **only** algorithm/format pair product-interop claims for M1 G1 fixtures until extended by SoT revision.
4. **SHA-256 API, not G1-claimed.** SHA-256 helpers are present for future object-format transition work ([[sot:root/git-compat-surface]]); M1 does **not** claim clean-room G1 parity for SHA-256 until vectors and gates say so.
5. **Stable surface.** Public types and functions listed in the crate root (`ObjectFormat`, `Digest`, `Hasher`, `hash_bytes`, `hash_object`, `HashError`) are the integration boundary; behavior changes require SoT/plan revision.
6. **No invented identity.** Alternate hash inputs (e.g. zlib-compressed bytes, truncated headers) are forbidden in Git-compat mode per [[sot:root/core-object-model]].

## Why

Bottom-up M1 needs a single, testable place to compute OIDs before `arks-oid` and `arks-object` wire loose encoding. Keeping hashing pure (no I/O, no subprocess, no `gix`) preserves engine ownership ([[sot:root/crate-architecture]], ADR native engine) and lets host unit tests iterate quickly while clean-room remains authoritative for product claims ([[sot:root/product-verification-clean-room]]).

## Verification

1. **Host (advisory).** `cargo test -p arks-hash` and `cargo clippy -p arks-hash -- -D warnings` on the developer host — fast feedback only; not product-verified.
2. **Unit vectors.** Tests MUST include at least: empty `blob` SHA-1 OID matching stock Git; `hello` blob SHA-1; incremental `Hasher` vs one-shot `hash_bytes` equivalence; SHA-256 empty `blob` digest length 32 and known OID; `Digest::from_bytes` length refusal; payload containing `NUL` bytes hashed as payload; moderately large payload stress.
3. **Clean-room (authoritative for product).** When G0/G1 harness exists, in-image stock `git hash-object` vs `arks` for the claimed SHA-1 subset is authoritative per [[sot:root/product-verification-clean-room]]; host-only green does not satisfy verified.
4. **Negative.** Wrong-length digest imports return `HashError::DigestLength`; no silent truncation.

### Open decisions

| Decision | Status | Notes |
|----------|--------|-------|
| SHA-256 G1 vectors | Open | Add when object-format transition milestone claims interop |
| Non-blob type vectors in crate | Open | May expand with `arks-object` fixtures; blob suffices for M1 unit scope |
