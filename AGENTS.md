# AGENTS.md — arks

Cross-tool always-on rules. Keep this file **short** — details load on demand.
This file is a **router**, not a second Source of Truth. Normative product intent lives in `.spec/sot/`.

## Mission

Build **arks** (`arks for Git`; working expansion **Agent Revision Kinetic System** per naming SoT) — a Git-compatible replacement CLI and infra for multi-session revision contention.
**Standards compatibility first; performance second.** Do not invent Git semantics without a cited public Git project / git-scm primary document. Product/crate/bin name lock: `.spec/sot/product-naming.sot.md`.

**Single release = `arks` only.** Peers: `git`→`arks`; `git-daemon`→`arks daemon` (internal `arks-daemon`); `gh`→`arks hub …` (internal `arks-hub`); agent bots→`arks agent …` (internal `arks-agent`). Never spawn `git`/`gh` on product paths. Spec index: `.spec/README.md`.

## Hard constraints

1. **English only / Hangul ban** — all authored project text (Specfold docs, AGENTS, `.agents/**`, `docs/`, comments, commit/PR prose) is English. Proper nouns (`arks`, `CELIR`, `humanymous`) stay. Korean/Hangul (and other languages) may appear in chat/coordinator briefings only — never in committed Specfold or `docs/`. SoT: `.spec/sot/language-english-only.sot.md`; rule `05-english-only`.
2. **Standards-first** — object/pack/ref/index/protocol behavior from primary docs only (`git-compat-surface` + research catalog).
3. **Clean-room authoritative** — product “verified” claims require Docker clean-room; host runs are advisory (`.spec/sot/product-verification-clean-room.sot.md`). **No invent Claimed/PASS** — rule `28-evidence-honesty`.
4. **No literal competing SoT** — do not restate or fork humanymous convention ledgers in this repo; cite live paths under `C:\workspaces\humanymous`.
5. **CELIR / lease boundary** — filesystem `.git` is the sole Git-compat **object store**; CELIR index feature is ops/CI-only (default OFF) — not product Claimed path. Lease/concurrency has its own SoT. Never silently replace Git object-store semantics in Git-compat mode (`.spec/sot/storage-celir-boundary.sot.md`, `.spec/adr/20261110052030-celir-index-optional-product-path-dep.adr.md`).
6. **Public surface** — repo will be public; no secrets in tracked paths; **human** (`docs/`, README, LICENSE, SECURITY) vs **agent** (`AGENTS.md`, `.agents/`, `.spec/`, `verify/` matrices) vocabulary split; thin Pages vs Specfold brain; English-only public copy; no “faster than Git” marketing outside receipts. SoT: `.spec/sot/public-surface.sot.md`; rules `80-public-surface`, `25-spec-classification`. `.spec.local/` stays gitignored.
7. **Structural conventions** — SRP, modularization, no flat dumps, naming compliance, context-based dirs, anti-fragmentation, and language-standard layouts (Rust Cargo workspace / crates members first; Specfold, docs, and agents are overlays). SoT: `.spec/sot/structural-conventions.sot.md`; rule `15-structural-conventions`.
8. **No folklore** — prefer updating `.spec/` over chat-only process invention. **No push** while remote is empty / push-deferred unless the owner explicitly authorizes (rule `91-commit-hygiene`).
9. **Commit order** — Specfold SoT meaning changes land **first**; implementation later, one concern per commit — rules `27-spec-first-commit-order`, `91-commit-hygiene`. SoT body `Version` ≠ product/crate release — rule `26-sot-version-vs-release` (details: `.spec/README.md`).
10. **CLI surface** — single bin `arks`; personalities `git|gh|arks`; never spawn stock `git`/`gh` on product paths; **`arks serve` ≠ `arks hub`**; ports normative≠demo — rules `30`, `32`, `33`, `34`; skills `guard-cli-surface`, `review-product-subprocess`, `self-host-serve`.
11. **Honesty wording** — stubs/Partial ≠ Claimed/Verified/PASS — rules `28`, `29`; hooks scan commit messages.
12. **Hooks** — `.cursor/hooks.json` denies force-push to main/master and staging `.spec.local`/secrets; asks on push / blind add / destructive git / `gh`; Hangul guards on edit + commit (rule `05`).

## Progressive disclosure (read more only when needed)

