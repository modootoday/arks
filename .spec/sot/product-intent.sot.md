---
id: sot:root/product-intent
title: Product intent
status: active
created: 2026-11-08T08:21:10Z
updated: 2026-11-11T00:25:00Z
tags: [product, intent, bottom-up, core, engine, concurrency, cli-parity, alias, dot-git, server, lease]
verify: manual
rel:
  depends_on: [sot:root/product-naming, sot:root/git-compat-surface, sot:root/cli-surface, sot:root/remote-host-compatibility-surface, sot:root/product-verification-clean-room, sot:root/program-roadmap, adr:root/20261108083507]
  evidence: [research:root/20261108083502, research:root/20261108083640, research:root/20261108183800]
---

# Product intent

Version **0.4.2**. Normative **product goals, non-goals, and success criteria** for **arks** (`arks for Git`; working expansion **Agent Revision Kinetic System** per [[sot:root/product-naming]]). Naming lock: [[sot:root/product-naming]]. Compatibility surfaces: [[sot:root/git-compat-surface]] (formats/protocols / **same `.git` layout**), [[sot:root/cli-surface]] (**Git CLI symmetric compatibility** + **`git`→`arks` alias drop-in**), [[sot:root/remote-host-compatibility-surface]] (**generic remote client** + **Git server role**). Verification authority: [[sot:root/product-verification-clean-room]]. Production exits: [[sot:root/program-roadmap]] (**M9** CLI-parity client; **M11** production-server). Native-engine architecture (hot-path in-house; zero-`gix` end state): [[adr:root/20261108083507]]. Multi-agent concurrency / allocator intent research: [[research:root/20261108083502]]. CLI-parity research: [[research:root/20261108183800]].

This SoT owns **inner/core product intent** and the **bottom-up build principle**. Command/flag inventory, refuse policy, and alias-drop-in requirements are owned by [[sot:root/cli-surface]]; on-disk `.git` sameness is owned by [[sot:root/git-compat-surface]]; remote client/server roles by [[sot:root/remote-host-compatibility-surface]]. This document locks same-`.git`, CLI/alias parity, **generic remotes**, and **Git server** as **first-class compatibility goals** (server under the separate **M11** bar).

## Rule

### Mission

1. Build **arks**: a **native Rust Git engine** — its own in-process implementation of Git formats, protocols, documented semantics, **a Git CLI–symmetric `arks` binary**, **and a Git server role** (upload-pack / receive-pack for claimed transports) — plus agent-scale concurrency, lease coordination, and extreme memory/throughput performance.
2. **Standards compatibility first; performance second** for any claimed Git-compat surface. Do not invent Git object, index, pack, ref, wire, or CLI option semantics. Cite Git project / git-scm primary docs via [[sot:root/git-compat-surface]], [[sot:root/cli-surface]], and the catalog research they depend on.
3. **Compatibility means interoperability with stock Git at store/wire and CLI layers**, not “shell out to `git`.” Stock Git is a **peer oracle** for clean-room round-trips and fixtures ([[sot:root/product-verification-clean-room]]), **not** the runtime implementation of product paths.
4. **Same `.git` (on-disk).** In Git-compat mode, arks reads and writes the **same repository directory layout** stock Git uses (typically `.git/` with objects, refs, index, config, hooks, … as claimed) — **not** a parallel proprietary store for day-to-day Git-compat workflows ([[sot:root/git-compat-surface]]). A repo created or mutated by arks MUST remain usable by stock Git for the claimed subset, and vice versa.
5. **Git CLI symmetric compatibility + alias drop-in** are part of the **single production bar** ([[sot:root/program-roadmap]], [[sot:root/cli-surface]]): complete argv/flag parity for the pinned inventory (minus documented exclusions), sufficient for users who previously used `git` to substitute via shell alias (e.g. `alias git=arks` or documented equivalent) and keep muscle-memory workflows. Delivered in phases; **not deferred as end-state**.
6. **Product leases are default product specification** (baseline agent coordination — not a late optional marketing add-on). **High-concurrency / N-way multi-commit** is an **optional selectable** mode — not the only path; classic serial Git-lock Git-compat remains available. Ship both per [[sot:root/lease-concurrency-intent]] **without** breaking stock-Git interchange, same-`.git` usability, claimed CLI parity, or alias-drop-in — and, when high-concurrency mode is enabled, with an explicit export/import path back to stock-readable repos. Alternate **object stores** (CELIR as ObjectStore) remain **out of scope / Refuse**; optional CELIR **indexing-service** dual-path is behind feature `celir-index` (default OFF) ([[sot:root/storage-celir-boundary]], [[adr:root/20261110052030]]).

### Goals

