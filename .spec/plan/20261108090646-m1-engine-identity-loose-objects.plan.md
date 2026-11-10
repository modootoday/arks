---
id: plan:root/20261108090646
title: M1 Engine identity and loose objects
status: active
created: 2026-11-08T09:06:46Z
updated: 2026-11-08T18:53:19Z
tags: [milestone, m1, phase-c, engine, wave-1, restart]
rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090645
    - plan:root/20261108085816
    - plan:root/20261108082110
    - sot:root/core-object-model
    - sot:root/git-compat-surface
    - sot:root/crate-architecture
    - sot:root/product-verification-clean-room
    - adr:root/20261108083507
  implements: [sot:root/program-roadmap]
  evidence: [evidence:root/20261108185319]
---

# M1 — Engine identity + loose objects (Phase C)

## Goal

Ship the smallest engine slice: in-house hashing, OID types, and loose object encode/decode interchangeable with stock Git for the claimed object subset.

**Roadmap v0.3.1 restart:** Program implementation **restarts at M1**. Prior M1 (and later) trees on disk are **inputs for gap-fill / re-verify**, not automatic carry-forward of verified status under the raised CLI-parity production bar ([[sot:root/program-roadmap]]). Re-run G0/G1 (and unit Specfold) against current canon; **Main** records a **new** clean-room evidence receipt before M1 exit is claimed again.

## In scope / crates

| Artifact | Role |
|----------|------|
| Root virtual Cargo workspace | Scaffolding under `crates/` |
| `crates/arks-hash` | Object-format hashing (SHA-1 / SHA-256 as claimed); no I/O |
| `crates/arks-oid` | OID newtype, hex parse/format/compare |
| `crates/arks-object` | Loose object header + payload + zlib for claimed types |
| Fixture intent docs | Bytes + expected OID; cite git-compat (no invented semantics) |
| Clean-room / verify stub | Name G0/G1 intent; **do not invent** image digests |

## Out of scope

- CLI porcelain; remotes; lease runtime; CELIR; pack writers as product claims.
- Introducing `gix`/gitoxide (prefer never; not an M1 target).
- Claiming product verified from host-only tests.

## Depends on

**M0** exit ([[plan:root/20261108090645]]).

## Exit gates (re-verify)

Under roadmap **v0.3.1**, M1 exit requires a **new** authoritative receipt after clean-room re-run (`verify/images.lock.json`, `verify/matrix-m1.md`).

| Gate | Clean-room (authoritative) | Advisory |
|------|---------------------------|----------|
| **G0** unit-core | **PASS** — [[evidence:root/20261108185319]] | Host `cargo test` |
| **G1** objects-roundtrip | **PASS** — [[evidence:root/20261108185319]] | Host parity experiments |

**Prior (non-carrying alone):** [[evidence:root/20261108092148]] — historical gap-fill input only under v0.3.1.

## Ordered work items

1. **Design** — Confirm claimed object subset + hash algorithms against [[sot:root/git-compat-surface]] / research (no invention).
2. **Design** — Wave-1 checklist locked ([[plan:root/20261108085816]]); **operator confirmation recorded 2026-11-08** (implementation armed).
3. **Scaffold** — Workspace + crate stubs (only after confirmation).
4. **Implement** — `arks-hash` → `arks-oid` → fixtures → `arks-object` (Wave-1 order W1.1–W1.6).
5. **Verify** — Host advisory tests labeled; clean-room G0/G1 when image lock exists; update `rel.governs` when dirs exist.

## Open questions

- Clean-room base image / Git pin / compose layout (clean-room SoT Opens).
- Exact SHA-256 object-format claim timing (cite primary docs before claiming).

## Cross-links

[[sot:root/program-roadmap]], [[plan:root/20261108085816]], [[plan:root/20261108082110]], [[adr:root/20261108083507]], [[sot:root/core-object-model]]

## What M1 must prove (under CLI-parity roadmap)

1. In-engine hash + OID + loose object encode/decode for the claimed subset (no product-path `git` subprocess; **zero `gix`**).
2. Unit Specfold SoTs for hash/oid/object crates still accurate.
3. Clean-room **G0** + **G1** green with stock Git peer oracle when harness exists (host tests advisory only).
4. Does **not** prove CLI porcelain, remotes, or CLI-parity production (M9).

## Explicit

Wave-1 Rust implementation was authorized historically ([[plan:root/20261108085816]]). Under roadmap v0.3, treat M1 as **restart / re-verify** before M2+.
