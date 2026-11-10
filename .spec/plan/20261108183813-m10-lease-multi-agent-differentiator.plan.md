---
id: plan:root/20261108183813
title: M10 Lease capability and optional high-concurrency
status: active
created: 2026-11-08T18:38:13Z
updated: 2026-11-08T23:28:00+09:00
tags: [milestone, m10, phase-e, lease, high-concurrency, opt-in]
rel:
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108183812
    - sot:root/lease-concurrency-intent
    - research:root/20261108083502
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - adr:root/20261108083507
    - adr:root/20261108232800
    - sot:root/arks-lease
    - evidence:root/20261108231249
  implements: [sot:root/program-roadmap]
---

# M10 — Default lease capability + optional high-concurrency (Phase E)

## Goal

After **CLI-parity production (M9)**, ship **default product leases** (L1→L2) and, as claimed, an **opt-in / selectable** high-concurrency path toward **N ≈ hundreds** agents — without replacing Git lock bytes on Git-compat paths, without forcing N-way as the only writer path, and **without regressing CLI parity**.

Leases are **default product specification** ([[sot:root/lease-concurrency-intent]]); M10 is the **shipping** milestone for that capability — not “leases were optional marketing until M10.”

## In scope / crates

| Artifact | Role |
|----------|------|
| `crates/arks-lease` | L1 in-process → L2 durable (**default product capability**); L3 pipeline only as **opt-in** per lease SoT |
| Dual-path modes | **Git-compat** (default interchange / classic serial locks) vs **high-concurrency product mode** (selectable; export/import required if divergent) |
| Allocator principles | Follow research; **no invented allocator crate** until Open closes |

## Out of scope

- Skipping Git locks because a lease is held (Git-compat mode) — **lease ≠ skip Git locks**.
- Inventing lock-file dialects or CELIR APIs to “solve” concurrency.
- Treating M10 as a blocker for M9 production claims.
- Silently dropping CLI-parity gates.
- Forcing high-concurrency mode as the sole writer path.
- Treating “leases = differentiator fluff deferred forever” as product truth.

## Depends on

**M9** exit by default. Local-only lease earlier requires explicit SoT amendment to [[sot:root/program-roadmap]].

**Prior L0:** Git locks already honored since M3 — do not re-litigate L0 here.

## Exit gates

| Gate | Clean-room | Advisory |
|------|------------|----------|
| **G11** lease-product | Authoritative when lease harness exists | Host unit tests |
| G1–G7 + G12–G14 regression | Must stay green | Host |

## Ordered work items

1. **Design** — Close or explicitly carry Opens from [[sot:root/lease-concurrency-intent]] / [[research:root/20261108083502]] via ADR before L3 bytes. Dual-path / selectable high-concurrency is **already locked** in the lease SoT.
2. **Scaffold** — `arks-lease` crate; feature flags distinguishing **default lease** (L1/L2) vs **opt-in high-concurrency** (L3).
3. **Implement** — L1 grant/release/expire → L2 on-disk state → L3 **refuse-close** for M10 production ([[adr:root/20261108232800]]): valve + `WriterMode::HighConcurrency` clear error; N-way bytes only after a **superseding** ADR + G11 expansion.
4. **Verify** — G11 + CLI-parity regression; document lease vs lock interaction; stock Git still opens Git-compat repos; high-concurrency claims include export/import when divergent.

## Open questions

- Lock sharding vs global critical section; worktree-per-agent; batch commit; exact CLI/config valve for high-concurrency select — remain **Open**.
- **L3 N-way pipeline ship** — **refuse-closed** for M10 production bars ([[adr:root/20261108232800]]; [[evidence:root/20261108231249]] L1→L2). Dual-path selectability and opt-in valve are **closed**; pipeline implementation is not claimed.
- **Allocator helper crate placement** — **deferred Open** (explicit) per [[adr:root/20261108232800]]; principles in [[research:root/20261108083502]] only; no invented crate until a future ADR.

## Cross-links

[[sot:root/program-roadmap]], [[sot:root/lease-concurrency-intent]], [[sot:root/arks-lease]], [[research:root/20261108083502]], [[adr:root/20261108083507]], [[adr:root/20261108232800]], [[evidence:root/20261108231249]]

## Supersession

Replaces the production-ordering role of former [[plan:root/20261108090652]] (old M7 lease under v0.2 roadmap). Title retained historically as “differentiator” in the filename for Specfold id stability; normative name is **lease capability + optional high-concurrency**.
