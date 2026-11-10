---
id: plan:root/20261108085816
title: Program milestones Wave-1
status: active
created: 2026-11-08T08:58:16Z
updated: 2026-11-09T02:52:00Z
tags: [milestones, roadmap, wave-1, bottom-up, confirmation, gix-exit, m1, m12, m13, m14]
rel:
  depends_on:
    - sot:root/program-roadmap
    - sot:root/product-intent
    - sot:root/crate-architecture
    - sot:root/core-object-model
    - sot:root/git-compat-surface
    - sot:root/cli-surface
    - sot:root/storage-celir-boundary
    - sot:root/lease-concurrency-intent
    - sot:root/product-verification-clean-room
    - sot:root/interop-test-matrix
    - sot:root/structural-conventions
    - adr:root/20261108083507
    - plan:root/20261108082148
    - plan:root/20261108082110
    - plan:root/20261108090646
  implements: [sot:root/program-roadmap]
---

# Program milestones — Wave-1 (M1 first wave)

## Outcome

Executable **Wave-1** checklist for **M1 start** only. Canonical milestone identities, **CLI-parity production = M9**, **production-server = M11**, post-production **issue-resolution M12/M13 (G17/G18)** then **future perf M14 (G19 ≥20% after issue-resolution)**, and per-milestone staged plans live in [[sot:root/program-roadmap]] v0.5.0+. This plan does **not** redefine M0–M14.

**Roadmap v0.3:** Implementation **restarts at M1** (gap-fill / re-verify prior trees). Do not treat historical G0/G1 evidence alone as skipping restart discipline without re-check against current SoTs.

## Goal

1. Publish the first-work checklist for M1 (hash / OID / loose objects + verify stubs).
2. Keep later milestones out of Wave-1 execution.
3. Carry the **gix-exit** checklist (mandatory before M4; prefer never introduce).

## Authority

| Concern | Authority |
|---------|-----------|
| Milestone IDs / production exits | [[sot:root/program-roadmap]] (**M9** CLI-parity; **M11** production-server) |
| M1 staged plan | [[plan:root/20261108090646]] |
| Phase letters | [[plan:root/20261108082148]] |
| Core crate slice order | [[plan:root/20261108082110]] |
| Native engine / zero-`gix` | [[adr:root/20261108083507]] |
| Gate IDs / verified venue | [[sot:root/interop-test-matrix]], [[sot:root/product-verification-clean-room]] |
| CLI parity end-state | [[sot:root/cli-surface]] |

## Milestone map (pointer only)

| Milestone | Phase | Production role | Staged plan |
|-----------|-------|-----------------|-------------|
| M0 | Pre-C | Canon | [[plan:root/20261108090645]] |
| **M1** | **C** | Engine start — **this wave / restart** | [[plan:root/20261108090646]] |
| M2 | S | Store | [[plan:root/20261108090647]] |
| M3 | R | Refs/index/repo | [[plan:root/20261108090648]] |
| M4 | P | Pack/protocol + gix-exit | [[plan:root/20261108090649]] |
| M5 | L | Local CLI foundation | [[plan:root/20261108090650]] |
| M6 | H | Remotes client (pre-production) | [[plan:root/20261108090651]] |
| M7 | Pa | Porcelain wave A | [[plan:root/20261108183810]] |
| M8 | Pb | Porcelain wave B | [[plan:root/20261108183811]] |
| **M9** | **Pc** | **CLI-parity production** | [[plan:root/20261108183812]] |
| M10 | E | Lease capability (+ optional high-concurrency) | [[plan:root/20261108183813]] |
| **M11** | **Sv** | **Production-server** | [[plan:root/20261108105446]] |
| **M12** | **Cm** | Issue-resolution compat (**G17**) | [[plan:root/20261109024634]] |
| **M13** | **Cd** | Issue-resolution differentiate (**G18**) | [[plan:root/20261109024635]] |
| **M14** | **Pf** | Comparative ≥20% (**G19**) — **after** M12+M13 | [[plan:root/20261109024636]] |
| CELIR | side | Cancelled / non-product | [[plan:root/20261108090653]] |

## Steps

### Wave-0 (docs)

- [x] Product / architecture / boundary SoTs active (includes CLI-parity raise).
- [x] Build-order + core-slice + roadmap + per-milestone plans through M10.
- [x] Operator confirms M1 **restart** Wave-1 under v0.3.1 before claiming M1 exit again (2026-11-08 restart session).

### Wave-1 — M1 first work — **RESTART UNDER v0.3**

| # | Target | Kind | Notes |
|---|--------|------|-------|
| W1.1 | Root virtual Cargo workspace | scaffolding | Gap-fill/re-verify |
| W1.2 | `crates/arks-hash` | crate | Unit SoT [[sot:root/arks-hash]] |
| W1.3 | `crates/arks-oid` | crate | Unit SoT [[sot:root/arks-oid]] |
| W1.4 | Fixture intent for OID / blob parity | docs/fixtures | `verify/fixtures/m1/` |
| W1.5 | Clean-room / verify stub | verify stub | `verify/` + `images.lock.json` |
| W1.6 | `crates/arks-object` | crate | Unit SoT [[sot:root/arks-object]] |
| W1.7 | Update crate-architecture `rel.governs` | SoT | As needed |
| W1.8 | Host `cargo test` W1.2–W1.6 | advisory | Advisory only |
| W1.9 | Clean-room G0/G1 | verified | **Done** — [[evidence:root/20261108185319]] |

**Wave-1 non-targets:** CLI porcelain; remotes; lease runtime beyond documenting L0; CELIR invention; pack/protocol product claims; introducing `gix`/gitoxide; claiming CLI-parity production.

### gix-exit (mandatory before M4 exit)

Prefer never introducing `gix` so this checklist is vacuously satisfied at M4 ([[adr:root/20261108083507]], [[plan:root/20261108090649]]).

- [x] Prefer never introduce: Wave-1+ use stock-Git fixtures / in-house codecs only.
- [x] If transitional cold `gix` ever added: **N/A** — never introduced (no labeled transitional feature / `dev-dependencies`).
- [x] Before **M4 exit**: zero `gix`/gitoxide edges remain (including optional features and `dev-dependencies`) — proven via `cargo metadata` on M4 evidence receipt.
- [x] Clean-room stock Git remains the only verification oracle.

### Later waves

Owned by per-milestone plans M2–M14 (not confirmed here). Next post-production execution after residual worktree/submodule Done is **M12 Cm-1 + G17 harness** — not M14 perf.

## Record

| Date (UTC) | Note |
|------------|------|
| 2026-11-08 | Plan created; Wave-1 awaiting confirmation. |
| 2026-11-08 | Reframed as **M1 first wave**; production was M6 (v0.2). |
| 2026-11-08 | Operator armed M1/Wave-1; M1 exit evidence recorded historically. |
| 2026-11-08 | **v0.3 CLI-parity:** production = M9; M1 **restart / re-verify** policy. |