Convention canon for Specfold / agent layout: **cite** humanymous live trees
(`C:\workspaces\humanymous\AGENTS.md`, `C:\workspaces\humanymous\.agents\`,
`C:\workspaces\humanymous\.spec\sot\structural-development.sot.md`) — do not copy mega ledgers.

Before changing governed code or specs: `spec context <path>` → edit → `spec check`.

Current program honesty (pointers only — owning SoTs win): **M0–M26**; gates **G0–G38** (**G33** CRED, **G34** shared-index, **G35** depth, **G36** Basic, **G37** gpgsig SSH, **G38** OpenPGP·X.509·tag-s). **M25** server transports **G29–G32 Claimed**; **M26 / G34 Claimed+Verified**; **G28** live-private journeys **Claimed** + offline **J-CLONE-LOCAL/PATH** PASS (checklist N/A=0; land tip ~`902d728`); L3 **Claimed** shape (worktree/ref-shard). Tip operator honesty **PARTIAL** after **FS_final YES** ([[sot:root/program-roadmap]] tip table; `docs/completion.md`). W1–W5 residual honesty: **CRED G33**, hooks honor list, **GPGSIG** SSH (**G37**) + OpenPGP/X.509/`tag -s` (**G38** / R4), **G9 `--depth`/G35** + **R3 `blob:none`**, **AUTH Basic G36** (OAuth Refuse), **PF5 tip Claimed(partial)** (M17 19/19; ALIAS `aggressive_na` **0.5489** + REVERT Claimed [[evidence:root/20261111025200]]; historical **8/8** at `79fc9bf` archival — **no G19 thrash**), **FW-G8-DEFAULT** / **FW-GH-LIVE-SCALE** harness Claimed (live latency Blocked without secrets). Symmetry ladder **S0–S_final Done** ([[research:root/20261110105436]]; S6 g38/g39/g40); **FS0–FS_final Done**; **L9** closed. Catalog **Deferred=0**; CELIR **index-only** dual-path (**M24**, ObjectStore Refuse; **G22–G25** Verified). Do not invent live PASS and do not clash sole-runner Docker with a sibling live-GH agent.

| If you are working on… | Read next |
|------------------------|-----------|
| Product / crate / bin name | `.spec/sot/product-naming.sot.md` |
| Product intent | `.spec/sot/product-intent.sot.md` |
| Program roadmap / production exits (M9 CLI-parity; M11 server; M16 active alias; M17 journeys; M21 Deferred=0; M23 L3 Claimed; M24 CELIR index dual-path; M25 server transports; M26 shared-index) / milestones | `.spec/sot/program-roadmap.sot.md` + `.spec/plan/*-program-milestones*.plan.md` + `.spec/plan/*-m[0-9]-*.plan.md` + `.spec/plan/*-m1[0-7]-*.plan.md` + `.spec/plan/*-m2[0-6]-*.plan.md` |
| Post-bar residual disposition (no bare development Opens; Deferred=0; §F W1–W5 Claimed honesty) | `.spec/sot/production-residual-register.sot.md` |
| Product-completion ladder (post-PF5 8/8; L0–L9 historical Done; L9 closed) | `.spec/research/20261110095142-product-completion-goal-ladder.research.md` |
| Git symmetry expanded DoD ladder (S0–S_final; Done) | `.spec/research/20261110105436-git-symmetry-expanded-dod-goal-ladder-s0-s-final.research.md` |
| Optional residuals O-ladder (O1–O_final; historical Done) | `.spec/research/20261110205403-optional-residuals-o-ladder-o1-o-final.research.md` |
| Full Symmetry ladder (FS0–FS_final; **active git feature DoD**) | `.spec/research/20261110212414-full-symmetry-ladder-fs0-fs-final.research.md` |
| Tip PARTIAL completion / per-gap unblock | `.spec/research/20261111001816-partial-completion-unblock-roadmap.research.md` + `.spec/research/20261110225000-fs-final-completion-currency.research.md` + `docs/completion.md` |
| Gap classification + **G-ladder** (G1…G_final Best ROI) | `.spec/research/20261111022345-gap-classification-self-dep-ffi.research.md` + `.spec/research/20261111012840-still-blocked-external-deps-unblock.research.md` + `.spec/plan/20261111024440-gap-classification-g-ladder.plan.md` + `.spec/plan/20261111024441-g-ladder-goal-directive.plan.md` + roadmap tip table |
| Git CLI symmetric compatibility | `.spec/sot/cli-surface.sot.md` |
| Crate architecture | `.spec/sot/crate-architecture.sot.md` |
| Native engine vs wrapper; zero-gix end state | `.spec/adr/20261108083507-native-rust-git-engine-versus-wrapper-and-gix.adr.md` |
| Structural conventions (SRP / layout) | `.spec/sot/structural-conventions.sot.md` + rule `15-structural-conventions` |
| CLI surface | `.spec/sot/cli-surface.sot.md` |
| Core object model | `.spec/sot/core-object-model.sot.md` |
| Git formats / protocols | `.spec/sot/git-compat-surface.sot.md` + `.spec/research/*-git-*.research.md` |
| Remotes (generic client + GitHub-class verify) / Git server role | `.spec/sot/remote-host-compatibility-surface.sot.md` + M11 plan `.spec/plan/*-m11-*.plan.md` |
| Interop test matrix (G0–G38; G33 CRED / G34 shared-index / G35 depth / G36 Basic / G37 gpgsig SSH / G38 OpenPGP·X.509·tag-s) | `.spec/sot/interop-test-matrix.sot.md` |
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
| English-only / Hangul ban | `.spec/sot/language-english-only.sot.md` + rule `05-english-only` |
| SoT Version ≠ product release | `.spec/README.md` + rule `26-sot-version-vs-release` |
| Spec-first commit order | `.spec/README.md` + rule `27-spec-first-commit-order` + skill `commit-governance` |
| Evidence honesty (no invent Claimed) | rule `28-evidence-honesty` + residual register |
| Spec / design before code | skill `design-sot` → `.spec/sot/`, `.spec/plan/` |
| Implement accepted SoT slice | skill `implement-sot-slice` (bottom-up) + rule `70-bottom-up-build-order` |
| CLI personality / serve≠hub / no git-gh spawn | rules `30`, `32`, `33` + skill `guard-cli-surface` + `.spec/sot/arks-cli-personality-factory.sot.md` |
| Review Git-compat claims | skill `review-git-compat` |
| Review CELIR / lease seams | skills `review-celir-boundary`, `review-lease` |
| Rust style | rule `10-rust-conventions` → cite humanymous (no fork) |
| Session overlap + commit hygiene | rules `90-session-overlap`, `91-commit-hygiene` |
| Public surface / human vs agent vocabulary | `.spec/sot/public-surface.sot.md` + rule `80-public-surface`; site `docs/` (`remote_theme: modootoday/jekyll-theme-modootoday`) |
| Spec classification (`.spec` vs `.spec.local`) | `.spec/README.md` + rules `25-spec-classification`, `35-spec-local-journey` + `.spec/sot/agent-governance.sot.md` |
| Stub vs Claimed wording | rule `29-stub-vs-claimed` |
| Ports normative vs demo | rule `34-ports-normative-vs-demo` + `.spec/sot/arks-self-host-server.sot.md` + research `20261111055609` |
| Cross-OS / pin honesty | rule `61-cross-os-pin-honesty` |
| Symmetry matrices / release gate | skills `run-symmetry-matrix`, `release-symmetry-gate` + `.spec/sot/verify-cleanroom-symmetry-matrix.sot.md` + `.spec/sot/release-symmetry-precheck.sot.md` |
| Self-host serve lane | skill `self-host-serve` + rules `33`, `34` |
| Research → SoT pipeline | skill `research-to-sot` |
| Product git/gh spawn review | skill `review-product-subprocess` + rule `30` |
| Governance gap matrix | `.spec/research/20261111061037-agent-governance-gap-matrix.research.md` |
| `arks hub` / `arks daemon` / `arks agent` / self-host | `.spec/sot/arks-hub-product-intent.sot.md` · `.spec/sot/arks-daemon-product-intent.sot.md` · `.spec/sot/arks-agent-product-intent.sot.md` · `.spec/sot/arks-self-host-server.sot.md` |
| Role perspectives | `.agents/agents/` |
| Cursor hooks (enforceable guards) | `.cursor/hooks.json` + `.cursor/hooks/*.ps1` |

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
