---
id: sot:root/lease-concurrency-intent
title: Lease concurrency intent
status: active
created: 2026-11-08T08:21:10Z
updated: 2026-11-10T07:40:00Z
tags: [lease, concurrency, locks, agents, allocator, two-tier, m23, m26, l3, shared-index, g034-shared-index-contention]
verify: manual
rel:
  depends_on: [sot:root/product-intent, sot:root/crate-architecture, sot:root/git-compat-surface, sot:root/product-verification-clean-room, sot:root/storage-celir-boundary, sot:root/program-roadmap, adr:root/20261108232800, adr:root/20261109194200, adr:root/20261110074000, adr:root/20261109122126, sot:root/arks-lease, sot:root/shared-index-nway-clean-room, plan:root/20261109194208, plan:root/20261110074000]
  evidence: [research:root/20261108083502, research:root/20261109023117, evidence:root/20261108192100, evidence:root/20261108231249, evidence:root/20261109194858, evidence:root/20261110074100, evidence:root/20261110074200]
---

# Lease concurrency intent

Version **0.4.0**. Normative **intent** for product **leases**, optional **high-concurrency** multi-writer mode, and how they relate to Git’s documented **lock** behaviors. Product: [[sot:root/product-intent]]. Crate: `arks-lease` in [[sot:root/crate-architecture]]. Compatibility envelope: [[sot:root/git-compat-surface]]. Milestone ladder: [[sot:root/program-roadmap]]. Verification: [[sot:root/product-verification-clean-room]]. Scale / allocator research stub: [[research:root/20261108083502]]. Multi-agent scenario ID catalog (does not redefine M9/M10/M11 or ship L3): [[sot:root/multi-agent-git-concurrency-scenarios]] / [[research:root/20261109023117]]. CELIR is **out of product deps** ([[sot:root/storage-celir-boundary]] v0.6.0+); historical CELIR concurrency notes are N/A for the product store.

This SoT does **not** invent Git lock-file formats. Ref and index locking remain as documented by Git (update-ref / index lock patterns cited via [[research:root/20261108071914]] and [[research:root/20261108071915]]).

## Rule

### Two-tier product model (normative)

| Tier | Status | Meaning |
|------|--------|---------|
| **Product leases** | **Default product specification** | Baseline agent-oriented coordination capability. Always part of the product SoT — **not** a late optional marketing add-on, and **not** “deferred forever because M10 is after M9.” Delivery of the lease **runtime/API** still follows bottom-up milestones (see Phasing / Milestone mapping). |
| **High-concurrency multi-commit pipeline** (N-way / bottleneck-relief path) | **Optional / selectable** | Opt-in mode aimed at **N ≈ hundreds** concurrent writers. **Not** forced as the only writer path. Users who want classic serial Git-lock behavior stay on the **Git-compat** path. |

Rules:

1. Product leases MUST be designed as **default product capability** in SoT and architecture (`arks-lease`), even while M1–M9 delay **shipping** the lease runtime.
2. High-concurrency mode MUST be **explicit and selectable** (capability, config, feature flag, or repo extension negotiation — exact valve open; must not silently become the only path).
3. Git-compat / classic serial lock behavior MUST remain available for claimed interchange and alias workflows.
4. **Lease ≠ skip Git locks** on Git-compat paths (see Dual path).

### Problem statement (primary pain)

1. **Scale target (optional mode):** arks MUST be **capable** of a high-concurrency path so **N ≈ hundreds** of agents can attempt commits / coordinated mutations at once against a shared logical repository without throughput collapsing to single-writer serialization — as an **opt-in** product mode, not a forced default.
2. **Git lock reality:** Stock Git uses short-lived exclusive locks (e.g. ref `*.lock`, index lock) that **serialize** writers. Under dense multi-agent commit storms, lock contention is the dominant failure mode — not lack of porcelain flags.
3. **arks response:** **Default product leases** coordinate agents; an **optional high-concurrency commit pipeline** MAY relieve lock storms — while preserving **stock-Git interchange** for Git-compat paths and for any product-mode export/import ([[research:root/20261108083502]]).
4. **Engine constraint:** Coordination and commit pipeline run **in the native Rust engine** ([[adr:root/20261108083507]]); leases are not a shell script around `git`.

