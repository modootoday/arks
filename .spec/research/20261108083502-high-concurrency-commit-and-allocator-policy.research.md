---
id: research:root/20261108083502
title: High-concurrency commit and allocator policy
status: active
created: 2026-11-08T08:35:02Z
updated: 2026-11-08T09:15:00Z
tags: [concurrency, lease, allocator, agents, performance]
rel:
  depends_on: [sot:root/product-intent, sot:root/lease-concurrency-intent, sot:root/crate-architecture, sot:root/git-compat-surface, adr:root/20261108083507]
---

# High-concurrency commit and allocator policy

## Question

What **principles** should guide arks’s high-concurrency commit pipeline and memory allocation policy so that **N ≈ hundreds** of agents can commit concurrently, without inventing unfinished CELIR APIs and without replacing documented Git locks on Git-compat interchange paths?

## Method

1. Capture product pain from [[sot:root/product-intent]] and [[sot:root/lease-concurrency-intent]]: Git exclusive locks serialize dense multi-agent writers.
2. Separate **Git-compat interchange mode** from a possible **high-concurrency product mode** (export/import required).
3. Record allocator principles (arena / pool / zero-copy **intent only**) for the native Rust engine ([[adr:root/20261108083507]]).
4. List open design alternatives; do not pick wire formats or CELIR schemas.
5. Cite Git lock documentation via existing refs/index research only when discussing L0 lock honor — no new lock-file invention.

Research snapshot date: **2026-11-08**.

## Findings

### Compatibility invariant

- On Git-compat paths, product concurrency MUST still take documented Git locks when writing refs/index/objects ([[sot:root/lease-concurrency-intent]] L0+).
- Stock Git remains a **peer oracle** for clean-room round-trips; it is not the runtime ([[adr:root/20261108083507]]).
- Object IDs and pack/ref meanings stay under [[sot:root/git-compat-surface]] / [[sot:root/core-object-model]].

### High-concurrency commit pipeline (principles)

| Principle | Meaning |
|-----------|---------|
| Pre-coordinate with leases | Agents acquire scoped grants before entering expensive write paths |
| Shrink exclusive critical sections | Prefer long parallel prep (hash blobs, build trees) + short locked tip update |
| Batch when possible | Multiple agent results MAY apply under one ordered apply step (design open) |
| Explicit product mode if layout diverges | No silent private encodings; export/import to stock Git required |
| Measure contention | Future clean-room gates SHOULD include multi-writer stress when L3 claims ship |

Candidate shapes (**open** — see lease SoT open questions): lock sharding; worktree-per-agent; lease+batch commit; separate concurrency mode.

### Memory allocator policy (principles only)

| Principle | Meaning |
|-----------|---------|
| Hot-path arenas / pools | Prefer bump/arena or sized pools for transient decode/encode buffers under concurrency |
| Minimize cross-thread allocator traffic | Avoid designs that convoy on a global allocator during N-way commit storms |
| Zero-copy where safe | Prefer views into mmap/pack windows over reflexive full copies; do not violate ownership/safety |
| No CELIR API invention | Allocator policy is engine-local; CELIR remains behind [[sot:root/storage-celir-boundary]] |
| Measurable | Allocator claims need benchmarks + clean-room stress when product-advertised |

### Non-findings / deferred

- Exact lease wire format, fencing algorithm, and batch scheduler — deferred to later ADR.
- gitoxide/`gix` dependency — closed: hot path always in-house; end state zero Cargo edges; transitional cold only under [[adr:root/20261108083507]] with mandatory exit before M4.
- Concrete Rust allocator crate choices — implementation ADR later.

## Open decisions

1. Single-repo lock sharding vs worktree-per-agent vs lease+batch vs dual mode (primary).
2. ~~Where arenas live in the crate graph~~ — **Closed** as ring-internal modules (no dedicated crate) by [[adr:root/20261109122126]]; fine-grained store vs object module choice remains implementation detail under that ADR.
3. Stress-test topology for N≈hundreds in clean-room (compose services, synthetic agents).

## Links

- Normative intent: [[sot:root/lease-concurrency-intent]], [[sot:root/product-intent]]
- Engine architecture: [[adr:root/20261108083507]], [[sot:root/crate-architecture]]
