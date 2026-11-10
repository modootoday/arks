---
id: plan:root/20261108090647
title: M2 Git-first storage facade
status: active
created: 2026-11-08T09:06:47Z
updated: 2026-11-08T10:00:33Z
tags: [milestone, m2, phase-s, store, restart]
rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090646
    - sot:root/storage-celir-boundary
    - sot:root/crate-architecture
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - evidence:root/20261108185319
  implements: [sot:root/program-roadmap]
  evidence: [evidence:root/20261108190033]
---

# M2 — Git-first storage facade (Phase S)

## Goal

Route all **object** I/O through a store facade with a filesystem Git backend; keep the CELIR slot empty / not-ready.

**Roadmap v0.3.1 restart:** After M1 re-prove ([[evidence:root/20261108185319]]), M2 did **not** auto-carry prior verified status. Prior M2 tree and [[evidence:root/20261108093059]] were **advisory / gap-fill inputs** only. Gap-fill + **re-verify** G10 objects-subset + G1 regression completed under [[evidence:root/20261108190033]].

## In scope / crates

| Artifact | Role |
|----------|------|
| `crates/arks-store` | Object-store facade trait + filesystem Git backend |
| CELIR slot | Compile-gated or explicitly unimplemented stub only |

## Out of scope

- Activating CELIR or inventing CELIR APIs.
- Refs/index/repo product claims (M3).
- Claiming full G1–G4-through-facade before those gates exist.

## Depends on

**M1** exit re-proven under v0.3.1 — [[evidence:root/20261108185319]] ([[plan:root/20261108090646]]).

## Exit gates (re-verify)

Under roadmap **v0.3.1**, M2 exit is carried by the **new** authoritative receipt after clean-room re-run (`verify/images.lock.json`, `verify/matrix-m2.md`).

| Gate | Clean-room (authoritative) | Advisory |
|------|---------------------------|----------|
| **G10** objects-subset | **PASS** — [[evidence:root/20261108190033]] | Host facade unit tests |
| **G1** regression | **PASS** — [[evidence:root/20261108190033]] (via `run-g10.sh`) | Host |

**Prior (non-carrying alone):** [[evidence:root/20261108093059]] — historical gap-fill input only under v0.3.1; **not** M2 exit by itself.

## Ordered work items

1. **Design** — Facade trait surface vs [[sot:root/storage-celir-boundary]] Phase A (Git-only). **Done** — [[sot:root/arks-store]].
2. **Scaffold** — `arks-store` crate behind workspace membership. **Done**.
3. **Implement** — FS backend put/get/exists; wire M1 object types through facade. **Done** — `FilesystemGitStore` + `MemoryObjectStore`; CELIR feature stub; zero `gix`.
4. **Verify** — G10 objects-subset + G1 regression (clean-room). **Done** — [[evidence:root/20261108190033]] (restart under v0.3.1; historical [[evidence:root/20261108093059]] non-carrying alone).

## Open questions

- Exact trait method set — **closed for M2:** `put` / `get` / `exists` only (iterate deferred).
- When to run full G1–G4-through-facade (after M4/M5 — see interop matrix) — still open; not an M2 blocker.

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/storage-celir-boundary]], [[plan:root/20261108090653]], [[sot:root/interop-test-matrix]], [[sot:root/arks-store]], [[evidence:root/20261108185319]], [[evidence:root/20261108190033]], [[evidence:root/20261108093059]] (historical / non-carrying alone)

## Explicit

Under roadmap v0.3.1, M2 exit is carried by [[evidence:root/20261108190033]]. Historical [[evidence:root/20261108093059]] does **not** satisfy M2 exit alone.
