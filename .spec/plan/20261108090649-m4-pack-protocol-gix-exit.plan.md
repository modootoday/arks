---
id: plan:root/20261108090649
title: M4 Pack protocol and gix-exit
status: active
created: 2026-11-08T09:06:49Z
updated: 2026-11-08T10:15:00Z
tags: [milestone, m4, phase-p, pack, protocol, gix-exit]
rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108090648
    - plan:root/20261108085816
    - adr:root/20261108083507
    - sot:root/git-compat-surface
    - sot:root/crate-architecture
    - sot:root/interop-test-matrix
    - sot:root/arks-pack
    - sot:root/arks-protocol
    - evidence:root/20261108191033
  implements: [sot:root/program-roadmap]
---

# M4 — Pack + protocol client libs (Phase P) + gix-exit

## Goal

Deliver pack/idx codec and pkt-line / protocol helpers as libraries (no porcelain required), and complete **gix-exit** so the workspace has zero `gix`/gitoxide Cargo edges.

## In scope / crates

| Artifact | Role |
|----------|------|
| `crates/arks-pack` | Pack + idx decode (encode phased); claimed subset — [[sot:root/arks-pack]] |
| `crates/arks-protocol` | pkt-line; capability advertise/parse helpers — [[sot:root/arks-protocol]] |
| gix-exit checklist | [[plan:root/20261108085816]] — prefer never introduced |

## Claimed subsets (design lock)

| Area | Claim | Encode phasing |
|------|-------|----------------|
| Pack | v2 decode; types 1–4 + OFS/REF delta; SHA-1 fixtures | Undeltified pack v2 write for **G4** |
| Idx | v2 decode (+ large-offset table) | Matching idx v2 write for **G4** |
| Protocol | pkt-line + flush/delim/response-end; capability parse/advertise | Offline **G5** fixtures; no live remotes |
| Negotiate | Prefer negotiate readiness (parse both v1 trailer + v2 special pkts); no invented caps | Full upload-pack client → M6 |

## Out of scope

- Permanent optional/`dev-dependencies` `gix` as end state (**rejected**).
- Remote porcelain / live GitHub (M6).
- CLI Wave L foundation (M5).
- MIDX / `.rev` product claims; pack v3; thin-pack product claims.

## Depends on

**M3** exit — [[evidence:root/20261108191033]] (tree `6dfffb387c211a26a9072d117ced251cd93da917`).

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G4** pack-local-roundtrip | Authoritative — `verify/matrix-m4.md` / `verify/g4` | Host |
| **G5** protocol-offline | Offline fixtures — `verify/g5` | Host |
| **gix-exit** | `cargo metadata` / workspace graph shows zero gix edges | CI/docs checklist |
| G1/G2/G10 regression | Embedded in M4 clean-room entry | Host |

## Ordered work items

1. **Design** — Claimed pack/protocol subsets locked in unit SoTs (this revision).
2. **Scaffold** — pack + protocol crates; ensure no `gix` features land by default.
3. **Implement** — pack decode → undeltified encode → protocol fixtures; remove any transitional cold `gix` if it ever appeared.
4. **Verify** — G4/G5; gix-exit checklist complete before claiming M4 exit.

## Open questions

- Pack delta **encode** (beyond undeltified) remains post-M4.
- Whether M5 exposes pack helper bins or stays library-only until M6 ([[plan:root/20261108090650]]).

## Cross-links

[[sot:root/program-roadmap]], [[adr:root/20261108083507]], [[plan:root/20261108085816]], [[sot:root/crate-architecture]], [[sot:root/arks-pack]], [[sot:root/arks-protocol]]
