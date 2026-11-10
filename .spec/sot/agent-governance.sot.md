---
id: sot:root/agent-governance
title: Agent governance
status: active
created: 2026-11-08T08:20:00Z
updated: 2026-11-09T21:40:00Z
tags: [agents, governance, specfold, conventions, language, public, m24, g28, celir-index, deferred-zero, l3]
verify: manual
rel:
  depends_on:
    - sot:root/conventions-inherit
    - sot:root/language-english-only
    - sot:root/product-naming
    - sot:root/git-compat-surface
    - sot:root/product-verification-clean-room
    - sot:root/structural-conventions
    - sot:root/public-surface
    - sot:root/program-roadmap
    - sot:root/production-residual-register
    - sot:root/storage-celir-boundary
    - sot:root/celir-index-dual-path
    - sot:root/interop-test-matrix
    - sot:root/agent-dev-readiness
  governs:
    - AGENTS.md
    - .agents/
---

# Agent governance

Version **0.2.1**. Normative intent for how AI agents use **AGENTS.md**, `.agents/**`, and Specfold in this repository. Product semantics remain owned by domain SoTs; this SoT owns **process routing**.

## Rule

### AGENTS.md is a router, not a second SoT

1. Root `AGENTS.md` MUST stay short: mission, hard constraints, progressive disclosure table, Done means, ambiguity.
2. Normative product/compat/verification intent MUST live in `.spec/sot/` (and cited research). Agents MUST NOT treat AGENTS prose as a competing product SoT.
3. When AGENTS and a SoT disagree, the **owning SoT wins**; fix the router link rather than expanding AGENTS into a ledger.

### Required trees

4. Agent governance for arks REQUIRES:
   - `AGENTS.md` (instructions asset)
   - `.agents/rules/` (always-on numbered rules)
   - `.agents/skills/` (SKILL.md with “Use when…” descriptions)
   - Optional `.agents/agents/` role perspectives
5. These trees are part of Specfold design canon for this repo (see [[sot:root/conventions-inherit]]). Register them under `spec.toml` `[[assets]]` so `spec assets` / rules A1–A2 apply.

### Hard constraints agents must honor

6. **English only** — [[sot:root/language-english-only]]; rule `.agents/rules/05-english-only.md`.
7. **Standards-first** — [[sot:root/git-compat-surface]].
8. **Clean-room authoritative** for product “verified” — [[sot:root/product-verification-clean-room]].
9. **No literal competing convention SoT** — cite humanymous live paths; pointer map in [[sot:root/conventions-inherit]].
10. **CELIR / lease boundary** — filesystem `.git` is the sole Git-compat **object store**. CELIR MAY appear only as optional index-only feature `celir-index` (default OFF); dual-path companion is **M24** ([[sot:root/storage-celir-boundary]], [[sot:root/celir-index-dual-path]], [[adr:root/20261110052030]]). **ObjectStore via CELIR remains Refuse.** Lease/concurrency has its own SoT (rule `.agents/rules/50-celir-boundary.md`). Never silently replace Git object-store semantics in Git-compat mode.
11. **Bottom-up build order** — inner modules first (rule `.agents/rules/70-bottom-up-build-order.md`; skill `implement-sot-slice`). Program ladder **M0–M24** lives in [[sot:root/program-roadmap]].
12. **Structural conventions** — SRP, modularization, no flat listing, naming compliance, context-based directories, anti-fragmentation, and language-standard layouts (Rust Cargo workspace / `crates/` first; Git/Specfold/`docs`/`.agents` are orthogonal overlays) ([[sot:root/structural-conventions]]; rule `.agents/rules/15-structural-conventions.md`).
13. **Public surface** — no secrets in tracked paths; thin Pages vs Specfold brain; **human** vs **agent** vocabulary split ([[sot:root/public-surface]]; rule `.agents/rules/80-public-surface.md`).

### Current program honesty (router pointers — not a second ledger)

14. Agents MUST progressive-disclose live truth from owning SoTs rather than chat memory:
    - **Milestones:** **M0–M24** — [[sot:root/program-roadmap]] (CLI-parity production historical = **M9**; active alias = **M16**; full journeys = **M17**; L3 claimed shape = **M23**; CELIR index dual-path = **M24**).
    - **Gates:** **G0–G28** — [[sot:root/interop-test-matrix]] (**G17–G21** issue/CLI/perf; **G22–G25** CELIR index Verified; **G26–G27** local/smart-HTTP PR sims Claimed; **G28** live-private journeys Claimed + offline clone PASS, N/A=0; land tip ~`902d728`).
    - **L3:** Claimed shape = lease + batch-apply + worktree/ref-shard ([[adr:root/20261109194200]]). Shared-index N-way = **M26** / **G34** ([[adr:root/20261110074000]]); ≠ M24/G22/G24.
    - **Catalog:** **Deferred=0** success exit for M21 — Closed-deferred is not a lasting work-item disposition ([[sot:root/production-residual-register]]).
    - **Live journey honesty:** **G28** is Claimed ([[evidence:root/20261109212500]] + [[evidence:root/20261109213400]]); re-runs stay opt-in — default Refuse without secrets; do not invent PASS; do not clash sole-runner Docker with a sibling live-GH agent.
15. Prefer updating owning `.spec/` documents when process or bar truth changes — **no folklore** in chat-only process invention.

### Specfold loop

16. Before governed edits: `spec context <path>`. Prefer updating `.spec/` over chat folklore. Before docs/spec done: `spec check`.

### Ownership

17. This SoT does **not** own product-intent, crate-architecture, structural-conventions, cli-surface, core-object-model, interop-test-matrix content, storage-celir-boundary detail, celir-index-dual-path detail, lease-concurrency-intent, public-surface placement detail, program-roadmap exits, residual-register rows, or agent-dev-readiness checklists. Those sibling SoTs own their domains; AGENTS may only progressive-disclose them.
18. Do not edit sibling domain SoTs from agent-governance work except to add reciprocal cross-links when those owners request them.

## Why

Agents otherwise invent process, duplicate humanymous ledgers, or drift from current M0–M24 / G17–G28 / CELIR-index / L3 / Deferred=0 truth. A thin router plus registered rules/skills keeps multi-tool sessions aligned with Specfold without a second source of product truth.

## Verification

Manual:

1. `spec check` clean for this document and related assets.
2. `spec assets` lists instructions, rules, skills, and agents roots.
3. AGENTS.md contains no normative Git wire/object semantics that are absent from owning SoTs.
4. Rules/skills named in AGENTS exist on disk.
5. Progressive-disclosure pointers name M0–M24, G0–G28 (G28 Claimed), CELIR index-only (not ObjectStore), L3 Claimed shape, Deferred=0, and live-journey honesty without dumping gate catalogs into AGENTS.
