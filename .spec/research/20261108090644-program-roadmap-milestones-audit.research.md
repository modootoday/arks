---
id: research:root/20261108090644
title: Program roadmap milestones audit
status: active
created: 2026-11-08T09:06:44Z
updated: 2026-11-08T09:06:44Z
tags: [roadmap, milestones, audit, production, contradictions]
rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108082148
    - plan:root/20261108082110
    - plan:root/20261108085816
    - adr:root/20261108083507
    - sot:root/interop-test-matrix
    - sot:root/crate-architecture
    - sot:root/storage-celir-boundary
    - sot:root/lease-concurrency-intent
    - sot:root/product-verification-clean-room
---

# Program roadmap milestones audit

**Superseded for production bar:** CLI-parity raise and M0–M10 restructure are audited in [[research:root/20261108183820]]. This document remains historical for the v0.2 **M6 Git-compat production** bar.

Audit of arks milestone / phase / gate / crate-order canon against a **production-level** delivery bar. Design only — no Rust implementation. Findings drove [[sot:root/program-roadmap]] v0.2 and per-milestone plans `plan:root/20261108090645`…`20261108090653`.

## Production bar (audit target)

| Axis | Production expectation |
|------|------------------------|
| Daily-driver Git-compat | Local porcelain (CLI Phase A) + GitHub-class remote client (Phase B / surface A) |
| Verification | Clean-room receipts for claimed Gate IDs; host greens advisory only |
| Engine | Native in-process Rust; **zero `gix`/gitoxide** end state (gix-exit ≤ M4) |
| Differentiator | Lease / N≈hundreds is **after** Git-compat production (M7 / Phase E), not a production blocker |
| CELIR | Side track; not required for Git-compat production |
| Public surface | English narrative honest; no overclaim; release obligations tracked |
| Server-role / allocator crate | **Open** — not invented; not production blockers |

## Holes and contradictions

| ID | Finding | Disposition |
|----|---------|-------------|
| **H1** | No explicit **production exit** definition — M0–M7 read as early engine + features without a “daily-driver done” line | **Fixed** in [[sot:root/program-roadmap]]: Production = M6 exit; Differentiator = M7 |
| **H2** | [[plan:root/20261108082110]] slice order put **refs before store**, contradicting phases C→**S→R** and milestones M2→M3 | **Fixed**: core slices reordered to store → refs → index/repo |
| **H3** | [[sot:root/interop-test-matrix]] G10 text said “once G1–G4 exist”, while M2 claims G10 after M1 only | **Fixed**: G10-M2 = facade-over-objects (G1 regression); full G1–G4-through-facade after those gates exist |
| **H4** | M3 listed G3 while G3 is index-**commit** round-trip (needs porcelain commit path) | **Fixed**: M3 = G2 + **index r/w portion**; full G3 at M5 |
| **H5** | Core slice placed CLI after remotes/CELIR; phases require **L before H** | **Fixed** in core-build-slice ordering notes |
| **H6** | Lease L1 appeared as early core slice 9; product lease exit is M7 | **Fixed**: L0 Git locks in M3; L1+ product work deferred to M7 plan |
| **H7** | Leftover “optional cold `gix` forever” risk vs ADR zero-`gix` end state | **Fixed** (sibling + this pass): ADR / SoTs / Wave plan **gix-exit before M4**; prefer never introduce |
| **H8** | Wave-1 plan doubled as whole-program sketch; per-milestone staged plans missing | **Fixed**: Wave-1 = M1 first wave only; plans M0–M7 + CELIR side track |
| **H9** | Clean-room image/compose/digest still Open — milestones claimed “verified” without harness | **Residual Open**: milestones may claim host-advisory until lock exists; verified only with receipts |
| **H10** | Server-role (arks as Git server) Open | **Residual Open** — not in production exit; no invented server milestone |
| **H11** | Allocator / arena helper crate placement Open | **Residual Open** — principles only; no crate invented |
| **H12** | Protocol v1-first vs v2-mandatory inside Phase P | **Residual Open** — prefer negotiate; order TBD inside M4 plan |
| **H13** | G8 live network policy Open | **Residual Open** — G7 recorded is production CI default when live creds absent |
| **H14** | CELIR readiness not flipped (v0.4 catalog ≠ ready) | **Residual Open** / side track — never silent Git-compat replace |
| **H15** | High-concurrency product mode dual-path design Open | **Residual Open** — M7 intent only until ADR |

## Alignment check (after fixes)

| Layer | Order |
|-------|-------|
| Phases | C → S → R → P → L → H → E |
| Milestones | M0 → M1 → M2 → M3 → M4 → M5 → M6 (**Production**) → M7 (Differentiator) |
| Gates (primary) | M1:G0–G1; M2:G10⊂; M3:G2+index; M4:G4–G5+gix-exit; M5:G6+G3full; M6:G7(+G8/G9); M7:G11 |
| Crates (bottom-up) | hash/oid/object → store → refs/index/repo → pack/protocol → `arks` → remote → lease; celir blocked |

## Non-findings (already coherent)

- CELIR side track vs Git-compat mode boundary SoT is clear.
- Product “verified” vs host-advisory rule is consistent across clean-room SoT and interop matrix (harness Open is expected).
- Native engine vs `git` wrapper decision closed; not reopened.

## Record

| Date (UTC) | Note |
|------------|------|
| 2026-11-08 | Audit authored; production bar + fix/Open dispositions. No Rust crates. |