### Two mechanisms (normative split)

| Mechanism | Owner | Purpose |
|-----------|-------|---------|
| **Git locks** | Git-compat surface | Short-lived exclusive locks for ref updates (e.g. `*.lock`), index writes, and other documented critical sections so stock Git and arks do not corrupt shared files |
| **Product leases** | This SoT / `arks-lease` | Longer-lived, agent-oriented coordination grants (who may mutate which logical resource) — **default product capability**, additive to Git locks on Git-compat paths; **not** a replacement for Git lock bytes on those paths |

1. Product leases MUST NOT redefine object IDs, pack layouts, or refname rules.
2. Holding a product lease MUST NOT skip Git locks when writing **Git-compatible** refs/index/objects on a stock-interchange path.
3. Absence of a **shipped** lease runtime MUST still allow single-process Git-compat operation (M1–M9).
4. **Lease ≠ skip Git locks** on stock-compatible repos. Only an **explicit opt-in high-concurrency product mode** MAY diverge, and only with a documented export/import path back to stock Git (see Dual path below).

### Dual path (Git-compat interchange vs high-concurrency product mode)

| Mode | Default? | Writer coordination | On-disk / interchange rule |
|------|----------|---------------------|----------------------------|
| **Git-compat mode** | **Yes** (claimed interchange / classic serial locks) | Honor documented Git locks; leases (when shipped) MAY **pre-coordinate** agents but do **not** replace lock bytes | Repo MUST remain readable/writable by stock Git for the claimed subset |
| **High-concurrency product mode** | **No** — **opt-in / selectable** | Lease + batch / sharded / worktree / queued commit pipeline aimed at N≈hundreds | MAY use product-private coordination artifacts **beside** or **staged into** Git layout; MUST provide **export** (and ideally **import**) that yields a stock-Git-readable repository without invented object IDs |

Rules for high-concurrency product mode:

1. Mode MUST be **explicit** (capability, config, feature flag, or repo extension negotiation — exact valve open; must not silently diverge or replace Git-compat as the only path).
2. Export/import MUST preserve OID identity and ref meanings per [[sot:root/git-compat-surface]] / [[sot:root/core-object-model]].
3. Clean-room gates MUST prove both modes when claimed ([[sot:root/product-verification-clean-room]]).
4. Do not invent CELIR APIs or pull CELIR into the product graph to “solve” concurrency ([[sot:root/storage-celir-boundary]] v0.6.0+).
5. **Product leases own multi-writer coordination on the filesystem Git store.** CELIR concurrency (single-writer index / DB Process) is **N/A for product** — CELIR is excluded from dependencies. Leases still MUST NOT skip Git locks on Git-compat filesystem paths.

### Intent (what leases are for)

1. **Agent workflows:** coordinate concurrent planners/workers on the same logical repo without inventing a new object-ID scheme — **default product capability**.
2. **Scoped grants:** lease a ref namespace, worktree path set, or explicit resource id — exact grant schema is phased; do not invent wire bytes before a lease-format research/ADR.
3. **Timeout / fencing:** leases are expected to expire or be fenced so dead agents cannot block forever — exact algorithm deferred; intent only here.
4. **Observability:** lease state SHOULD be inspectable by arks tooling when the feature ships; not required for MVP object/ref slices.
5. **Optional pipeline coupling:** leases SHOULD be able to feed an **opt-in** high-concurrency commit pipeline (batch apply, ordered critical section, or sharded resources) — mechanism open in [[research:root/20261108083502]]; pipeline is **not** mandatory for every user of leases.

