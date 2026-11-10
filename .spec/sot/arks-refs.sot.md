---
id: sot:root/arks-refs
title: arks-refs unit
status: active
created: 2026-11-08T10:00:00Z
updated: 2026-11-08T10:05:00Z
tags: [m3, unit, refs, ring-3]
verify: manual
rel:
  governs: [crates/arks-refs]
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090648
    - sot:root/git-compat-surface
    - sot:root/core-object-model
    - sot:root/lease-concurrency-intent
    - sot:root/crate-architecture
    - sot:root/arks-oid
    - research:root/20261108071914
  evidence: [evidence:root/20261108191033]
---

# arks-refs unit

Version 0.1.2. Unit SoT for Ring 3 Git refs under a filesystem `GIT_DIR` ([[plan:root/20261108090648]], M3 Phase R under [[sot:root/program-roadmap]] **v0.3.1**).

**Pre-restart caution:** historical or host-only M3 trees are **not** product-verified; exit needs a clean-room receipt per [[sot:root/product-verification-clean-room]] and `verify/matrices/matrix-m3.md`.

## Rule

1. **Public surface** MUST remain: `is_valid_refname`, `is_valid_refname_allow_onelevel`, `RefTarget`, `RefsError`, `FilesystemRefStore::{open, read, resolve, update, delete, set_symbolic, list_refs}`.
2. **Refnames** MUST follow the claimed [[research:root/20261108071914]] / [git-check-ref-format](https://git-scm.com/docs/git-check-ref-format) subset: reject `..`, control/space/`~^:?*[\`, `@{`, trailing `/` or `.`, `.lock` suffix, components starting with `.`; require `/` unless `HEAD` or `--allow-onelevel` mode. No path traversal in stored paths (validated names only).
3. **Loose refs** under `refs/` and root `HEAD` per [gitrepository-layout](https://git-scm.com/docs/gitrepository-layout); direct refs are lowercase hex OID + newline on write; symrefs `ref: <target>\n`. Read accepts trimmed content without requiring trailing newline.
4. **`packed-refs` read-only** (M3 **decided**): per [git-pack-refs](https://git-scm.com/docs/git-pack-refs), skip `#` headers and peel lines `^{}`; map refname → OID. **Loose wins** over packed for the same name. **packed-refs write / rewrite:** out of scope for M3 (closes plan open question “packed-refs write policy” as read-only-first).
5. **OIDs** via [[sot:root/arks-oid]] (`from_hex` width inference: SHA-1 40 / SHA-256 64 hex).
6. **L0 locks** per [git-update-ref](https://git-scm.com/docs/git-update-ref): exclusive `ref.lock` (`create_new`), write payload, `sync_all`, rename into place; delete uses lock then remove ref and lock. **No L1+ lease API** ([[sot:root/lease-concurrency-intent]]).
7. **`read`** returns one hop; **`resolve`** follows symrefs with depth cap. **`update`/`delete`** optional `old` OID compare against current value from loose **or** packed (same as `read`). **`update`** always writes a **loose** ref (overrides packed). **`delete`** of a packed-only name MUST fail until packed-refs rewrite is claimed (M3: no packed-refs write).
8. **Out of scope:** reftable, reflog write, packed-refs rewrite, pack/protocol/CLI, remotes, CELIR, `gix`, product lease APIs.

## Why

Refs are the mutable pointer layer between repo layout and objects; isolating refname rules, loose/packed read, and L0 locks in one crate gives M3 **G2** a stable facade for `arks-repo` without inventing concurrency beyond documented Git file locks.

## Verification

1. Host (advisory): `CARGO_BUILD_JOBS=2 cargo test -p arks-refs`.
2. Clean-room **G2** refs-roundtrip via `verify/scripts/run-g2.sh` / compose service `g2` (`verify/matrices/matrix-m3.md`); carrying receipt: [[evidence:root/20261108191033]]. Host green alone is not product-verified ([[sot:root/product-verification-clean-room]]).
3. `spec check` clean for this SoT.
4. Workspace `cargo metadata` shows no `gix` edge on `arks-refs`.