| Goal | Normative meaning |
|------|-------------------|
| Native Rust Git engine | Core ops (hash, object/ref/index/pack parse & write, repo layout, claimed protocol client/server pieces) run **in-engine** in Rust crates ([[sot:root/crate-architecture]], [[adr:root/20261108083507]]) |
| Same `.git` / Git-compatible core | Claimed object/ref/index/pack/protocol/layout subsets honor [[sot:root/git-compat-surface]]; Git-compat mode uses stock Git’s on-disk layout (no parallel proprietary repo for claimed work) |
| Git CLI symmetric compatibility | End-state `arks` CLI matches pinned stock `git` command/flag inventory per [[sot:root/cli-surface]]; production requires inventory closure ([[sot:root/program-roadmap]] M9) |
| Alias drop-in (`git`→`arks`) | Users who used `git` can alias the binary to `arks` and run muscle-memory workflows; argv/behavior/exit-code parity sufficient for substitution ([[sot:root/cli-surface]]) |
| Product leases (default) | Agent-oriented lease coordination is **default product capability/spec** ([[sot:root/lease-concurrency-intent]]); lease **runtime** ships on the M10 ladder after M9, but SoT treats leases as baseline — not “optional forever because after production” |
| High-concurrency multi-commit (opt-in) | **Selectable** path targeting **N ≈ hundreds** concurrent writers without global lock serialization killing throughput ([[sot:root/lease-concurrency-intent]], [[research:root/20261108083502]]); users MAY stay on classic serial Git-lock Git-compat |
| Lock-contention relief | Default leases + optional commit-pipeline design address Git’s short exclusive locks as the primary multi-agent pain — **without** skipping documented Git locks on Git-compat interchange paths (**lease ≠ skip Git locks**) |
| Extreme allocator / memory policy | Memory allocation policy (arena/pool/zero-copy **principles**) is a **first-class** performance goal for highly concurrent engine paths ([[research:root/20261108083502]]) — not an afterthought |
| Bottom-up delivery | Implement smallest inner modules first; outer layers consume stable inner APIs (see Build principle) |
| Object store | Filesystem `.git` / Git-compat store via `arks-store` is the **sole** product object-store path. CELIR as ObjectStore remains **Refuse**; optional index-only feature `celir-index` (default OFF) MAY path-dep `celir-index` / `celir-index-runtime` ([[sot:root/storage-celir-boundary]] v0.8.0, [[adr:root/20261110052030]]; [[evidence:root/20261108192100]], [[research:root/20261108083640]]). Multi-writer coordination on the FS store: `arks-lease` |
| Generic Git remotes (client) | Clone/fetch/push to **any** remote speaking claimed smart HTTP/SSH (etc.) protocols; **GitHub-class hosts are the representative clean-room verify peer**, not the exclusive remote type ([[sot:root/remote-host-compatibility-surface]]) — required before CLI-parity production (**M6→M9**) |
| Git server role | arks hosts repositories so stock Git (and arks-as-client) can clone/fetch/push for the claimed upload-pack / receive-pack + transport subset ([[sot:root/remote-host-compatibility-surface]]) — **production-server** exit at **M11** (not required for **M9**) |
| Honest verification | “Verified” means Docker clean-room per [[sot:root/product-verification-clean-room]] |

### Non-goals (product-wide, this SoT)

1. **Thin CLI / process wrapper around the `git` executable** for product runtime paths (hash-object, update-ref, commit, pack, fetch, etc.). Invoking stock `git` is allowed only as clean-room oracle, fixture generator, or explicit developer tooling — never as the architecture for core engine ops ([[adr:root/20261108083507]]).
2. Replacing Git’s documented formats with private encodings in Git-compat mode, or requiring a non-`.git` proprietary layout for claimed Git-compat workflows.
3. Implementing ObjectStore via CELIR / inventing CELIR APIs / treating CELIR as a silent Git object store; enabling `celir-index` by default ([[sot:root/storage-celir-boundary]] v0.8.0, [[adr:root/20261110052030]]).
4. **Pretending a CLI subset is production.** Phased delivery waves are allowed; calling Wave L/H “production” or deferring full porcelain as permanent end-state is **not**. Inventory, refuse policy, and exclusions: [[sot:root/cli-surface]].
5. Host-only “verified” claims.
6. Forking humanymous Specfold/convention prose — cite `C:\workspaces\humanymous` (especially structural bottom-up patterns in `sot:root/structural-development` there); do not duplicate competing convention SoTs here.
7. Embedding full **gitk/git-gui/citool** pixel GUIs inside the arks CLI binary, treating Git LFS **server product** as a default production gate without a dedicated SoT, or forge product APIs (PRs/Checks/Apps / GitHub App **as** arks server) — see [[research:root/20261110212414]] FS6/FS7 for **closest parity** slices (filter-process / external-tool handoff). Casual “agent non-interactive N/A” for real git interactive flows is **forbidden** under the full-symmetry DoD.
8. Any permanent Cargo dependency on gitoxide/`gix` (runtime, optional feature, or permanent `dev-dependencies`) — **rejected** as end state. Hot paths always in-house; prefer never introducing `gix`; transitional cold use only under [[adr:root/20261108083507]] with mandatory exit before M4 (gix-exit).
9. Treating **M9** CLI-parity production as if it included the Git **server** role — server is **M11** / production-server ([[sot:root/program-roadmap]]).