### Memory / allocator coupling (intent only)

Extreme concurrency (when the opt-in mode is claimed) requires an **allocation policy** that avoids allocator lock convoys and excess copies on hot paths (arena/pool/zero-copy **principles**). Normative product goal lives in [[sot:root/product-intent]]; principles in [[research:root/20261108083502]]. This SoT does **not** invent allocator APIs or re-open CELIR as a product store. **Allocator helper identity** is **Closed** as **ring-internal modules** (no dedicated allocator crate) per [[adr:root/20261109122126]].

### Explicit non-goals

1. Replacing `git update-ref` locking semantics with a custom lock file dialect in Git-compat mode.
2. Distributed consensus research (Raft, etc.) as a day-one dependency.
3. Claiming lease correctness “verified” without clean-room gates ([[sot:root/product-verification-clean-room]]).
4. Coupling leases to CELIR (excluded from product deps) — leases target the filesystem Git store.
5. Closing the open design questions below without a dedicated research/ADR revision (except the dual-path / selectable-mode decision already locked above).
6. Re-introducing CELIR path-deps or inventing CELIR multi-writer APIs ([[sot:root/storage-celir-boundary]]).
7. Treating “leases = M10 marketing differentiator only / optional forever” as product truth — delivery timing ≠ product-spec optionality.
8. Forcing high-concurrency mode as the sole writer path for all arks users.

### Phasing

| Phase | Deliverable | Product role |
|-------|-------------|--------------|
| **L0** | Honor Git locks only; no product lease API; core object/ref slices proceed in-engine | Git-compat baseline |
| **L1** | In-process lease manager API in `arks-lease` (grant/release/expire intent); no cross-host protocol | **Default product lease capability** (API begins) |
| **L2** | On-disk lease state durable across arks processes; still additive to Git locks on Git-compat paths | **Default product lease capability** (durable) |
| **L3** | Opt-in high-concurrency commit pipeline aimed at N≈hundreds; **claimed shape** locked by [[adr:root/20261109194200]] (lease + batch-apply + worktree/ref-shard); **Claimed** [[evidence:root/20261109194858]]; export = identity for claimed apply. Shared-index N-way is **additive M26** ([[adr:root/20261110074000]]; Verified via **G34**) — not required for L3 Done | **Selectable / optional** (**Claimed**) |

Bottom-up rule ([[sot:root/product-intent]]): do not implement L2/L3 before L0 Git-lock correctness and Rings 0–4 object/ref stores exist. Do not implement L3 before L1 intent is clear.

### Milestone mapping (delivery vs product-spec)

| When | What ships / holds | Notes |
|------|--------------------|-------|
| **M3** (Phase R) | **L0** — Git locks honored on refs/index updates | Required for Git-compat; not a lease API |
| **M1–M9** | No product lease **runtime** by default | Lease remains **default product SoT**; do not pull L1+ into M1–M9 without an explicit roadmap amendment |
| **M10** (Phase E) | **L1 → L2** lease API as **default product capability** shipped after CLI-parity (**M9**) | M10 is the **shipping** milestone for the baseline lease capability — **not** “leases were optional marketing until then.” G11 + CLI-parity regression |
| **M10** production | L1→L2 only; L3 was refuse-closed | [[adr:root/20261108232800]], [[evidence:root/20261108231249]] |
| **M23** | **L3** claimed shape ship — **opt-in** | [[adr:root/20261109194200]] + [[plan:root/20261109194208]] + [[evidence:root/20261109194858]] |
| **M26** | Shared-index N-way — **opt-in additive** | [[adr:root/20261110074000]] + [[plan:root/20261110074000]]; product Verified = **G34** |

Marketing MUST NOT say “leases are only an M10 differentiator deferred forever.” Accurate framing: **leases are default product spec; M10 ships that capability; high-concurrency N-way is the optional selectable mode.**

