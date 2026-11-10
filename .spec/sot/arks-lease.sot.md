---
id: sot:root/arks-lease
title: arks-lease unit
status: active
created: 2026-11-08T23:12:49Z
updated: 2026-11-10T07:40:00Z
tags: [m10, m23, m26, unit, lease, ring-6, phase-e, l1, l2, l3-opt-in, shared-index, g034-shared-index-contention]
verify: manual
rel:
  governs: [crates/arks-lease]
  depends_on:
    - sot:root/program-roadmap
    - plan:root/20261108183813
    - plan:root/20261109194208
    - plan:root/20261110074000
    - sot:root/lease-concurrency-intent
    - sot:root/shared-index-nway-clean-room
    - sot:root/crate-architecture
    - sot:root/git-compat-surface
    - sot:root/interop-test-matrix
    - sot:root/product-verification-clean-room
    - sot:root/storage-celir-boundary
    - research:root/20261108083502
    - adr:root/20261108232800
    - adr:root/20261109194200
    - adr:root/20261110074000
    - adr:root/20261109122126
    - adr:root/20261108083507
    - evidence:root/20261108150000
  evidence: [evidence:root/20261108231249, evidence:root/20261109194858, evidence:root/20261110074100, evidence:root/20261110074200]
---

# arks-lease unit

Version **0.1.0**. Unit SoT for Ring 6 **default product lease** runtime (`crates/arks-lease`) under [[plan:root/20261108183813]] / roadmap Phase **E** (**M10**). Normative intent: [[sot:root/lease-concurrency-intent]]. **Entry:** M9 CLI-parity production proven — [[evidence:root/20261108150000]].

**M10 does not redefine M9.** Lease capability is a separate bar after CLI-parity production.

## Rule

### Role

1. **Default product leases.** Ship **L1** (in-process grant/release/expire) → **L2** (on-disk durable lease state) as the **default product capability**.
2. **Opt-in L3 only.** High-concurrency N-way pipeline is behind an **explicit** feature/config valve (`high-concurrency` Cargo feature and/or runtime mode flag). MUST NOT be the sole writer path.
3. **Lease ≠ skip Git locks.** On Git-compat paths, holding a lease MUST NOT bypass ref `*.lock` or `index.lock` bytes ([[sot:root/lease-concurrency-intent]] Dual path).
4. **Dual path.** Default = Git-compat mode (leases pre-coordinate; Git locks still used). High-concurrency product mode = opt-in **Claimed** shape per [[adr:root/20261109194200]] + [[evidence:root/20261109194858]]; export = identity for claimed apply. **M10 history:** L3 was refuse-closed ([[adr:root/20261108232800]], [[evidence:root/20261108231249]]).
5. **Inward deps.** MAY depend on Rings 0–4 as needed for resource ids / repo paths / batch apply. MUST NOT depend on `arks` CLI types, `arks-remote`, `arks-server`, CELIR, or `gix`.

### Claimed surface

| Layer | Claim | Notes |
|-------|-------|-------|
| **L1** | In-process `LeaseManager`: grant / release / expire / inspect | Resource id = opaque string (ref namespace or explicit id); TTL fencing |
| **L2** | Durable store under `.git/arks/leases/` (product-private beside Git layout) | JSON lines or single lease table file; stock Git ignores unknown paths |
| **L3** | Feature `high-concurrency` + `WriterMode::{GitCompat, HighConcurrency}` + lease/batch-apply/worktree-ref-shard pipeline | **Claimed** [[evidence:root/20261109194858]] |
| **Shared-index N-way (M26)** | Exclusive `index:shared` lease + `arks-index` `index.lock` write path | **Claimed + G34 Verified** [[evidence:root/20261110074100]] / [[evidence:root/20261110074200]] |
| Git locks | Unchanged requirement | Callers / pipeline still use `arks-refs` / `arks-index` lock APIs; lease crate MUST NOT wrap-away locks |

### Out of scope

- Inventing Git lock-file dialects on Git-compat paths.
- CELIR; forge; lease-over-wire for M11 server.
- Forcing N-way as sole path; redefining M9 success.
- Claiming shared-index Verified without **G34** Docker PASS.
- Substituting M24/G22/G24 for shared-index proof.

## Closed / refuse (explicit)

1. **L3 claimed shape** — **Claimed** ([[adr:root/20261109194200]], [[evidence:root/20261109194858]]). M10 refuse-close ADR is **superseded** for this shape only.
2. **Shared-index N-way** — unlocked by [[adr:root/20261110074000]] (**M26**); host advisory Claimed; product Verified = **G34** only. Not part of M23 L3 Done.
3. **Allocator helper identity** — **Closed** as ring-internal modules (no dedicated crate) per [[adr:root/20261109122126]]; principles [[research:root/20261108083502]].

## Why

M10 ships the lease **runtime** already specified as default product SoT, after CLI-parity, without collapsing Git-compat interchange or inventing concurrency as a lock replacement.

## Verification

1. Host (advisory): `cargo test -p arks-lease` (incl. `--test g034-shared-index-contention_shared_index_contention`).
2. Clean-room **G11** `lease-product` + L3 stress expansion per [[plan:root/20261109194208]] — authoritative for L3 Claimed.
3. Clean-room **G34** shared-index contention per [[plan:root/20261110074000]] / [[sot:root/shared-index-nway-clean-room]] — authoritative for shared-index Verified.
4. Prove dual-path: Git-compat still acquires ref/index locks while lease held; HighConcurrency claimed shape unlocked; shared-index still takes `index.lock`.
5. `spec check` clean; zero `gix` / zero CELIR ObjectStore edges.