### Success criteria

A core slice is **successful** when all of the following hold for the claimed subset:

1. Semantics cite primary sources through [[sot:root/git-compat-surface]] / [[sot:root/cli-surface]] as applicable (no invented bytes or flags).
2. Inner crate graph obeys [[sot:root/crate-architecture]] (core never depends on CLI/remote; core never requires the `git` binary for product ops).
3. Object/ref identity matches [[sot:root/core-object-model]] and is computed in-engine.
4. Authoritative interop/acceptance for that slice is green in clean-room ([[sot:root/product-verification-clean-room]]), with stock Git as peer oracle — not host-only.
5. Storage and lease work (when claimed) stay inside their boundary/phasing SoTs.
6. Concurrency / allocator claims (when made) cite [[sot:root/lease-concurrency-intent]] / [[research:root/20261108083502]] and do not regress Git-compat interchange or claimed CLI parity for the claimed mode.
7. **CLI-parity production (**M9**)** additionally requires CLI-parity inventory closure, **same-`.git` interchange**, **alias-drop-in readiness**, and **remote client** gates per [[sot:root/program-roadmap]] / [[sot:root/cli-surface]] (not merely format interop + CLI subset).
8. **Production-server (**M11**)** additionally requires the claimed Git server subset (upload-pack / receive-pack + transports) green under clean-room **G15** per [[sot:root/remote-host-compatibility-surface]] / [[sot:root/program-roadmap]] — without inventing forge UX.

### Build principle (normative)

**Bottom-up, inner → outer** is a required delivery order, not a suggestion. Affinity: humanymous structural-development bottom-up acceptance-before-code and layer contracts (`C:\workspaces\humanymous\.spec\sot\structural-development.sot.md`) — cite that pattern; do not fork its full prose into this repo.

1. **Define acceptance before code** for each slice (fixture + clean-room pointer).
2. **Implement the smallest dependency-free module first**, then compose outward.
3. **Inner crates MUST NOT depend on outer crates** (CLI, porcelain, remote client/server, host APIs).
4. **Do not skip layers** to “demo” porcelain against unfinished object/ref stores.
5. **Phased claims:** advertise only what the claimed subset implements; refuse unknown extensions per git-compat.
6. Executable ordering for the first modules lives in [[plan:root/20261108082110]]. Program milestones (**M0–M11**) and Wave confirmation: [[sot:root/program-roadmap]], [[plan:root/20261108085816]].

### Ownership boundary

| Zone | Owner |
|------|-------|
| Product intent, crate graph, object model, CELIR boundary, lease intent, core build plan | **This ownership zone (inner/core)** |
| Git formats/protocols compatibility surface | [[sot:root/git-compat-surface]] (do not rewrite wholesale) |
| Git CLI symmetric compatibility / command inventory | [[sot:root/cli-surface]] |
| Native engine vs wrapper; zero-`gix` end state | [[adr:root/20261108083507]] |
| Multi-agent concurrency / allocator research | [[research:root/20261108083502]] + [[sot:root/lease-concurrency-intent]] |
| CELIR dual-path (index-only opt-in) + research archive | [[sot:root/storage-celir-boundary]], [[adr:root/20261110052030]] (+ historical [[research:root/20261108083640]]) |
| Remote client + Git server role | [[sot:root/remote-host-compatibility-surface]] |
| Clean-room verification | [[sot:root/product-verification-clean-room]] |
| Production / milestone ladder | [[sot:root/program-roadmap]] |
| `AGENTS.md`, `.agents/**` | Sibling ownership — do not edit from this zone |

## Why

arks exists because stock Git serializes heavily under many concurrent writers (classic agent swarm pain) and because a subprocess wrapper cannot own allocator policy or in-process concurrency. A native Rust engine that stays byte/protocol-faithful to Git, a CLI that reaches Git argv symmetry, **and** a standards-faithful Git **server** role gives agents and humans one interchangeable tool while **default product leases** and an **optional** high-concurrency mode (plus memory policy) evolve. Bottom-up core-first prevents porcelain and remotes from encoding unstable or invented semantics; phased CLI waves must not redefine CLI-parity production as a permanent subset; server must not silently inflate the M9 bar; and “leases = only M10 marketing fluff” must not contradict the lease default-spec SoT.

## Verification

Manual until harness exists.

1. `spec check` clean for this SoT and dependents.
2. Implementation PRs that claim a product goal cite this SoT plus the slice’s clean-room gate under [[sot:root/product-verification-clean-room]].
3. Any PR that introduces an outer→inner dependency cycle fails [[sot:root/crate-architecture]] review.
4. Any PR that implements core Git ops by shelling out to `git` fails this SoT and [[adr:root/20261108083507]].
5. Do not mark product goals “verified” from host-only runs.
