# AGENTS.md — arks

Cross-tool always-on rules. Keep this file **short** — details load on demand.
This file is a **router**, not a second Source of Truth. Normative product intent lives in `.spec/sot/`.

## Mission

Build **arks** (`arks for Git`; working expansion **Agent Revision Kinetic System** per naming SoT) — a Git-compatible replacement CLI and infra for multi-session revision contention.
**Standards compatibility first; performance second.** Do not invent Git semantics without a cited public Git project / git-scm primary document. Product/crate/bin name lock: `.spec/sot/product-naming.sot.md`.

## Hard constraints

1. **English only** — all authored project text (Specfold docs, AGENTS, `.agents/**`, `docs/`, comments, commit/PR prose) is English. Proper nouns (`arks`, `CELIR`, `humanymous`) stay. SoT: `.spec/sot/language-english-only.sot.md`; rule `05-english-only`.
2. **Standards-first** — object/pack/ref/index/protocol behavior from primary docs only (`git-compat-surface` + research catalog).
3. **Clean-room authoritative** — product “verified” claims require Docker clean-room; host runs are advisory (`.spec/sot/product-verification-clean-room.sot.md`).
4. **No literal competing SoT** — do not restate or fork humanymous convention ledgers in this repo; cite live paths under `C:\workspaces\humanymous`.
5. **CELIR / lease boundary** — filesystem `.git` is the sole Git-compat **object store**; CELIR MAY appear only as optional index-only feature `celir-index` (default OFF). Lease/concurrency has its own SoT. Never silently replace Git object-store semantics in Git-compat mode (`.spec/sot/storage-celir-boundary.sot.md`, `.spec/adr/20261110052030-celir-index-optional-product-path-dep.adr.md`).
6. **Public surface** — repo will be public; no secrets in tracked paths; **human** (`docs/`, README, LICENSE, SECURITY) vs **agent** (`AGENTS.md`, `.agents/`, `.spec/`, `verify/` matrices) vocabulary split; thin Pages vs Specfold brain; English-only public copy; no “faster than Git” marketing outside receipts. SoT: `.spec/sot/public-surface.sot.md`; rule `80-public-surface`.
7. **Structural conventions** — SRP, modularization, no flat dumps, naming compliance, context-based dirs, anti-fragmentation, and language-standard layouts (Rust Cargo workspace / crates members first; Specfold, docs, and agents are overlays). SoT: `.spec/sot/structural-conventions.sot.md`; rule `15-structural-conventions`.
8. **No folklore** — prefer updating `.spec/` over chat-only process invention.

## Progressive disclosure (read more only when needed)

