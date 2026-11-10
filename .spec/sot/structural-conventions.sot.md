---
id: sot:root/structural-conventions
title: Structural conventions
status: active
created: 2026-11-08T08:54:10Z
updated: 2026-11-08T08:55:00Z
tags: [structure, srp, modular, naming, crates, conventions, agents]
verify: manual
rel:
  depends_on:
    - sot:root/product-naming
    - sot:root/crate-architecture
    - sot:root/conventions-inherit
    - sot:root/public-surface
    - sot:root/product-intent
---

# Structural conventions

Version 0.1.0. Normative **structure and modularity** rules for arks source, Specfold, and agent surfaces. Dependency rings and proposed crate names remain owned by [[sot:root/crate-architecture]]. Product / crate / bin identity remains owned by [[sot:root/product-naming]]. Bottom-up delivery order remains owned by [[sot:root/product-intent]] and rule `70-bottom-up-build-order`.

Convention affinity (cite, do not fork megas): humanymous `C:\workspaces\humanymous\.spec\sot\crate-architecture.sot.md`, `C:\workspaces\humanymous\.spec\sot\rust-migration.sot.md` (Structure and naming), `C:\workspaces\humanymous\.spec\sot\structural-development.sot.md` (bottom-up / `src/<context>` pattern), `C:\workspaces\humanymous\.agents\rules\10-rust-conventions.md`.

## Rule

### 1. Single Responsibility Principle (SRP)

1. **One bounded responsibility per crate** (and per primary module). Split only when dependency direction, target (lib vs bin), or consumer isolation requires it ([[sot:root/crate-architecture]] rule “One responsibility per crate”).
2. Do not merge unrelated domains into one crate merely to reduce package count (e.g. hashing + CLI porcelain).
3. Do not treat file count or wrapper size alone as SRP proof or failure — ownership and consumer reason matter (humanymous crate-architecture affinity).

### 2. Modularization

4. Prefer **inner → outer** modules and facades: pure parsers/format logic separate from I/O; store facade before backends; CLI composes proven plumbing ([[sot:root/crate-architecture]] rings; [[sot:root/storage-celir-boundary]]).
5. Composition roots **wire** services; they do not re-own domain logic. Pure transforms stay free of clocks, randomness, and network I/O unless the owning SoT says otherwise.
6. Specfold product brain (`.spec/`) stays modular by document kind (`sot` / `plan` / `research` / `evidence`); do not dump unrelated product rules into one mega file when a sibling SoT already owns the domain.

### 3. Flat-listing prohibition

7. **No flat dump** of unrelated modules, crates, or docs at one directory level without a parent grouping or ownership reason.
8. Arks examples that fail this rule: dumping every future crate as a bare sibling under repo root; placing unrelated Rust files as a single flat `src/*.rs` catalog with no domain folders; mixing Specfold SoTs, public Pages prose, and scratch notes in one undifferentiated folder.
9. Prefer grouped parents: `crates/arks-*` (when the workspace lands), `.spec/{sot,plan,research,evidence}/`, `docs/` for public narrative only ([[sot:root/public-surface]]).

### 4. Naming convention compliance

10. Product / primary package / CLI bin: **`arks`**; no literal `git` substring in short names ([[sot:root/product-naming]]).
11. Workspace library crates: **`arks-<context>`** kebab-case package names matching [[sot:root/crate-architecture]] (e.g. `arks-hash`, `arks-oid`, `arks-store`).
12. Rust identifiers follow ecosystem norms: snake_case modules/functions/variables, PascalCase types/traits, SCREAMING_SNAKE_CASE constants (cite humanymous rust-migration Structure and naming — do not fork the mega ledger).
13. Specfold document ids and English titles follow existing arks Specfold practice; agent rules stay numbered `NN-kebab.md`.

### 5. Context-based directory structure

14. Group files by **domain context and responsibility**, not by mechanical file type alone when that would mix unrelated owners.
15. Inside a crate, prefer context modules (e.g. `src/<context>/…` or cohesive module trees) with a small public API at `lib.rs` / `main.rs` facades — affinity to humanymous `src/<context>` convention; arks adapts names to Git-compat domains (`hash`, `oid`, `object`, `pack`, `refs`, `store`, …).
16. Keep `.spec/` (product brain) distinct from `docs/` (thin public narrative) and from future `crates/` (implementation).

### 6. Anti-fragmentation

17. **Reject artificial fragmentation**: no new `arks-*` crate (or forwarding-only module) for every helper, config knob, or one-line re-export ([[sot:root/crate-architecture]]; humanymous “no artificial fragmentation”).
18. Each responsibility has **one owning implementation** and one contract; consumers reuse it. Extract shared helpers only when repeated semantics justify it, inside the context that owns those semantics.
19. Standard `lib.rs` / `mod.rs` namespace facades are allowed; redundant forwarding functions and catch-all `util` / `common` dumps are not.

### 7. Language-standard directory structures (prefer ecosystem norms)

20. **Prefer the language ecosystem’s standard layout** when implementing in that language. Do not invent alien trees when a well-known standard exists.
21. **For arks Rust code:** Cargo workspace norms apply first — root virtual workspace, members under `crates/` (proposed `crates/arks-*` plus primary package `arks`), standard `Cargo.toml` + `src/lib.rs` or `src/main.rs`, conventional `tests/` / `benches/` / `examples/` as needed. Exact membership remains design-open until the first workspace lands; update [[sot:root/crate-architecture]] when it does.
22. **Orthogonal overlays:** Git working-tree layout, Specfold (`.spec/`, `spec.toml`), public Pages (`docs/`), and agent assets (`.agents/`) are **product/process overlays**, not substitutes for Cargo layout. They MUST NOT force non-standard Rust source roots when Cargo conventions suffice.
23. If another language appears later under an explicit SoT exception, prefer that language’s standard project layout the same way — still English-only authored prose ([[sot:root/language-english-only]]).

### Agent surfaces

24. Always-on rule: `.agents/rules/15-structural-conventions.md`.
25. Router: root `AGENTS.md` hard constraint + progressive-disclosure row → this SoT.
26. Pointer row: [[sot:root/conventions-inherit]].
27. Governance: [[sot:root/agent-governance]] lists this SoT among hard process constraints agents must honor.

### Out of scope here

- Inventing Git object/pack/ref/index/protocol semantics (owned by [[sot:root/git-compat-surface]]).
- Replacing [[sot:root/crate-architecture]] ring table or proposed crate list.
- Forking humanymous L0–L5 structural ledgers or Rust-migration tables into this repo.
- Final physical tree before the first `Cargo.toml` (layout **intent** is normative; paths update when membership is real).

## Why

Agents otherwise invent flat crate dumps, catch-all utils, alien directory trees, or micro-crates that fight the inner→outer graph. A single arks SoT locks SRP, modularization, anti-flat-listing, naming, context directories, anti-fragmentation, and ecosystem-standard layouts while citing humanymous for deeper Rust/structural pattern detail.

## Verification

1. `spec check` clean for this SoT and its `depends_on` targets.
2. New crate or top-level directory proposals cite this SoT and update [[sot:root/crate-architecture]] when membership changes.
3. Spot-check: no always-on rule or AGENTS hard constraint contradicts rings / bottom-up / product-naming.
4. When a workspace exists: `cargo metadata` membership paths follow `crates/` (or documented Cargo-standard exception) and kebab-case `arks-*` names.
