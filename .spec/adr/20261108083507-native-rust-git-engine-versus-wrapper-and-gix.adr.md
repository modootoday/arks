---
id: adr:root/20261108083507
title: Native Rust Git engine versus wrapper and gix
status: active
created: 2026-11-08T08:35:07Z
updated: 2026-11-08T09:15:00Z
tags: [engine, rust, architecture, gix, gitoxide]
rel: {}
---

# Native Rust Git engine versus wrapper and gix

## Context

arks must be Git-**compatible** (byte/semantic/protocol fidelity and stock-Git interchange) while solving multi-agent lock contention and owning memory allocation policy ([[sot:root/product-intent]], [[sot:root/lease-concurrency-intent]]). Two architecture classes were considered:

1. **Subprocess wrapper** — CLI that shells out to stock `git` for core ops.
2. **Native Rust engine** — in-process implementation of Git formats/protocols/semantics in arks crates.

Separately: whether the workspace may **depend on gitoxide / `gix`** at all, versus clean-room implementing every codec, and whether any temporary cold-path use is allowed during early milestones.

## Decision

1. **Accepted — native Rust Git engine.** Product runtime paths MUST implement claimed Git ops **in-engine** (parse, hash, encode, ref/index/pack/repo IO, claimed protocol helpers) inside the crate graph ([[sot:root/crate-architecture]]). Compatibility is **interoperability with stock Git**, not embedding stock Git as the implementation.
2. **Rejected — thin wrapper around the `git` executable** for product paths. Stock `git` MAY appear only as:
   - clean-room **oracle** / acceptance peer ([[sot:root/product-verification-clean-room]]);
   - fixture generator in tests;
   - optional developer convenience tooling explicitly labeled non-product.
3. **Accepted — zero `gix` / gitoxide end state; hot path always in-house.** Closing the gitoxide item (amends the earlier “permanent optional cold `gix`” wording):
   - **End state (MUST).** Product and workspace default MUST reach **no Cargo dependency** on `gix` / gitoxide (or related crates) — **not** as a permanent optional feature, and **not** as a permanent `dev-dependencies` design. Clean-room **stock Git** remains the **only** verification oracle for product “verified” claims ([[sot:root/product-verification-clean-room]]). `gix` is **never** a substitute oracle.
   - **Hot / product paths MUST be in-house at all times.** Rings **0–4** runtime (`arks-hash`, `arks-oid`, `arks-object`, `arks-pack`, `arks-index`, `arks-refs`, `arks-store`, `arks-repo`), `arks-lease`, allocator / arena policy, and the high-concurrency commit pipeline MUST NOT depend on `gix` / gitoxide for hashing, OID types, loose object IO, refs, store facade, or commit orchestration.
   - **Always in-house regardless of features:** OID / hash, hot commit path, allocator / arena helpers used on product paths, leases, and store concurrent-write orchestration.
   - **Default recommendation: never introduce `gix`.** Prefer stock-Git fixtures and in-house codecs from Wave-1 onward.
   - **Transitional cold `gix` (MAY, temporary only).** If early milestones introduce `gix` at all, it MUST be:
     - explicitly labeled **transitional** in the Cargo feature / crate docs;
     - feature-gated (e.g. non-default `oracle-gix`) and/or confined to temporary `dev-dependencies`;
     - limited to cold scaffolding (fixtures, advisory decode helpers, labeled developer tooling);
     - **not** exported through Rings 0–4 public APIs;
     - **not** required for default product builds;
     - **removed** before the **gix-exit** gate (see below) — permanent optional/`dev-dependencies` `gix` is **rejected** as an end state.
   - **gix-exit gate (concrete).** Any transitional `gix` / gitoxide edge MUST be gone from the workspace (including optional features and `dev-dependencies`) **before M4 exit** (pack + protocol library claims, [[sot:root/program-roadmap]]). Checklist: [[plan:root/20261108085816]] **gix-exit**. Prefer never landing the dependency so the checklist is vacuously satisfied.
   - This item does **not** reopen decision (1)–(2) (native engine vs `git` subprocess).

## Consequences

- [[sot:root/crate-architecture]] forbids “must invoke `git` binary for core ops” as architecture; engine crates own parsing/IO; Rings 0–4 and lease/allocator hot paths stay free of `gix`; workspace end state is zero `gix`.
- [[sot:root/git-compat-surface]] defines compat as in-engine fidelity to primary docs + stock-Git round-trip tests.
- Performance, lease, and allocator work can target in-process concurrency ([[research:root/20261108083502]]) without inheriting gitoxide allocator or lock models on product paths.
- Transitional cold `gix`, if ever used, does not relax clean-room stock-Git verification authority and MUST exit before M4.