### Interaction with remotes

Remote push/fetch concurrency follows Git protocol and host rules ([[sot:root/remote-host-compatibility-surface]]). Product leases do not invent GitHub lock APIs. Server-role lease exposure over the wire remains **out of scope** until this SoT explicitly claims it (**M11** locks Git server wire roles without implying lease-over-wire).

### Open design questions

| # | Question | Status |
|---|----------|--------|
| 1 | Single-repo **lock sharding** (per-ref / per-namespace) vs global CS for tip update? | **Closed for claimed shape** — ordered short CS over leased **ref shards** ([[adr:root/20261109194200]]); shared-index is separate M26 path |
| 2 | **Worktree-per-agent** (or isolated index) with lease-mediated apply? | **Closed for L3 claimed shape** — worktree / ref-shard isolation REQUIRED for M23; shared-index N-way unlocked under **M26** / **G34** ([[adr:root/20261110074000]]) |
| 3 | **Lease + batch commit** (agents stage; engine applies under Git locks)? | **Closed for claimed shape** — parallel prep + ordered batch apply under short Git locks |
| 4 | How allocator arenas interact with pack decode / tree walk / concurrent hash? | **Open** (principles [[research:root/20261108083502]]; placement [[adr:root/20261109122126]]) |
| 5 | Exact CLI/config/feature-flag valve? | **Closed enough** — `WriterMode` + `high-concurrency` feature; CLI surface follows [[plan:root/20261109194208]] |

**M10 history:** [[adr:root/20261108232800]] refuse-closed L3 for M10 bars; [[evidence:root/20261108231249]] proves L1→L2 + valve only.

**M23 Claimed:** [[adr:root/20261109194200]] + [[evidence:root/20261109194858]]. Shared-index N-way is **not** required for product L3 Done.

**M26 Claimed + G34 Verified:** [[adr:root/20261110074000]] supersedes refuse [[adr:root/20261110160000]]. Host [[evidence:root/20261110074100]]; Docker [[evidence:root/20261110074200]] ([[sot:root/shared-index-nway-clean-room]]). Lease on `index:shared` is **additive** to `index.lock` — never a skip.

**Closed (allocator identity):** helper placement = **ring-internal** (no dedicated crate) — [[adr:root/20261109122126]]; principles remain [[research:root/20261108083502]].

**Closed here:** High-concurrency product mode is a **separate, opt-in / selectable** path; Git-compat classic serial locks remain the default interchange path; leases are **default product specification**; L3 **claimed shape** is M23 ([[plan:root/20261109194208]]); shared-index N-way is M26.

## Why

Hundreds of agents committing at once expose Git’s exclusive lock model as a product ceiling. Leases are the **baseline** product coordination answer; a dual-path rule keeps stock-Git interchange honest while allowing an **optional** high-concurrency mode with export. Keeping leases additive on Git-compat paths prevents forking the ecosystem; keeping the pipeline in-engine ([[adr:root/20261108083507]]) makes allocator and concurrency policy controllable. Separating “default lease spec” from “optional N-way pipeline” and from “M10 shipping milestone” stops marketing language from contradicting product SoT.

## Verification

1. `spec check` clean for this SoT.
2. L0: ref/index update paths document Git-lock behavior against primary sources and run in-engine (no `git` subprocess for product ops).
3. L1+: unit tests host-advisory; product claims require clean-room per [[sot:root/product-verification-clean-room]].
4. PRs must not remove Git locks because a lease is held on Git-compat paths.
5. Any high-concurrency mode divergence MUST document export/import and clean-room gates before “verified.”
6. Slice ordering: [[plan:root/20261108082110]] keeps lease work after core object/ref slices; [[sot:root/program-roadmap]] M10 ships L1/L2; L3 remains opt-in.
7. Docs/marketing that claim “leases are optional forever / only M10 fluff” fail this SoT.
