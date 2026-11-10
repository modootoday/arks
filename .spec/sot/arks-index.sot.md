---
id: sot:root/arks-index
title: arks-index unit
status: active
created: 2026-11-08T10:05:00Z
updated: 2026-11-08T10:05:00Z
tags: [m3, unit, index, ring-2]
verify: manual
rel:
  governs: [crates/arks-index]
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090648
    - sot:root/git-compat-surface
    - sot:root/core-object-model
    - sot:root/lease-concurrency-intent
    - sot:root/crate-architecture
    - sot:root/arks-oid
    - sot:root/arks-hash
    - research:root/20261108071915
  evidence: [evidence:root/20261108191033]
---

# arks-index unit

Version 0.1.1. Unit SoT for Ring 2 Git index file read/write ([[plan:root/20261108090648]], M3 Phase R under [[sot:root/program-roadmap]] **v0.3.1**). Formats from [gitformat-index](https://git-scm.com/docs/gitformat-index) via [[research:root/20261108071915]]. Path: `$GIT_DIR/index` per [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout).

**Pre-restart caution:** host-only or pre-restart M3 index trees are **not** product-verified. M3 claims **index r/w** only — **not** full **G3** index-commit round-trip ([[sot:root/interop-test-matrix]]; full G3 waits for M5).

## Rule

1. **Public surface** MUST remain: `Index` (`read`, `read_path`, `write`, `write_path`), `IndexEntry`, `IndexExtension`, `IndexError`, `sort_entries`, `cmp_entries`, `entries_sorted`, and the documented entry flag constants (`CE_EXTENDED`, `CE_STAGEMASK`, `CE_VALID`).
2. **Claimed versions:**
   - **Read:** index versions **2, 3, and 4** (header `DIRC` + version + entry count; trailing hash checksum over content before the checksum; OID width follows repository object format via [[sot:root/arks-oid]] / [[sot:root/arks-hash]]).
   - **Write:** produce **version 2** by default; emit **version 3** when extended entry flags / long-name needs documented by gitformat-index require it. **Do not claim v4 write** in M3 (v4 path compression is read-only).
3. **Entries** MUST be ordered by path as unsigned bytes (`memcmp`) then stage, per gitformat-index. Writes MUST refuse unsorted entry lists.
4. **Extensions:** optional extensions whose signature starts with `A`–`Z` MAY be ignored when unknown on read (gitformat-index). M3 MUST **refuse** split-index (`LINK`) and sparse-directory (`SDIR`) rather than silently expand or corrupt. Safe optional extensions MAY pass through on rewrite when the crate marks them passthrough-safe; do not invent new extension signatures.
5. **L0 locks:** `write_path` MUST use exclusive `index.lock` (`create_new`), write payload, `sync_all`, rename over `index` — same short-lived exclusive pattern documented for Git index updates; cite layout + update-ref/index lock practice via [[research:root/20261108071915]] / [[sot:root/lease-concurrency-intent]] **L0**. **No L1+ lease API.**
6. **Out of scope (M3):** full **G3** (index + commit/tree porcelain round-trip — **M5**); sparse-index write semantics; split-index; cache-tree correctness product claims; racy-git policy invention beyond documented fields; pack/protocol/CLI; remotes; CELIR; `gix`.

## Why

The index is the working-tree cache shared with stock Git. Isolating claimed version codecs and L0 `index.lock` in `arks-index` lets M3 prove byte-level r/w parity without pulling Wave L commit porcelain (full G3) into Phase R.

## Verification

1. Host (advisory): `CARGO_BUILD_JOBS=2 cargo test -p arks-index`.
2. Clean-room **index r/w** rows in `verify/matrices/matrix-m3.md` (portion toward G3; not full G3) vs in-image stock Git; carrying receipt: [[evidence:root/20261108191033]]. Host green alone is not product-verified ([[sot:root/product-verification-clean-room]]).
3. `spec check` clean for this SoT.
4. Workspace `cargo metadata` shows no `gix` edge on `arks-index`.