Convention canon for Specfold / agent layout: **cite** humanymous live trees
(`C:\workspaces\humanymous\AGENTS.md`, `C:\workspaces\humanymous\.agents\`,
`C:\workspaces\humanymous\.spec\sot\structural-development.sot.md`) — do not copy mega ledgers.

Before changing governed code or specs: `spec context <path>` → edit → `spec check`.

Current program honesty (pointers only — owning SoTs win): **M0–M26**; gates **G0–G37** (**G33** CRED, **G34** shared-index, **G35** depth-only, **G36** Basic, **G37** gpgsig SSH). **M25** server transports **G29–G32 Claimed**; **M26 / G34 Claimed+Verified**; **G28** live-private journeys **Claimed** + offline **J-CLONE-LOCAL/PATH** PASS (checklist N/A=0; land tip ~`902d728`); L3 **Claimed** shape (worktree/ref-shard). W1–W5 residual honesty: **CRED G33**, hooks honor list, **GPGSIG** SSH create/verify (OpenPGP N/A), **G9 `--depth`/G35** (filter/promisor N/A), **AUTH Basic G36** (OAuth Refuse), **PF5 Claimed 8/8** (tip remasure primary/carrying `79fc9bf` [[evidence:root/20261110142500]]: LOCAL **0.1579** / PATH **0.1739**; miss=[]; prior `875359e` / 7/8 packs historical), **FW-G8-DEFAULT** / **FW-GH-LIVE-SCALE** harness Claimed. Catalog **Deferred=0**; CELIR **index-only** dual-path (**M24**, ObjectStore Refuse; **G22–G25** Verified). Do not invent live PASS and do not clash sole-runner Docker with a sibling live-GH agent.

| If you are working on… | Read next |
|------------------------|-----------|
| Product / crate / bin name | `.spec/sot/product-naming.sot.md` |
| Product intent | `.spec/sot/product-intent.sot.md` |
| Program roadmap / production exits (M9 CLI-parity; M11 server; M16 active alias; M17 journeys; M21 Deferred=0; M23 L3 Claimed; M24 CELIR index dual-path; M25 server transports; M26 shared-index) / milestones | `.spec/sot/program-roadmap.sot.md` + `.spec/plan/*-program-milestones*.plan.md` + `.spec/plan/*-m[0-9]-*.plan.md` + `.spec/plan/*-m1[0-7]-*.plan.md` + `.spec/plan/*-m2[0-6]-*.plan.md` |
| Post-bar residual disposition (no bare development Opens; Deferred=0; §F W1–W5 Claimed honesty) | `.spec/sot/production-residual-register.sot.md` |
| Git CLI symmetric compatibility | `.spec/sot/cli-surface.sot.md` |
| Crate architecture | `.spec/sot/crate-architecture.sot.md` |
| Native engine vs wrapper; zero-gix end state | `.spec/adr/20261108083507-native-rust-git-engine-versus-wrapper-and-gix.adr.md` |
| Structural conventions (SRP / layout) | `.spec/sot/structural-conventions.sot.md` + rule `15-structural-conventions` |
| CLI surface | `.spec/sot/cli-surface.sot.md` |
| Core object model | `.spec/sot/core-object-model.sot.md` |
| Git formats / protocols | `.spec/sot/git-compat-surface.sot.md` + `.spec/research/*-git-*.research.md` |
| Remotes (generic client + GitHub-class verify) / Git server role | `.spec/sot/remote-host-compatibility-surface.sot.md` + M11 plan `.spec/plan/*-m11-*.plan.md` |
| Interop test matrix (G0–G37; G33 CRED / G34 shared-index / G35 depth / G36 Basic / G37 gpgsig SSH) | `.spec/sot/interop-test-matrix.sot.md` |
| Clean-room verification | `.spec/sot/product-verification-clean-room.sot.md` + skill `verify-clean-room` |
| Journey-compare (G16 func + timing; PF5 Claimed 8/8 honesty) | `.spec/sot/clean-room-journey-compare.sot.md` + `verify/matrices/matrix-journey.md` |
| Live private forge journeys (G28 opt-in Claimed) | residual register + `.spec/sot/g28-live-private-arks-journeys.sot.md` + `verify/matrices/matrix-g28.md` |
| Credential helper / askpass (G33) | `.spec/sot/credential-helper-askpass.sot.md` + `verify/matrices/matrix-g33.md` |
| Shared-index N-way (M26 / G34) | `.spec/sot/shared-index-nway-clean-room.sot.md` + `verify/matrices/matrix-g34.md` |
| CELIR dual-path / index-only opt-in | `.spec/sot/storage-celir-boundary.sot.md` + ADR `20261110052030` + `.spec/sot/celir-index-dual-path.sot.md` + skill `review-celir-boundary` |
| Lease / concurrency intent (incl. L3 Claimed shape; shared-index = M26) | `.spec/sot/lease-concurrency-intent.sot.md` + skill `review-lease` |
| Multi-agent Git concurrency scenarios (catalog; Deferred=0) | `.spec/sot/multi-agent-git-concurrency-scenarios.sot.md` + `.spec/research/*-multi-agent-git-concurrency*.research.md` |
| Agent governance / AGENTS role | `.spec/sot/agent-governance.sot.md` |
| Agent / Specfold readiness | `.spec/sot/agent-dev-readiness.sot.md` |
| Inherited conventions map | `.spec/sot/conventions-inherit.sot.md` |
| English-only language lock | `.spec/sot/language-english-only.sot.md` + rule `05-english-only` |
| Spec / design before code | skill `design-sot` → `.spec/sot/`, `.spec/plan/` |
| Implement accepted SoT slice | skill `implement-sot-slice` (bottom-up) + rule `70-bottom-up-build-order` |
| Review Git-compat claims | skill `review-git-compat` |
| Review CELIR / lease seams | skills `review-celir-boundary`, `review-lease` |
| Rust style | rule `10-rust-conventions` → cite humanymous (no fork) |
| Session overlap (light) | rule `90-session-overlap` |
| Public surface / human vs agent vocabulary | `.spec/sot/public-surface.sot.md` + rule `80-public-surface`; site `docs/` (`remote_theme: modootoday/jekyll-theme-modootoday`) |
| Role perspectives | `.agents/agents/` |

## Done means

| Scope | Gate |
|-------|------|
| Spec / docs slice | Primary sources cited (URL + scope); no invented wire/object/index semantics; `spec check` clean (or residual warnings documented); summarize + link — do not paste large copyrighted text |
| Implementation slice | Owning SoT accepted; bottom-up order (rule `70`); scoped Rust tests/fmt/clippy as applicable; no Git-semantic invention |
| Product “verified” | Docker clean-room receipt per `.spec/sot/product-verification-clean-room.sot.md` — host green alone is **not** done |

## Skills & agents

Canon: `.agents/skills/`, `.agents/rules/`, `.agents/agents/` (registered in `spec.toml` `[[assets]]` when present).
Ops entry: skill `specfold-ops`.

## Ambiguity

If requirements conflict, SoT coverage is missing, or Git vs CELIR/lease scope is unclear: **stop and ask** before large edits (rule `40-ambiguity-ask`).
